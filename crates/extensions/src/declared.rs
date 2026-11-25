//! The actions a package declares.
//!
//! A blueprint that sets up somebody's product has to run that product's
//! program: ask it something, tell it something. It could do that with
//! `shell`, which is a script, written for one system's shell and allowed
//! to run anything. So a package says instead, under `actions`, each way
//! its program is run: with which arguments, how what it prints is read,
//! when that is a failure and in what words, and what is given out. A
//! blueprint then does `<package>.<name>`, and the application runs the
//! program itself ([`crate::performed`]), with no shell in between.
//!
//! This is the half that knows what a package said: it reads `actions` out
//! of the manifest, checked like everything else there, and works out each
//! part of one doing of an action from what the step gave it. It starts
//! nothing.

use std::collections::BTreeMap;
use std::sync::Arc;

use blueprint::expr::{self, Expr};
use blueprint::template::{self, Template};
use blueprint::{ActionSpec, Cond, ValueType};
use kernel::Fields;
use serde_json::{Map, Value};

use crate::extension::{Args, Check, holds};

/// What an expression about the daemon can read: `daemon.url` and so on.
pub const DAEMON: &[(&str, &str)] = &[
    ("url", "where the daemon itself answers, as http://127.0.0.1:7700"),
    ("user", "who the daemon runs as on its device; empty where that is not known"),
];

/// What an expression can read once the program has run, besides what it
/// could read before.
const AFTER: &[&str] = &["answer", "code", "stdout", "stderr"];

const FIELDS: &[&str] =
    &["description", "makes", "with", "first", "refuse", "args", "answer", "fails", "out", "repeatable"];

/// A condition, and the sentence that is the failure when it holds.
struct Verdict {
    /// Where it is in the manifest, for when it cannot be worked out.
    at: String,
    when: Cond,
    says: Template,
}

/// One action of a package, checked and ready to be done.
pub struct Declared {
    /// How a blueprint sees it: `<package>.<name>`, what it takes and gives
    /// out, and `extensions:<package>` as what using it needs.
    pub spec: ActionSpec,
    /// What is filled in for an argument the step left out.
    defaults: Vec<(String, Value)>,
    /// Another action of the package, done before this one.
    first: Option<String>,
    refuse: Vec<Verdict>,
    args: Vec<Args>,
    /// Whether what it prints is read as one JSON value, or kept as text.
    json: bool,
    fails: Vec<Verdict>,
    out: Vec<(String, Value)>,
}

/// An expression somewhere in an action, kept until every action has been
/// read: what it may read depends on the others.
struct Read {
    at: String,
    expr: Expr,
    /// Whether the program has run by the time it is worked out.
    after: bool,
}

