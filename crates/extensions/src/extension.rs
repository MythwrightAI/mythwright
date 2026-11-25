//! Extension packages: what is not part of the product.
//!
//! The product knows how to run *an* agent that is a command-line program
//! printing one JSON value a line. It does not know any particular one. A
//! package is a folder with a manifest that says which program, how one turn
//! is put to it, how to read what it prints, and how to ask it which models
//! it offers. A package may also, or instead, bring blueprints
//! ([`crate::brought`]): how somebody's product is set up, written in the
//! language everything else is set up in; and actions
//! ([`crate::declared`]): ways of running its program that a blueprint then
//! does by name. Everything specific to a vendor
//! lives in such a package, and nothing in this crate (or any other) names
//! one: the `providers` gate checks that.
//!
//! The manifest is written in the language blueprints are written in: the
//! same strict subset of YAML, the same `${ }` expressions. [`Extension::load`]
//! checks it whole before any of it is used, and says what is wrong in terms
//! of where in the file it is.

use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use blueprint::template::{self, Template};
use blueprint::{Cond, expr};
use kernel::BoxFut;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::adapter::{self, Adapter, Failure, Reply, Turn};
use crate::handed::Handing;

/// The manifest's file name inside a package.
pub const MANIFEST: &str = "extension.yaml";

/// What the manifest a package was replaced over is kept as, beside the
/// one that replaced it.
pub const REPLACED: &str = "extension.was.yaml";

/// How long a program gets to say which models it offers.
const ASK_PATIENCE: Duration = Duration::from_secs(30);

/// A model an agent can be run on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// What the program is given to select it.
    pub id: String,
    /// What a person is shown.
    pub label: String,
    pub note: String,
    /// The one the program uses when none is named.
    pub default: bool,
}

/// Arguments that are passed when their condition holds, or always.
pub struct Args {
    pub when: Option<Cond>,
    pub args: Vec<Value>,
}

/// What to take from a line of output when its condition holds.
#[derive(Default)]
struct Rule {
    when: Option<Cond>,
    /// A list in the line to go through; each element is `item` in the rest.
    each: Option<Template>,
    session: Option<Template>,
    model: Option<Template>,
    show: Option<Template>,
    reply: Option<Template>,
    cost_usd: Option<Template>,
    input_tokens: Option<Template>,
    output_tokens: Option<Template>,
    /// Something its program was refused, each time it says one.
    refused: Option<Template>,
    failed: Option<Cond>,
    error: Option<Template>,
}

/// How to ask the program for its models.
struct Ask {
    args: Vec<Value>,
    /// Written to its standard input, one JSON value a line.
    send: Vec<Value>,
    /// Which line it prints holds the answer.
    answer: Cond,
    list: Template,
    id: Template,
    label: Option<Template>,
    note: Option<Template>,
    hidden: Option<Cond>,
    default: Option<Cond>,
    /// For a program whose list does not say which it uses when none is
    /// named: how to ask it that.
    usual: Option<Usual>,
}

/// A second question: which model the program uses when none is named.
struct Usual {
    args: Vec<Value>,
    /// Which line it prints holds the answer. A line is its JSON value, or
    /// for a line that is not JSON, its text.
    when: Cond,
    id: Template,
}

enum Models {
    /// Whatever is typed.
    Unknown,
    Listed(Vec<Model>),
    Asked(Box<Ask>),
}

/// A package, checked and ready to run.
pub struct Extension {
    pub name: String,
    pub title: String,
    pub description: String,
    /// The program it runs: its agent's, or the one its actions and
    /// blueprints run. Empty for a package that says none.
    pub program: String,
    /// What the package has to say to someone who does not have the
    /// program: how it is got and made ready.
    pub install: String,
    /// The manifest as written.
    pub source: String,
    /// Whether it brings an agent. One that does not brings actions,
    /// blueprints, or both.
    pub agent: bool,
    /// The actions it declares, by their own names: `serve` is done in a
    /// blueprint as `<package>.serve`.
    pub actions: BTreeMap<String, Arc<crate::declared::Declared>>,
    /// The blueprints it brings, under the names they are looked up by:
    /// `<package>/<name>`.
    pub blueprints: BTreeMap<String, String>,
    /// The files its blueprints link, under the names they are looked up
    /// by: `<package>/prompts/<file>`.
    pub prompts: BTreeMap<String, String>,
    /// What it shows of a repository it serves, beside what git says of it
    /// ([`crate::shown`]).
    pub shows: Option<crate::shown::Shows>,
    args: Vec<Args>,
    input: Option<Template>,
    output: Vec<Rule>,
    models: Models,
    /// The permission modes its program can be held to; `None` is all of them.
    permissions: Option<Vec<String>>,
    /// How its program is handed a sign-in the factory holds. A package
    /// that does not say cannot be handed one.
    pub signin: Option<Handing>,
    /// What its program needs behind a wall, besides what any does.
    pub wall: crate::walled::Needs,
}