/// Whether `name` can be read after a dot in an expression.
fn is_word(name: &str) -> bool {
    name.starts_with(|first: char| first.is_ascii_lowercase())
        && name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

/// Every expression in the `${ }` holes of `value`.
fn exprs_in(value: &Value) -> Vec<Expr> {
    let mut found = Vec::new();
    template::each_template(value, &mut |text| {
        if let Ok(template) = Template::parse(text) {
            found.extend(template.exprs().cloned());
        }
    });
    found
}

/// Reads what a manifest says under `actions`.
pub fn read(check: &mut Check, package: &str, value: Option<&Value>) -> BTreeMap<String, Arc<Declared>> {
    let entries = match value {
        None | Some(Value::Null) => return BTreeMap::new(),
        Some(Value::Object(entries)) => entries,
        Some(_) => {
            check.problem("actions", "must be a mapping from an action's name to what it does");
            return BTreeMap::new();
        }
    };
    if entries.is_empty() {
        check.problem("actions", "is empty: leave it out if the package brings none");
    }
    let mut drafts: BTreeMap<String, (Declared, Vec<Read>)> = BTreeMap::new();
    for (name, entry) in entries {
        let at = format!("actions.{name}");
        if name.is_empty() || *name != expr::slug(name) {
            check.problem(&at, "must be named in lowercase letters, digits and dashes");
            continue;
        }
        if let Some(draft) = one(check, package, name, &at, entry) {
            drafts.insert(name.clone(), draft);
        }
    }
    // What each may read, now that what the others take and give is known.
    for (name, (declared, reads)) in &drafts {
        let at = format!("actions.{name}");
        let first = declared.first.as_ref().and_then(|first| {
            let found = drafts.get(first).map(|(first, _)| first);
            match found {
                _ if first == name => check.problem(&format!("{at}.first"), "is this action itself"),
                None => {
                    let known: Vec<&str> = drafts.keys().map(String::as_str).collect();
                    check.problem(
                        &format!("{at}.first"),
                        format!(
                            "names `{first}`, which is not an action of this package; it has: {}",
                            known.join(", ")
                        ),
                    );
                }
                Some(found) if found.first.is_some() => check.problem(
                    &format!("{at}.first"),
                    format!("names `{first}`, which has a `first` of its own; only one action is done before another"),
                ),
                Some(found) => {
                    for needed in found.spec.args.iter().filter(|arg| arg.required) {
                        if !declared.spec.args.iter().any(|arg| arg.name == needed.name) {
                            check.problem(
                                &format!("{at}.first"),
                                format!(
                                    "names `{first}`, which needs `{}`: `{first}` is given the arguments of \
                                     this action that it takes too, so this one has to take `{}`",
                                    needed.name, needed.name
                                ),
                            );
                        }
                    }
                }
            }
            found.filter(|_| first != name)
        });
        for read in reads {
            may_read(check, read, declared, first);
        }
    }
    drafts.into_iter().map(|(name, (declared, _))| (name, Arc::new(declared))).collect()
}

/// Says so if an expression reads what is not there to be read where it is.
fn may_read(check: &mut Check, read: &Read, action: &Declared, first: Option<&Declared>) {
    let mut names = vec!["with", "daemon"];
    if action.first.is_some() {
        names.push("first");
    }
    if read.after {
        names.extend(AFTER);
    }
    for path in read.expr.paths() {
        let (root, member) = (path[0].as_str(), path.get(1).map(String::as_str));
        if !names.contains(&root) {
            let later = if AFTER.contains(&root) {
                format!("; `{root}` is there once the program has run")
            } else {
                String::new()
            };
            check.problem(
                &read.at,
                format!(
                    "reads `{root}`, which is not there to read; here there are: {}{later}",
                    names.join(", ")
                ),
            );
            continue;
        }
        let Some(member) = member else { continue };
        let known: Option<Vec<&str>> = match root {
            "with" => Some(action.spec.args.iter().map(|arg| arg.name.as_str()).collect()),
            "daemon" => Some(DAEMON.iter().map(|(name, _)| *name).collect()),
            "first" => first.map(|first| first.out.iter().map(|(name, _)| name.as_str()).collect()),
            _ => None,
        };
        if let Some(known) = known.filter(|known| !known.contains(&member)) {
            let has = if known.is_empty() { "nothing".to_string() } else { known.join(", ") };
            check.problem(&read.at, format!("reads `{root}.{member}`, and `{root}` has: {has}"));
        }
    }
}

/// Reads one action. What its expressions read is checked by the caller.
fn one(
    check: &mut Check,
    package: &str,
    name: &str,
    at: &str,
    entry: &Value,
) -> Option<(Declared, Vec<Read>)> {
    let map = check.fields(at, entry, FIELDS)?;
    let mut reads = Vec::new();
    let mut note = |at: &str, exprs: Vec<Expr>, after: bool| {
        reads.extend(exprs.into_iter().map(|expr| Read { at: at.to_string(), expr, after }));
    };
    let description = check.text(&format!("{at}.description"), map.get("description"), true);
    let mut spec = ActionSpec::new(&format!("{package}.{name}"), &description)
        .capability(&format!("extensions:{package}"));

    let mut defaults = Vec::new();
    let with = match map.get("with") {
        None | Some(Value::Null) => None,
        Some(Value::Object(with)) => Some(with),
        Some(_) => {
            check.problem(&format!("{at}.with"), "must be a mapping from an argument's name to what it is");
            None
        }
    };
    for (arg, said) in with.into_iter().flatten() {
        let at = format!("{at}.with.{arg}");
        if !is_word(arg) {
            check.problem(&at, "must be named in lowercase letters, digits and `_`, starting with a letter");
            continue;
        }
        let Some(said) = check.fields(&at, said, &["type", "required", "default", "description"]) else {
            continue;
        };
        let default = said.get("default").filter(|default| !default.is_null());
        let ty = match said.get("type") {
            None | Some(Value::Null) => default.map_or(ValueType::Any, ValueType::of),
            Some(ty) => ty.as_str().and_then(ValueType::parse).unwrap_or_else(|| {
                let known: Vec<&str> = ValueType::ALL.iter().map(|ty| ty.name()).collect();
                check.problem(&format!("{at}.type"), format!("must be one of: {}", known.join(", ")));
                ValueType::Any
            }),
        };
        let required = match said.get("required") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(required)) => *required,
            Some(_) => {
                check.problem(&format!("{at}.required"), "must be `true` or `false`");
                false
            }
        };
        let help = check.text(&format!("{at}.description"), said.get("description"), false);
        spec = if required { spec.required(arg, ty, &help) } else { spec.arg(arg, ty, &help) };
        let Some(default) = default else { continue };
        if required {
            check.problem(&at, "is `required` and has a `default`, which it would never be given; say one");
        }
        // Worked out when the action is done, from where the daemon is: a
        // package cannot know that when it is written.
        let filled = exprs_in(default);
        if filled.is_empty() && !ty.accepts(default) {
            check.problem(&format!("{at}.default"), format!("must be {}, as its `type` says", ty.name()));
        }
        check.list(&format!("{at}.default"), Some(&Value::Array(vec![default.clone()])));
        for expr in filled {
            let reads: Vec<String> = expr.paths().into_iter().map(|path| path[0].clone()).collect();
            if let Some(other) = reads.iter().find(|root| *root != "daemon") {
                check.problem(
                    &format!("{at}.default"),
                    format!("reads `{other}`; a default can read `daemon` and nothing else"),
                );
            }
            note(&format!("{at}.default"), vec![expr], false);
        }
        defaults.push((arg.clone(), default.clone()));
    }

    // What it says it makes, for whoever reads in words what a blueprint
    // that asks it does: the thing, and which of its arguments names it.
    if let Some(said) = map.get("makes").filter(|said| !said.is_null())
        && let Some(said) = check.fields(&format!("{at}.makes"), said, &["what", "named"])
    {
        let what = check.text(&format!("{at}.makes.what"), said.get("what"), true);
        let named = check.text(&format!("{at}.makes.named"), said.get("named"), true);
        if !spec.args.iter().any(|arg| arg.name == named) {
            check.problem(
                &format!("{at}.makes.named"),
                "must be one of the arguments under `with`: the one that says what the thing made is called",
            );
        }
        spec = spec.making(&what, &named);
    }

    let first =
        Some(check.text(&format!("{at}.first"), map.get("first"), false)).filter(|first| !first.is_empty());

    let mut verdicts = |check: &mut Check, field: &str, after: bool| {
        let mut verdicts = Vec::new();
        let at = format!("{at}.{field}");
        if map.get(field).is_some_and(|value| !value.is_array() && !value.is_null()) {
            check.problem(&at, "must be a list, each with `when` and `says`");
        }
        for (index, entry) in map.get(field).and_then(Value::as_array).into_iter().flatten().enumerate() {
            let at = format!("{at}[{index}]");
            let Some(entry) = check.fields(&at, entry, &["when", "says"]) else { continue };
            let when = check.cond(&format!("{at}.when"), entry.get("when"));
            let says = check.template(&format!("{at}.says"), entry.get("says"));
            let (Some(when), Some(says)) = (when, says) else {
                check.problem(&at, "needs `when`, a condition, and `says`, the sentence it fails with");
                continue;
            };
            note(&format!("{at}.when"), vec![when.expr.clone()], after);
            note(&format!("{at}.says"), says.exprs().cloned().collect(), after);
            verdicts.push(Verdict { at, when, says });
        }
        verdicts
    };
    let refuse = verdicts(check, "refuse", false);
    let fails = verdicts(check, "fails", true);

    let args = check.args(&format!("{at}.args"), map.get("args"));
    for (index, entry) in args.iter().enumerate() {
        let at = format!("{at}.args[{index}]");
        note(&at, entry.when.iter().map(|when| when.expr.clone()).collect(), false);
        note(&at, entry.args.iter().flat_map(exprs_in).collect(), false);
    }

    let json = match map.get("answer") {
        None | Some(Value::Null) => false,
        Some(Value::String(how)) if how == "json" => true,
        Some(Value::String(how)) if how == "text" => false,
        Some(_) => {
            check.problem(
                &format!("{at}.answer"),
                "must be `json`, for a program that prints one JSON value, or `text`",
            );
            false
        }
    };

    let mut out = Vec::new();
    match map.get("out") {
        None | Some(Value::Null) => {}
        Some(Value::Object(gives)) => {
            for (field, value) in gives {
                let at = format!("{at}.out.{field}");
                if !is_word(field) {
                    check.problem(
                        &at,
                        "must be named in lowercase letters, digits and `_`, starting with a letter",
                    );
                    continue;
                }
                check.list(&at, Some(&Value::Array(vec![value.clone()])));
                note(&at, exprs_in(value), true);
                spec = spec.output(field, "");
                out.push((field.clone(), value.clone()));
            }
        }
        Some(_) => check.problem(
            &format!("{at}.out"),
            "must be a mapping from a name to the expression that is given out under it",
        ),
    }

    match map.get("repeatable") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => {}
        Some(Value::Bool(true)) => spec = spec.idempotent(),
        Some(_) => check.problem(&format!("{at}.repeatable"), "must be `true` or `false`"),
    }
    Some((Declared { spec, defaults, first, refuse, args, json, fails, out }, reads))
}