// ---- reading the manifest ---------------------------------------------------------

/// Collects what is wrong with a manifest, each as a sentence saying where.
pub struct Check {
    problems: Vec<String>,
}

impl Check {
    pub fn problem(&mut self, at: &str, what: impl std::fmt::Display) {
        self.problems.push(format!("`{at}` {what}"));
    }

    /// The fields of a mapping, refusing any that are not known: a manifest
    /// that accepts anything cannot say what was misspelled.
    pub fn fields<'a>(
        &mut self,
        at: &str,
        value: &'a Value,
        known: &[&str],
    ) -> Option<&'a Map<String, Value>> {
        let Some(map) = value.as_object() else {
            self.problem(at, "must be a mapping");
            return None;
        };
        for key in map.keys().filter(|key| !known.contains(&key.as_str())) {
            self.problem(
                &format!("{at}.{key}"),
                format!("is not a known field; it can have: {}", known.join(", ")),
            );
        }
        Some(map)
    }

    pub fn text(&mut self, at: &str, value: Option<&Value>, needed: bool) -> String {
        match value {
            Some(Value::String(text)) if !text.trim().is_empty() => text.trim().to_string(),
            None | Some(Value::Null) if !needed => String::new(),
            None | Some(Value::Null) => {
                self.problem(at, "is needed");
                String::new()
            }
            Some(_) => {
                self.problem(at, "must be text");
                String::new()
            }
        }
    }

    pub fn template(&mut self, at: &str, value: Option<&Value>) -> Option<Template> {
        let text = match value? {
            Value::String(text) => text.clone(),
            Value::Null => return None,
            other => expr::display(other),
        };
        match Template::parse(&text) {
            Ok(template) => Some(template),
            Err(err) => {
                self.problem(at, format!("does not read as an expression: {}", err.message));
                None
            }
        }
    }

    pub fn cond(&mut self, at: &str, value: Option<&Value>) -> Option<Cond> {
        let text = match value? {
            Value::String(text) => text.clone(),
            Value::Null => return None,
            other => expr::display(other),
        };
        match Cond::parse(&text) {
            Ok(cond) => Some(cond),
            Err(err) => {
                self.problem(at, format!("does not read as a condition: {}", err.message));
                None
            }
        }
    }

    /// A list whose every `${ }` reads; the values are kept as written and
    /// filled in when used.
    pub fn list(&mut self, at: &str, value: Option<&Value>) -> Vec<Value> {
        let items = match value {
            Some(Value::Array(items)) => items.clone(),
            None | Some(Value::Null) => return Vec::new(),
            Some(_) => {
                self.problem(at, "must be a list");
                return Vec::new();
            }
        };
        for item in &items {
            template::each_template(item, &mut |text| {
                if let Err(err) = Template::parse(text) {
                    self.problem(
                        at,
                        format!("has `{text}`, which does not read as an expression: {}", err.message),
                    );
                }
            });
        }
        items
    }

    /// How a program is started: a list of entries, each a list of
    /// arguments that are always passed, or a mapping with `when` and
    /// `args`. An agent's turn and a package's action say it the same way.
    pub fn args(&mut self, at: &str, value: Option<&Value>) -> Vec<Args> {
        let mut args = Vec::new();
        for (index, entry) in value.and_then(Value::as_array).into_iter().flatten().enumerate() {
            let at = format!("{at}[{index}]");
            match entry {
                Value::Array(_) => args.push(Args { when: None, args: self.list(&at, Some(entry)) }),
                Value::String(_) => {
                    args.push(Args { when: None, args: self.list(&at, Some(&json!([entry]))) })
                }
                _ => {
                    if let Some(map) = self.fields(&at, entry, &["when", "args"]) {
                        let when = self.cond(&format!("{at}.when"), map.get("when"));
                        args.push(Args { when, args: self.list(&format!("{at}.args"), map.get("args")) });
                    }
                }
            }
        }
        if value.is_some_and(|value| !value.is_array() && !value.is_null()) {
            self.problem(at, "must be a list");
        }
        args
    }
}

const RULE_FIELDS: &[&str] = &[
    "when",
    "each",
    "session",
    "model",
    "show",
    "reply",
    "cost_usd",
    "input_tokens",
    "output_tokens",
    "refused",
    "failed",
    "error",
];

/// What a package says of the agent it brings.
struct Agent {
    program: String,
    args: Vec<Args>,
    input: Option<Template>,
    output: Vec<Rule>,
    models: Models,
    permissions: Option<Vec<String>>,
    signin: Option<Handing>,
    wall: crate::walled::Needs,
}

impl Extension {
    /// Reads a manifest. Everything wrong with it is reported at once.
    pub fn load(source: &str) -> Result<Extension, Vec<String>> {
        let parsed = blueprint::syntax::parse(source)
            .map_err(|err| vec![format!("line {}: {}", err.span.start.line, err.message)])?;
        let root = parsed.to_json();
        let mut check = Check { problems: Vec::new() };
        let known = [
            "extension",
            "title",
            "description",
            "program",
            "install",
            "agent",
            "actions",
            "blueprints",
            "prompts",
            "repository",
        ];
        let Some(top) = check.fields("the manifest", &root, &known) else {
            return Err(check.problems);
        };
        let name = check.text("extension", top.get("extension"), true);
        // `std` is where the blueprints that ship are kept: a package of
        // that name would be bringing blueprints under theirs.
        // And `external` is what an agent no program here runs is said to
        // run on (`agents::external`).
        let taken = ["builtin", "std", crate::EXTERNAL];
        if !name.is_empty() && (name != expr::slug(&name) || taken.contains(&name.as_str())) {
            check.problem(
                "extension",
                "must be a short name of lowercase letters, digits and dashes, and not `builtin`, `std` or \
                 `external`",
            );
        }
        let title = check.text("title", top.get("title"), false);
        let description = check.text("description", top.get("description"), false);

        let agent = match top.get("agent") {
            None | Some(Value::Null) => None,
            Some(agent) => Self::agent(&mut check, agent),
        };
        let brings_agent = top.get("agent").is_some_and(|agent| !agent.is_null());
        // Its actions are read before its blueprints, which may do them.
        let actions = crate::declared::read(&mut check, &name, top.get("actions"));
        let blueprints = crate::brought::read(&mut check, &name, top.get("blueprints"), &actions);
        let prompts = crate::brought::linked(&mut check, &name, top.get("prompts"));
        crate::brought::fitted(&mut check, &blueprints, &prompts);
        let shows = crate::shown::read_shows(&mut check, top.get("repository"), &actions);
        let brings = |what: &str| top.get(what).is_some_and(|value| !value.is_null());
        if !brings_agent && !brings("actions") && !brings("blueprints") {
            check.problem(
                "the manifest",
                "brings nothing: it needs `agent`, which says how a program is run as an agent, `actions`, \
                 `blueprints`, or more than one of them",
            );
        }
        // The program is said once. With an agent it is the agent's.
        let own = check.text("program", top.get("program"), false);
        if brings_agent && !own.is_empty() {
            check.problem("program", "is said under `agent` when the package brings one; say it there only");
        }
        if !brings_agent && brings("actions") && own.is_empty() {
            check.problem("program", "is needed: it is what the package's actions run");
        }
        let install = check.text("install", top.get("install"), false);

        if !check.problems.is_empty() {
            return Err(check.problems);
        }
        let agent = agent.unwrap_or(Agent {
            program: own,
            args: Vec::new(),
            input: None,
            output: Vec::new(),
            models: Models::Unknown,
            permissions: None,
            signin: None,
            wall: Default::default(),
        });
        Ok(Extension {
            name,
            title,
            description,
            program: agent.program,
            install,
            source: source.to_string(),
            agent: brings_agent,
            actions,
            blueprints,
            prompts,
            shows,
            args: agent.args,
            input: agent.input,
            output: agent.output,
            models: agent.models,
            permissions: agent.permissions,
            signin: agent.signin,
            wall: agent.wall,
        })
    }