impl Declared {
    /// The action of the package that is done before this one, if it says one.
    pub fn first(&self) -> Option<&str> {
        self.first.as_deref()
    }

    /// What its expressions read before the program is run: `with`, which
    /// is what the step gave with what it left out filled in, and `daemon`.
    /// Arguments this action does not take are left out, so that one action
    /// can be handed another's.
    pub fn before(&self, given: &Fields, daemon: &Value) -> Result<Map<String, Value>, String> {
        let mut scope = crate::extension::scope("daemon", daemon.clone());
        let mut with = Map::new();
        for arg in &self.spec.args {
            let given = given.get(&arg.name).filter(|value| !value.is_null());
            let default = || self.defaults.iter().find(|(name, _)| name == &arg.name);
            let value = match (given, default()) {
                (Some(given), _) => given.clone(),
                (None, Some((_, default))) => template::render(default, &scope)
                    .map_err(|err| format!("the default of `{}` could not be worked out: {err}", arg.name))?,
                (None, None) => Value::Null,
            };
            with.insert(arg.name.clone(), value);
        }
        scope.insert("with".to_string(), Value::Object(with));
        Ok(scope)
    }

    /// The first of a list of verdicts that holds, as its sentence.
    fn verdict(verdicts: &[Verdict], scope: &Map<String, Value>) -> Result<Option<String>, String> {
        for verdict in verdicts {
            let at = &verdict.at;
            if verdict
                .when
                .test(scope)
                .map_err(|err| format!("`{at}.when` could not be worked out: {err}"))?
            {
                let says = verdict.says.render(scope);
                let says = says.map_err(|err| format!("`{at}.says` could not be filled in: {err}"))?;
                return Ok(Some(expr::display(&says)));
            }
        }
        Ok(None)
    }

    /// The sentence this is refused with, before the program is run.
    pub fn refused(&self, scope: &Map<String, Value>) -> Result<Option<String>, String> {
        Self::verdict(&self.refuse, scope)
    }

    /// The arguments the program is run with.
    pub fn argv(&self, scope: &Map<String, Value>) -> Result<Vec<String>, String> {
        let mut argv = Vec::new();
        for entry in self.args.iter().filter(|entry| holds(&entry.when, scope)) {
            for arg in &entry.args {
                let filled = template::render(arg, scope)
                    .map_err(|err| format!("`args` could not be filled in: {err}"))?;
                argv.push(expr::display(&filled));
            }
        }
        Ok(argv)
    }

    /// Adds what the program did to what its expressions read: `answer`,
    /// which is `null` for a program asked for JSON that printed something
    /// else, and `code`, `stdout` and `stderr`.
    pub fn ran(&self, scope: &mut Map<String, Value>, code: Option<i32>, stdout: &str, stderr: &str) {
        let answer = if self.json {
            serde_json::from_str(stdout).unwrap_or(Value::Null)
        } else {
            Value::String(stdout.to_string())
        };
        scope.insert("answer".to_string(), answer);
        scope.insert("code".to_string(), code.map_or(Value::Null, Value::from));
        scope.insert("stdout".to_string(), Value::String(stdout.to_string()));
        scope.insert("stderr".to_string(), Value::String(stderr.to_string()));
    }

    /// The sentence this failed with, going by what the program did.
    pub fn failed(&self, scope: &Map<String, Value>) -> Result<Option<String>, String> {
        Self::verdict(&self.fails, scope)
    }

    /// Whether it gives out something called `name`.
    pub fn gives_out(&self, name: &str) -> bool {
        self.out.iter().any(|(field, _)| field == name)
    }

    /// What the step gives out.
    pub fn gives(&self, scope: &Map<String, Value>) -> Result<Value, String> {
        let mut gives = Map::new();
        for (field, value) in &self.out {
            let filled = template::render(value, scope)
                .map_err(|err| format!("`out.{field}` could not be worked out: {err}"))?;
            gives.insert(field.clone(), filled);
        }
        Ok(Value::Object(gives))
    }
}