    /// Reads what a manifest says under `agent`.
    fn agent(check: &mut Check, agent: &Value) -> Option<Agent> {
        let known = ["program", "args", "input", "output", "models", "permissions", "signin", "wall"];
        let agent = check.fields("agent", agent, &known)?;
        let program = check.text("agent.program", agent.get("program"), true);

        let args = check.args("agent.args", agent.get("args"));
        let input = check.template("agent.input", agent.get("input"));

        let mut output = Vec::new();
        for (index, entry) in agent.get("output").and_then(Value::as_array).into_iter().flatten().enumerate()
        {
            let at = format!("agent.output[{index}]");
            let Some(map) = check.fields(&at, entry, RULE_FIELDS) else { continue };
            let field =
                |check: &mut Check, name: &str| check.template(&format!("{at}.{name}"), map.get(name));
            output.push(Rule {
                when: check.cond(&format!("{at}.when"), map.get("when")),
                each: field(check, "each"),
                session: field(check, "session"),
                model: field(check, "model"),
                show: field(check, "show"),
                reply: field(check, "reply"),
                cost_usd: field(check, "cost_usd"),
                input_tokens: field(check, "input_tokens"),
                output_tokens: field(check, "output_tokens"),
                refused: field(check, "refused"),
                failed: check.cond(&format!("{at}.failed"), map.get("failed")),
                error: field(check, "error"),
            });
        }
        if !output.iter().any(|rule| rule.reply.is_some()) {
            check.problem(
                "agent.output",
                "needs at least one rule with `reply`, or no turn could ever finish",
            );
        }

        let models = match agent.get("models") {
            None | Some(Value::Null) => Models::Unknown,
            Some(models) => Self::models(check, models),
        };

        // Which of the permission modes its program can be held to. A
        // program that cannot be told to keep its hands off must say so
        // here, or an agent meant only to look would be let do anything.
        let permissions = match agent.get("permissions") {
            None | Some(Value::Null) => None,
            Some(Value::Array(modes)) => {
                let known: Vec<&str> = crate::PERMISSIONS.iter().map(|(name, _)| *name).collect();
                let mut held = Vec::new();
                for mode in modes {
                    match mode.as_str().filter(|mode| known.contains(mode)) {
                        Some(mode) => held.push(mode.to_string()),
                        None => check.problem(
                            "agent.permissions",
                            format!("lists {mode}, which is not one of {}", known.join(", ")),
                        ),
                    }
                }
                if held.is_empty() {
                    check.problem(
                        "agent.permissions",
                        "is empty: leave it out if the program can be held to every mode",
                    );
                }
                Some(held)
            }
            Some(_) => {
                check.problem("agent.permissions", "must be a list, such as [full]");
                None
            }
        };

        let signin = match agent.get("signin") {
            None | Some(Value::Null) => None,
            Some(signin) => Handing::read(check, signin),
        };

        // What of the device its program must be shown behind a wall to
        // run at all and to be signed in.
        let wall = match agent.get("wall") {
            None | Some(Value::Null) => Default::default(),
            Some(wall) => crate::walled::Needs::read(check, wall),
        };

        Some(Agent { program, args, input, output, models, permissions, signin, wall })
    }

    fn models(check: &mut Check, models: &Value) -> Models {
        let Some(map) = check.fields("agent.models", models, &["ask", "list"]) else {
            return Models::Unknown;
        };
        if let Some(ask) = map.get("ask") {
            let known =
                ["args", "send", "answer", "list", "id", "label", "note", "hidden", "default", "usual"];
            let Some(ask) = check.fields("agent.models.ask", ask, &known) else { return Models::Unknown };
            let at = |name: &str| format!("agent.models.ask.{name}");
            let answer = check.cond(&at("answer"), ask.get("answer"));
            let list = check.template(&at("list"), ask.get("list"));
            let id = check.template(&at("id"), ask.get("id"));
            let (Some(answer), Some(list), Some(id)) = (answer, list, id) else {
                check.problem("agent.models.ask", "needs `answer`, `list` and `id`");
                return Models::Unknown;
            };
            let usual = match ask.get("usual") {
                None | Some(Value::Null) => None,
                Some(usual) => check.fields(&at("usual"), usual, &["args", "when", "id"]).and_then(|usual| {
                    let when = check.cond(&at("usual.when"), usual.get("when"));
                    let id = check.template(&at("usual.id"), usual.get("id"));
                    match (when, id) {
                        (Some(when), Some(id)) => {
                            Some(Usual { args: check.list(&at("usual.args"), usual.get("args")), when, id })
                        }
                        _ => {
                            check.problem(&at("usual"), "needs `when` and `id`");
                            None
                        }
                    }
                }),
            };
            return Models::Asked(Box::new(Ask {
                usual,
                args: check.list(&at("args"), ask.get("args")),
                send: check.list(&at("send"), ask.get("send")),
                answer,
                list,
                id,
                label: check.template(&at("label"), ask.get("label")),
                note: check.template(&at("note"), ask.get("note")),
                hidden: check.cond(&at("hidden"), ask.get("hidden")),
                default: check.cond(&at("default"), ask.get("default")),
            }));
        }
        let mut listed = Vec::new();
        for (index, entry) in map.get("list").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let at = format!("agent.models.list[{index}]");
            let Some(model) = check.fields(&at, entry, &["id", "label", "note", "default"]) else { continue };
            let id = check.text(&format!("{at}.id"), model.get("id"), true);
            let label = check.text(&format!("{at}.label"), model.get("label"), false);
            listed.push(Model {
                label: if label.is_empty() { id.clone() } else { label },
                id,
                note: check.text(&format!("{at}.note"), model.get("note"), false),
                default: model.get("default").and_then(Value::as_bool).unwrap_or(false),
            });
        }
        Models::Listed(listed)
    }

    /// Whether the program this package runs can be found. A package that
    /// names none has nothing to be missing.
    pub fn available(&self) -> bool {
        if self.program.is_empty() {
            return true;
        }
        let program = std::path::Path::new(&self.program);
        if program.components().count() > 1 {
            return program.is_file();
        }
        let paths = std::env::var_os("PATH");
        if cfg!(windows) {
            // There a program is a file whose name ends as the system's
            // list says, a script of its shell among them: found as it
            // is when it is started.
            let endings = std::env::var("PATHEXT").ok();
            return hosting::named::found(&self.program, paths.as_deref(), endings.as_deref()).is_some();
        }
        paths.is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(&self.program).is_file()))
    }

    /// The permission modes this package's program can be held to.
    pub fn permissions(&self) -> Vec<String> {
        match &self.permissions {
            Some(held) => held.clone(),
            None => crate::PERMISSIONS.iter().map(|(name, _)| name.to_string()).collect(),
        }
    }

    /// How this package comes by its models: `none`, `listed` or `asked`.
    pub fn models_kind(&self) -> &'static str {
        match self.models {
            Models::Unknown => "none",
            Models::Listed(_) => "listed",
            Models::Asked(_) => "asked",
        }
    }
}

// ---- one turn ------------------------------------------------------------------------

pub fn scope(name: &str, value: Value) -> Map<String, Value> {
    let mut scope = Map::new();
    scope.insert(name.to_string(), value);
    scope
}

/// A template's value, or nothing if it does not apply to this line. A rule
/// reaching for something a line does not have is not an error: lines differ.
fn value(template: &Option<Template>, scope: &Map<String, Value>) -> Option<Value> {
    template.as_ref()?.render(scope).ok().filter(|value| !value.is_null())
}

pub fn holds(cond: &Option<Cond>, scope: &Map<String, Value>) -> bool {
    cond.as_ref().is_none_or(|cond| cond.test(scope).unwrap_or(false))
}

/// What a program has printed so far in one turn, read by a package's rules.
pub struct Reading<'a> {
    extension: &'a Extension,
    pub reply: Reply,
    /// The reply has been seen; the turn has an answer.
    pub finished: bool,
    pub failure: Option<String>,
}

impl<'a> Reading<'a> {
    pub fn new(extension: &'a Extension) -> Self {
        Self { extension, reply: Reply::default(), finished: false, failure: None }
    }

    /// Takes one line of output. Lines that are not JSON say nothing.
    pub fn line(&mut self, line: &str, show: &mut dyn FnMut(&str)) {
        let Ok(event) = serde_json::from_str::<Value>(line) else { return };
        let mut at = scope("event", event);
        for rule in &self.extension.output {
            if !holds(&rule.when, &at) {
                continue;
            }
            match value(&rule.each, &at) {
                Some(Value::Array(items)) => {
                    for item in items {
                        at.insert("item".to_string(), item);
                        self.apply(rule, &at, show);
                    }
                    at.remove("item");
                }
                _ if rule.each.is_some() => {}
                _ => self.apply(rule, &at, show),
            }
        }
    }

    fn apply(&mut self, rule: &Rule, at: &Map<String, Value>, show: &mut dyn FnMut(&str)) {
        if let Some(Value::String(session)) = value(&rule.session, at) {
            self.reply.session = Some(session);
        }
        if let Some(Value::String(model)) = value(&rule.model, at) {
            self.reply.model = Some(model);
        }
        if let Some(text) =
            value(&rule.show, at).map(|value| expr::display(&value)).filter(|text| !text.is_empty())
        {
            show(&text);
        }
        if let Some(text) = value(&rule.reply, at) {
            self.reply.text = expr::display(&text);
            self.finished = true;
        }
        if let Some(cost) = value(&rule.cost_usd, at).and_then(|value| value.as_f64()) {
            self.reply.cost_usd = Some(cost);
        }
        let count = |template: &Option<Template>| value(template, at).and_then(|value| value.as_f64());
        if let Some(tokens) = count(&rule.input_tokens) {
            self.reply.input_tokens = tokens.max(0.0) as u64;
        }
        if let Some(tokens) = count(&rule.output_tokens) {
            self.reply.output_tokens = tokens.max(0.0) as u64;
        }
        if let Some(word) =
            value(&rule.refused, at).map(|value| expr::display(&value)).filter(|word| !word.is_empty())
        {
            self.reply.refused.push(word);
        }
        if rule.failed.as_ref().is_some_and(|cond| cond.test(at).unwrap_or(false)) {
            let said =
                value(&rule.error, at).map(|value| expr::display(&value)).filter(|text| !text.is_empty());
            self.failure = Some(said.unwrap_or_else(|| "it reported an error".to_string()));
        }
    }
}

impl Extension {
    /// The arguments for one turn.
    fn turn_args(&self, turn: &Value) -> Vec<String> {
        let at = scope("turn", turn.clone());
        let mut out = Vec::new();
        for entry in self.args.iter().filter(|entry| holds(&entry.when, &at)) {
            for arg in &entry.args {
                out.push(template::render(arg, &at).map(|value| expr::display(&value)).unwrap_or_default());
            }
        }
        out
    }

    /// What its program is started with for `turn`, as the package says:
    /// its arguments, in order. A package's examples hold it to them.
    pub fn started_with(&self, turn: &Turn) -> Vec<String> {
        self.turn_args(&Self::turn_value(turn))
    }

    /// What a turn looks like to a manifest's expressions.
    fn turn_value(turn: &Turn) -> Value {
        json!({
            "slot": turn.slot,
            "system": turn.system,
            "prompt": turn.prompt,
            "session": turn.session,
            "model": turn.model,
            "permission": turn.permission_mode,
            "workdir": turn.workdir.to_string_lossy(),
            // The factory's own command: what it is called, and where it is
            // for this program.
            "cli": brand::CLI,
            "cli_path": turn.cli.as_ref().map(|at| at.to_string_lossy()),
            "web": turn.web,
        })
    }

    /// Asks the program which models it offers, if the package says how.
    /// It is started as everything the daemon starts is (`hosting`), and
    /// signed in as whoever runs the daemon: no slot is asking.
    pub async fn ask_models(&self, hosting: Option<&hosting::Hosting>) -> Result<Vec<Model>, String> {
        self.asked(adapter::asking(&self.program, hosting), hosting, false).await
    }

    /// Whether its program can be asked something that is no prompt.
    pub fn answers_unprompted(&self) -> bool {
        matches!(self.models, Models::Asked(_))
    }

    /// The same asked of the program as `command` starts it: as a slot's
    /// turn would, behind its wall, to see that it starts and answers
    /// there. `heard`, what it printed on standard error is said when it
    /// does not answer.
    pub async fn asked(
        &self,
        mut command: tokio::process::Command,
        hosting: Option<&hosting::Hosting>,
        heard: bool,
    ) -> Result<Vec<Model>, String> {
        let ask = match &self.models {
            Models::Unknown => return Ok(Vec::new()),
            Models::Listed(models) => return Ok(models.clone()),
            Models::Asked(ask) => ask,
        };
        let nothing = Map::new();
        let text = |value: &Value| {
            template::render(value, &nothing).map(|value| expr::display(&value)).unwrap_or_default()
        };
        command
            .args(ask.args.iter().map(text))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(if heard { Stdio::piped() } else { Stdio::null() });
        let mut child = adapter::spawn(&mut command, &self.program).await?;
        let group = adapter::Group::of(&child);
        let stderr = child.stderr.take();
        let mut stdin = child.stdin.take().ok_or("the program takes no input")?;
        for line in &ask.send {
            let line = template::render(line, &nothing).unwrap_or_else(|_| line.clone());
            let _ = stdin.write_all(format!("{line}\n").as_bytes()).await;
        }
        let _ = stdin.flush().await;
        // Its input is left open: some programs take the end of their input
        // as the end of the conversation and leave before answering.
        let stdout = child.stdout.take().ok_or("the program prints nothing")?;
        let mut lines = BufReader::new(stdout).lines();
        // The answer is a line it prints. A program that prints one value
        // over many lines and ends has answered with all of it.
        let listed = |answer: Value| -> Option<Result<Vec<Value>, String>> {
            let at = scope("line", answer);
            if !ask.answer.test(&at).unwrap_or(false) {
                return None;
            }
            Some(match ask.list.render(&at) {
                Ok(Value::Array(items)) => Ok(items),
                // Models kept under their names: each is then its `key` and its `value`.
                Ok(Value::Object(named)) => {
                    Ok(named.into_iter().map(|(key, value)| json!({ "key": key, "value": value })).collect())
                }
                _ => Err(format!("`{}` answered, but not with a list of models", self.program)),
            })
        };
        let answer = async {
            let mut all = String::new();
            while let Ok(Some(line)) = lines.next_line().await {
                all.push_str(&line);
                all.push('\n');
                let Ok(line) = serde_json::from_str::<Value>(&line) else { continue };
                if let Some(found) = listed(line) {
                    return found;
                }
            }
            serde_json::from_str::<Value>(&all).ok().and_then(listed).unwrap_or_else(|| {
                Err(format!("`{}` ended without saying which models it offers", self.program))
            })
        };
        let answered = tokio::time::timeout(ASK_PATIENCE, answer).await.map_err(|_| {
            format!(
                "`{}` did not say which models it offers within {}s",
                self.program,
                ASK_PATIENCE.as_secs()
            )
        });
        let items = match answered.and_then(|answered| answered) {
            Ok(items) => items,
            Err(why) => {
                // Whatever it left running is stopped, so that all it had
                // to say for itself can be read to its end.
                drop(group);
                let said = adapter::tail(&adapter::errors(stderr).await);
                return Err(if said.is_empty() { why } else { format!("{why}: {said}") });
            }
        };
        let mut models = Vec::new();
        for item in items {
            let at = scope("model", item);
            if ask.hidden.as_ref().is_some_and(|cond| cond.test(&at).unwrap_or(false)) {
                continue;
            }
            let Some(id) = ask.id.render(&at).ok().filter(|id| !id.is_null()).map(|id| expr::display(&id))
            else {
                continue;
            };
            let field = |template: &Option<Template>| value(template, &at).map(|value| expr::display(&value));
            models.push(Model {
                label: field(&ask.label).unwrap_or_else(|| id.clone()),
                note: field(&ask.note).unwrap_or_default(),
                default: ask.default.as_ref().is_some_and(|cond| cond.test(&at).unwrap_or(false)),
                id,
            });
        }
        // Where the list did not say which is the usual one, the program
        // is asked that by itself, if the package knows how.
        if let Some(usual) = &ask.usual
            && !models.iter().any(|model| model.default)
            && let Some(id) = self.ask_usual(usual, hosting).await
            && let Some(model) = models.iter_mut().find(|model| model.id == id)
        {
            model.default = true;
        }
        Ok(models)
    }

    /// Asks the program which model it uses when none is named. Not being
    /// told is not a failure: the models are still the models.
    async fn ask_usual(&self, usual: &Usual, hosting: Option<&hosting::Hosting>) -> Option<String> {
        let nothing = Map::new();
        let args = usual.args.iter().map(|value| {
            template::render(value, &nothing).map(|value| expr::display(&value)).unwrap_or_default()
        });
        let mut command = adapter::asking(&self.program, hosting);
        command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = adapter::spawn(&mut command, &self.program).await.ok()?;
        let _group = adapter::Group::of(&child);
        let mut lines = BufReader::new(child.stdout.take()?).lines();
        let answer = async {
            while let Ok(Some(line)) = lines.next_line().await {
                let said = serde_json::from_str::<Value>(&line).unwrap_or(Value::String(line));
                let at = scope("line", said);
                if usual.when.test(&at).unwrap_or(false) {
                    return usual.id.render(&at).ok().filter(|id| !id.is_null()).map(|id| expr::display(&id));
                }
            }
            None
        };
        tokio::time::timeout(ASK_PATIENCE, answer).await.ok().flatten().map(|id| id.trim().to_string())
    }
}

/// Runs turns the way a package says to.
pub struct Packaged(pub Arc<Extension>);

impl Adapter for Packaged {
    fn run(&self, turn: Turn) -> BoxFut<'static, Result<Reply, Failure>> {
        let extension = self.0.clone();
        Box::pin(async move {
            let program = extension.program.clone();
            let described = Extension::turn_value(&turn);
            let input =
                value(&extension.input, &scope("turn", described.clone())).map(|value| expr::display(&value));
            // What the agent is asked goes in on standard input where the
            // package allows it, so it is never in the process list and
            // cannot be taken for a flag.
            let mut running = crate::running::begin(&turn, &program, |command| {
                command.args(extension.turn_args(&described));
                input.unwrap_or_default()
            })
            .await?;

            let mut reading = Reading::new(&extension);
            loop {
                let line = tokio::select! {
                    line = running.line() => line?,
                    () = turn.cancel.cancelled() => {
                        running.stop().await;
                        return Err("the turn was cancelled".into());
                    }
                };
                let Some(line) = line else { break };
                reading.line(&line, &mut |text| (turn.progress)(text));
            }
            let ended = running.ended().await?;
            if reading.failure.is_none() && reading.finished {
                running.done();
                return Ok(reading.reply);
            }
            // It failed. Whatever it left running is stopped now, so that
            // all it had to say for itself can be read to its end.
            let stderr = running.errors().await;
            running.done();
            let status = ended.said;
            let code = ended.code;
            let message = reading.failure.unwrap_or_else(|| {
                // It ended without saying how. Whatever it printed on the
                // way out is the best explanation there is.
                let tail = adapter::tail(&stderr);
                let detail = if tail.is_empty() { String::new() } else { format!(": {tail}") };
                format!("`{program}` ended ({status}) without a result{detail}")
            });
            Err(Failure { message, reply: reading.reply.text, code, stderr })
        })
    }
}

/// The packages in a folder, by name, and what is wrong with any that would
/// not load.
pub fn load_all(dir: &std::path::Path) -> (BTreeMap<String, Arc<Extension>>, Vec<String>) {
    let mut found = BTreeMap::new();
    let mut problems = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let manifest = entry.path().join(MANIFEST);
        let Ok(source) = std::fs::read_to_string(&manifest) else { continue };
        match Extension::load(&source) {
            Ok(extension) => {
                found.insert(extension.name.clone(), Arc::new(extension));
            }
            Err(wrong) => problems.push(format!("{}: {}", manifest.display(), wrong.join("; "))),
        }
    }
    (found, problems)
}
