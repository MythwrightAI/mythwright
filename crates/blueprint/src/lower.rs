//! From the syntax tree to the model.
//!
//! Every mapping in a blueprint has a closed set of fields. An unknown field
//! is an error that names the field and suggests the nearest real one, rather
//! than something silently ignored: a hook spelled `on_failure` that never
//! runs is exactly the kind of mistake a factory cannot afford.

use serde_json::{Map, Value as Json};

use crate::address;
use crate::diag::{Diagnostic, did_you_mean};
use crate::duration;
use crate::labels;
use crate::model::*;
use crate::syntax::{Entry, Node, Span, Value};
use crate::template::{Cond, Template, has_holes};
use crate::types::ValueType;
use crate::vocabulary::{ATTACHMENT, BLUEPRINT, HANDLER, HOOK, PARAM, POLL, RETRY, STEP, Shape, WATCH};

// What a handler is heard by and does (`on:`).
mod triggered;

/// The shortest interval a poller may use. Polling is how the previous system
/// melted its hub; it exists here only for things that cannot push, and never
/// tighter than this.
pub const MIN_POLL_MS: u64 = 5_000;

pub(crate) fn lower(root: &Node) -> (Option<Blueprint>, Vec<Diagnostic>) {
    let mut cx = Lowering { diagnostics: Vec::new() };
    let blueprint = cx.blueprint(root);
    (blueprint, cx.diagnostics)
}

struct Lowering {
    diagnostics: Vec<Diagnostic>,
}

fn field<'a>(entries: &'a [Entry], key: &str) -> Option<&'a Node> {
    entries.iter().find(|entry| entry.key == key).map(|entry| &entry.value)
}

pub(crate) fn is_ident(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_blueprint_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_./".contains(c))
}

impl Lowering {
    fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(code, message, span));
    }

    /// The entries of `node` if it is a mapping with only the fields of `shape`.
    fn fields<'a>(&mut self, node: &'a Node, what: &str, shape: &Shape) -> Option<&'a [Entry]> {
        let Some(entries) = node.as_map() else {
            self.error("wrong-type", format!("{what} must be a mapping, found {}", node.kind()), node.span);
            return None;
        };
        for entry in entries {
            if !shape.has(&entry.key) {
                let hint = match did_you_mean(&entry.key, shape.names()) {
                    Some(close) => format!("did you mean `{close}`?"),
                    None => format!("expected one of: {}", shape.names().collect::<Vec<_>>().join(", ")),
                };
                self.error(
                    "unknown-field",
                    format!("unknown field `{}` in {what}; {hint}", entry.key),
                    entry.key_span,
                );
            }
        }
        Some(entries)
    }

    fn string(&mut self, node: &Node, what: &str) -> Option<String> {
        match &node.value {
            Value::Str(text) => Some(text.clone()),
            Value::Int(number) => Some(number.to_string()),
            _ => {
                self.error("wrong-type", format!("{what} must be text, found {}", node.kind()), node.span);
                None
            }
        }
    }

    fn ident(&mut self, text: &str, what: &str, span: Span) -> bool {
        if is_ident(text) {
            return true;
        }
        self.error(
            "bad-name",
            format!(
                "{what} `{text}` must start with a letter and use only letters, digits and `_` \
                 (so it can be written in an expression)"
            ),
            span,
        );
        false
    }

    fn boolean(&mut self, node: &Node, what: &str) -> bool {
        match node.value {
            Value::Bool(flag) => flag,
            _ => {
                self.error("wrong-type", format!("{what} must be true or false"), node.span);
                false
            }
        }
    }

    fn millis(&mut self, node: &Node, what: &str) -> Option<u64> {
        let text = self.string(node, what)?;
        match duration::parse(&text) {
            Ok(millis) => Some(millis),
            Err(message) => {
                self.error("bad-duration", format!("{what}: {message}"), node.span);
                None
            }
        }
    }

    fn timing(&mut self, node: &Node, what: &str) -> Option<Timing> {
        let source = self.string(node, what)?;
        if has_holes(&source) {
            self.templates(node);
            return Some(Timing { source, fixed: None });
        }
        let fixed = self.millis(node, what)?;
        Some(Timing { source, fixed: Some(fixed) })
    }

    fn cond(&mut self, node: &Node, what: &str) -> Option<Cond> {
        let text = match &node.value {
            Value::Bool(flag) => flag.to_string(),
            Value::Str(text) => text.clone(),
            _ => {
                self.error("wrong-type", format!("{what} must be an expression"), node.span);
                return None;
            }
        };
        match Cond::parse(&text) {
            Ok(cond) => Some(cond),
            Err(err) => {
                self.error("bad-expression", format!("{what}: {}", err.message), node.span);
                None
            }
        }
    }

    /// Checks every `${ }` under `node` and returns the data.
    fn templates(&mut self, node: &Node) -> Json {
        match &node.value {
            Value::Str(text) if has_holes(text) => {
                if let Err(err) = Template::parse(text) {
                    self.error("bad-expression", err.message, node.span);
                }
            }
            Value::Seq(items) => items.iter().for_each(|item| {
                self.templates(item);
            }),
            Value::Map(entries) => entries.iter().for_each(|entry| {
                self.templates(&entry.value);
            }),
            _ => {}
        }
        node.to_json()
    }

    fn object(&mut self, node: &Node, what: &str) -> Map<String, Json> {
        match self.templates(node) {
            Json::Object(map) => map,
            Json::Null => Map::new(),
            _ => {
                self.error(
                    "wrong-type",
                    format!("{what} must be a mapping, found {}", node.kind()),
                    node.span,
                );
                Map::new()
            }
        }
    }

    /// `with:` is a mapping, or one `${ }` that produces a mapping.
    fn arguments(&mut self, node: Option<&Node>) -> Json {
        let Some(node) = node else { return Json::Object(Map::new()) };
        match &node.value {
            Value::Map(_) => self.templates(node),
            Value::Null => Json::Object(Map::new()),
            Value::Str(text) if has_holes(text) => self.templates(node),
            _ => {
                self.error(
                    "wrong-type",
                    format!("`with` must be a mapping, found {}", node.kind()),
                    node.span,
                );
                Json::Object(Map::new())
            }
        }
    }

    fn blueprint(&mut self, root: &Node) -> Option<Blueprint> {
        let entries = self.fields(root, "a blueprint", &BLUEPRINT)?;
        let name = match field(entries, "blueprint") {
            Some(node) => self.string(node, "`blueprint`").filter(|name| {
                let valid = is_blueprint_name(name);
                if !valid {
                    self.error(
                        "bad-name",
                        format!(
                            "blueprint name `{name}` must be lowercase letters, digits, `-`, `_`, `.` or `/`"
                        ),
                        node.span,
                    );
                }
                valid
            }),
            None => {
                self.error("missing-field", "a blueprint starts with `blueprint: <name>`", Span::default());
                None
            }
        };

        let mut blueprint = Blueprint {
            name: name.clone().unwrap_or_default(),
            version: None,
            description: None,
            params: Vec::new(),
            state: Map::new(),
            permissions: None,
            env: Map::new(),
            uses: Vec::new(),
            flow: Flow::default(),
            triggers: Vec::new(),
            outputs: Map::new(),
            on_success: Vec::new(),
            on_fail: Vec::new(),
            on_finish: Vec::new(),
            on_stop: Vec::new(),
            on_update: None,
            max_parallel: None,
            one_at_a_time: false,
            durable: true,
            span: root.span,
        };

        for entry in entries {
            let node = &entry.value;
            match entry.key.as_str() {
                "version" => blueprint.version = self.string(node, "`version`"),
                "description" => blueprint.description = self.string(node, "`description`"),
                "params" => blueprint.params = self.params(node),
                "state" => {
                    blueprint.state = self.object(node, "`state`");
                    for entry in node.as_map().unwrap_or_default() {
                        self.ident(&entry.key, "state key", entry.key_span);
                    }
                }
                "permissions" => blueprint.permissions = Some(self.string_list(node, "`permissions`")),
                "env" => blueprint.env = self.object(node, "`env`"),
                "uses" => blueprint.uses = self.attachments(node),
                "steps" => blueprint.flow = self.flow(node, "`steps`"),
                "on" => blueprint.triggers = self.triggers(node),
                "outputs" => blueprint.outputs = self.object(node, "`outputs`"),
                "on_success" => blueprint.on_success = self.hooks(node),
                "on_finish" => blueprint.on_finish = self.hooks(node),
                "on_fail" => blueprint.on_fail = self.hooks(node),
                "on_stop" => blueprint.on_stop = self.hooks(node),
                "on_update" => blueprint.on_update = Some(self.hooks(node)),
                "one_at_a_time" => blueprint.one_at_a_time = self.boolean(node, "`one_at_a_time`"),
                "durable" => blueprint.durable = self.boolean(node, "`durable`"),
                "max_parallel" => match node.value {
                    Value::Int(count) if count >= 1 => blueprint.max_parallel = Some(count as u32),
                    _ => self.error(
                        "wrong-type",
                        "`max_parallel` must be a whole number, 1 or more",
                        node.span,
                    ),
                },
                _ => {}
            }
        }
        // Only worth saying when nothing else is wrong: a blueprint whose steps
        // failed to read is not "empty", it is broken, and says so already.
        // One with `outputs` alone is a rule: it runs nothing, and gives
        // out what its outputs make of its params.
        let does_nothing = blueprint.flow.steps.is_empty()
            && blueprint.triggers.is_empty()
            && blueprint.uses.is_empty()
            && blueprint.outputs.is_empty();
        if does_nothing && self.diagnostics.is_empty() {
            self.error(
                "empty",
                "a blueprint needs `steps`, `on`, `uses` or `outputs`; this one does nothing and gives nothing out",
                root.span,
            );
        }
        name.map(|_| blueprint)
    }

    fn string_list(&mut self, node: &Node, what: &str) -> Vec<String> {
        match &node.value {
            Value::Seq(items) => items.iter().filter_map(|item| self.string(item, what)).collect(),
            Value::Str(text) => vec![text.clone()],
            Value::Null => Vec::new(),
            _ => {
                self.error("wrong-type", format!("{what} must be a list of names"), node.span);
                Vec::new()
            }
        }
    }

    fn params(&mut self, node: &Node) -> Vec<Param> {
        let Some(entries) = node.as_map() else {
            self.error("wrong-type", "`params` must be a mapping of name to default or spec", node.span);
            return Vec::new();
        };
        let mut params = Vec::new();
        for entry in entries {
            if !self.ident(&entry.key, "param", entry.key_span) {
                continue;
            }
            let mut param = Param {
                name: entry.key.clone(),
                ty: ValueType::Any,
                default: None,
                required: false,
                description: None,
                choices: Vec::new(),
                span: entry.key_span,
            };
            match &entry.value.value {
                // `name:` with nothing after it: required, any type.
                Value::Null => param.required = true,
                Value::Map(_) => {
                    let spec = self.fields(&entry.value, "a param", &PARAM).unwrap_or_default();
                    let mut typed = false;
                    if let Some(node) = field(spec, "type") {
                        let name = self.string(node, "`type`").unwrap_or_default();
                        match ValueType::parse(&name) {
                            Some(ty) => {
                                param.ty = ty;
                                typed = true;
                            }
                            None => {
                                let names: Vec<&str> = ValueType::ALL.iter().map(|t| t.name()).collect();
                                self.error(
                                    "bad-type",
                                    format!("unknown type `{name}`; expected one of: {}", names.join(", ")),
                                    node.span,
                                );
                            }
                        }
                    }
                    if let Some(node) = field(spec, "default") {
                        let default = node.to_json();
                        if !typed {
                            param.ty = ValueType::of(&default);
                        } else if !default.is_null() && !param.ty.accepts(&default) {
                            self.error(
                                "bad-default",
                                format!("the default for `{}` is not {}", param.name, param.ty.name()),
                                node.span,
                            );
                        }
                        param.default = Some(default);
                    }
                    if let Some(node) = field(spec, "required") {
                        param.required = self.boolean(node, "`required`");
                    } else {
                        param.required = param.default.is_none();
                    }
                    if let Some(node) = field(spec, "description") {
                        param.description = self.string(node, "`description`");
                    }
                    if let Some(node) = field(spec, "enum") {
                        match node.to_json() {
                            Json::Array(choices) => param.choices = choices,
                            _ => self.error("wrong-type", "`enum` must be a list", node.span),
                        }
                    }
                }
                _ => {
                    let default = entry.value.to_json();
                    param.ty = ValueType::of(&default);
                    param.default = Some(default);
                }
            }
            params.push(param);
        }
        params
    }

    fn attachments(&mut self, node: &Node) -> Vec<Attachment> {
        let Some(entries) = node.as_map() else {
            self.error("wrong-type", "`uses` must be a mapping of name to blueprint", node.span);
            return Vec::new();
        };
        let mut attachments = Vec::new();
        for entry in entries {
            if !self.ident(&entry.key, "attachment", entry.key_span) {
                continue;
            }
            let (blueprint, with) = match &entry.value.value {
                Value::Str(name) => (Some(name.clone()), Json::Object(Map::new())),
                _ => {
                    let spec = self.fields(&entry.value, "an attachment", &ATTACHMENT).unwrap_or_default();
                    let blueprint = field(spec, "blueprint").and_then(|n| self.string(n, "`blueprint`"));
                    if blueprint.is_none() && !spec.is_empty() {
                        self.error(
                            "missing-field",
                            "an attachment needs `blueprint: <name>`",
                            entry.key_span,
                        );
                    }
                    (blueprint, self.arguments(field(spec, "with")))
                }
            };
            if let Some(blueprint) = blueprint {
                attachments.push(Attachment {
                    key: entry.key.clone(),
                    blueprint,
                    with,
                    span: entry.key_span,
                });
            }
        }
        attachments
    }

    fn flow(&mut self, node: &Node, what: &str) -> Flow {
        let Some(entries) = node.as_map() else {
            self.error("wrong-type", format!("{what} must be a mapping of step name to step"), node.span);
            return Flow::default();
        };
        let mut flow = Flow::default();
        for entry in entries {
            if !self.ident(&entry.key, "step", entry.key_span) {
                continue;
            }
            let previous = flow.steps.last().map(|step: &Step| step.id.clone());
            if let Some(step) = self.step(entry, previous) {
                flow.steps.push(step);
            }
        }
        flow
    }

    fn step(&mut self, entry: &Entry, previous: Option<String>) -> Option<Step> {
        let id = &entry.key;
        let fields = self.fields(&entry.value, &format!("step `{id}`"), &STEP)?;
        let verbs: Vec<&Entry> =
            fields.iter().filter(|f| matches!(f.key.as_str(), "do" | "call" | "spawn" | "task")).collect();
        let mut inner = Flow::default();
        let kind = match verbs.as_slice() {
            [only] if only.key == "task" => {
                inner = self.flow(&only.value, "`task`");
                if only.value.as_map().is_some_and(|steps| steps.is_empty()) {
                    self.error(
                        "missing-field",
                        format!("task `{id}` has no steps; write the steps it groups under `task:`"),
                        only.key_span,
                    );
                }
                StepKind::Task
            }
            [only] => {
                let target = self.string(&only.value, &format!("`{}`", only.key))?;
                match only.key.as_str() {
                    "do" => StepKind::Do(target),
                    "call" => StepKind::Call(target),
                    _ => StepKind::Spawn(target),
                }
            }
            [] => {
                self.error(
                    "missing-field",
                    format!(
                        "step `{id}` needs one of `do: <action>`, `call: <blueprint>`, `spawn: <blueprint>` \
                         or `task:` with steps under it"
                    ),
                    entry.key_span,
                );
                return None;
            }
            [_, second, ..] => {
                self.error(
                    "conflict",
                    format!(
                        "step `{id}` has more than one of `do`, `call`, `spawn` and `task`; split it into \
                         separate steps"
                    ),
                    second.key_span,
                );
                return None;
            }
        };

        let (needs, needs_implicit) = match field(fields, "needs") {
            Some(node) => (self.string_list(node, "`needs`"), false),
            None => (previous.into_iter().collect(), true),
        };
        let mut step = Step {
            id: id.clone(),
            span: entry.key_span.to(entry.value.span),
            kind,
            with: self.arguments(field(fields, "with")),
            needs,
            needs_implicit,
            when: None,
            timeout: None,
            retry: None,
            allow_failure: false,
            again: false,
            watch: Vec::new(),
            on_start: Vec::new(),
            on_success: Vec::new(),
            on_finish: Vec::new(),
            on_fail: Vec::new(),
            on_timeout: Vec::new(),
            device: None,
            measure: None,
            labels: Map::new(),
            key: None,
            environment: None,
            for_each: None,
            description: None,
            env: Map::new(),
            asks: None,
            fail_code: None,
            inner,
        };
        for item in fields {
            let node = &item.value;
            match item.key.as_str() {
                "when" => step.when = self.cond(node, "`when`"),
                "timeout" => step.timeout = self.timing(node, "`timeout`"),
                "retry" => step.retry = self.retry(node),
                "allow_failure" => step.allow_failure = self.boolean(node, "`allow_failure`"),
                "again" => {
                    step.again = self.boolean(node, "`again`");
                    if !matches!(step.kind, StepKind::Do(_)) {
                        self.error(
                            "conflict",
                            "`again` is said of a step that does an action: a `call` or `spawn` finds the \
                             child it made, and each step of a task says it for itself",
                            item.key_span,
                        );
                    }
                }
                "watch" => step.watch = self.watches(node),
                "on_start" => step.on_start = self.hooks(node),
                "on_success" => step.on_success = self.hooks(node),
                "on_finish" => step.on_finish = self.hooks(node),
                "on_fail" => step.on_fail = self.hooks(node),
                "on_timeout" => step.on_timeout = self.hooks(node),
                "device" => {
                    step.device = self.string(node, "`device`");
                    self.templates(node);
                }
                "measure" => step.measure = self.string(node, "`measure`"),
                "labels" => {
                    let (labels, problems) = labels::read(item, &step.kind);
                    step.labels = labels;
                    self.diagnostics.extend(problems);
                }
                "as" => {
                    step.key = self.string(node, "`as`");
                    self.templates(node);
                    if !matches!(step.kind, StepKind::Call(_) | StepKind::Spawn(_)) {
                        self.error(
                            "conflict",
                            "`as` names a child blueprint; it only applies to `call` and `spawn`",
                            item.key_span,
                        );
                    }
                }
                "environment" => {
                    step.environment = self.string(node, "`environment`");
                    self.templates(node);
                    let shaped = |name: &str| {
                        has_holes(name)
                            || (!name.is_empty()
                                && name
                                    .chars()
                                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'))
                    };
                    if !matches!(step.kind, StepKind::Call(_) | StepKind::Spawn(_)) {
                        self.error(
                            "conflict",
                            "`environment` says what a child blueprint is started under; it only applies to `call` and `spawn`",
                            item.key_span,
                        );
                    } else if step.environment.as_deref().is_some_and(|name| !shaped(name)) {
                        self.error(
                            "wrong-type",
                            "`environment` names one of the project's environments: lowercase letters, digits and `-`, such as `prod`",
                            node.span,
                        );
                    }
                }
                "for_each" => step.for_each = self.cond(node, "`for_each`"),
                "description" => step.description = self.string(node, "`description`"),
                "env" => step.env = self.object(node, "`env`"),
                // An agent is named as it is spoken to in a channel, `@builder`,
                // or by its bare name.
                "if_fail_ask_agent" => {
                    step.asks = self
                        .string(node, "`if_fail_ask_agent`")
                        .map(|agent| agent.trim_start_matches('@').to_string())
                        .filter(|agent| !agent.is_empty());
                    if step.asks.is_none() {
                        self.error(
                            "wrong-type",
                            "`if_fail_ask_agent` names an agent, such as `@builder`",
                            node.span,
                        );
                    }
                }
                "else" => step.fail_code = self.string(node, "`else`"),
                _ => {}
            }
        }
        // Asking an agent is the last thing tried when the step fails: it is
        // told what failed, may put it right, and says whether the step is
        // to be run again. It is written here as the hook it is, so that
        // everything that knows about hooks knows about it. (The action is
        // the agents' own; a device with no agents says so when it checks.)
        match &step.asks {
            Some(agent) => {
                let (what, again) = match step.kind {
                    StepKind::Task => {
                        (format!("The task `{}` failed", step.id), "its steps run again from the first")
                    }
                    _ => (
                        format!("The step `{}` failed while running `{}`", step.id, step.kind.target()),
                        "the step run again",
                    ),
                };
                let prompt = format!(
                    "{what}:\n${{self.error}}\n\nPut it right if you can. \
                     When you have done what you can, choose `retry` to have {again}, \
                     or `fail` if it cannot be put right, and say why."
                );
                let ask = Hook {
                    action: "agent.consult".to_string(),
                    with: serde_json::json!({ "slot": agent, "prompt": prompt, "options": ["retry", "fail"] }),
                    when: None,
                    span: step.span,
                };
                // A step with hooks of its own for running out of time is
                // asked about there too; otherwise `on_fail` covers both.
                if !step.on_timeout.is_empty() {
                    step.on_timeout.push(ask.clone());
                }
                step.on_fail.push(ask);
            }
            None if step.fail_code.is_some() => self.error(
                "conflict",
                "`else` is what the step fails with when the agent asked could not put it right; it goes with `if_fail_ask_agent`",
                step.span,
            ),
            None => {}
        }
        if step.kind == StepKind::Task {
            self.task(&mut step, fields);
        }
        Some(step)
    }

    /// What is different about a step that is a task. It runs no action of
    /// its own, so what is said to an action means nothing on it and is
    /// refused rather than ignored.
    fn task(&mut self, step: &mut Step, fields: &[Entry]) {
        for (name, instead) in [
            ("with", "a task takes no arguments; give `with` to the steps inside it"),
            ("device", "a task runs on no device of its own; say `device` on the steps inside it"),
        ] {
            if let Some(entry) = fields.iter().find(|entry| entry.key == name) {
                self.error("conflict", instead, entry.key_span);
            }
        }
        if let Some(rule) = step.watch.iter().find(|rule| matches!(rule.cond, WatchCond::Log(_))) {
            self.error(
                "conflict",
                "a task prints nothing itself, so a `log` rule on it would never fire; put the rule on \
                 the step inside it whose output it is about",
                rule.hook.span,
            );
        }
        // The environment is the one thing here that is given to actions
        // and still has a meaning for a group of them: it is handed on.
        let env = std::mem::take(&mut step.env);
        inherit_env(&mut step.inner, &env);
    }

    fn retry(&mut self, node: &Node) -> Option<Retry> {
        let mut retry = Retry { max: 0, backoff_ms: 1_000, factor: 2.0, max_backoff_ms: 300_000 };
        match &node.value {
            Value::Int(max) if *max >= 0 => retry.max = *max as u32,
            Value::Map(_) => {
                let fields = self.fields(node, "`retry`", &RETRY)?;
                for item in fields {
                    let node = &item.value;
                    match (item.key.as_str(), &node.value) {
                        ("max", Value::Int(max)) if *max >= 0 => retry.max = *max as u32,
                        ("max", _) => self.error("wrong-type", "`max` must be a whole number", node.span),
                        ("backoff", _) => retry.backoff_ms = self.millis(node, "`backoff`").unwrap_or(1_000),
                        ("max_backoff", _) => {
                            retry.max_backoff_ms = self.millis(node, "`max_backoff`").unwrap_or(300_000);
                        }
                        ("factor", Value::Int(factor)) if *factor >= 1 => retry.factor = *factor as f64,
                        ("factor", Value::Float(factor)) if *factor >= 1.0 => retry.factor = *factor,
                        ("factor", _) => {
                            self.error("wrong-type", "`factor` must be a number, 1 or more", node.span)
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                self.error(
                    "wrong-type",
                    "`retry` must be a count or a mapping with `max` and `backoff`",
                    node.span,
                );
                return None;
            }
        }
        Some(retry)
    }

    /// One hook or a list of them.
    fn hooks(&mut self, node: &Node) -> Vec<Hook> {
        match &node.value {
            Value::Seq(items) => items.iter().filter_map(|item| self.hook(item)).collect(),
            Value::Null => Vec::new(),
            _ => self.hook(node).into_iter().collect(),
        }
    }

    fn hook(&mut self, node: &Node) -> Option<Hook> {
        // A bare word is an action with no arguments: `on_fail: [retry]`.
        if let Value::Str(action) = &node.value {
            return Some(Hook {
                action: action.clone(),
                with: Json::Object(Map::new()),
                when: None,
                span: node.span,
            });
        }
        let fields = self.fields(node, "a hook", &HOOK)?;
        self.hook_fields(fields, node.span)
    }

    /// The `do`/`emit`/`with`/`when` part shared by hooks and watch rules.
    fn hook_fields(&mut self, fields: &[Entry], span: Span) -> Option<Hook> {
        let when = field(fields, "when").and_then(|node| self.cond(node, "`when`"));
        match (field(fields, "do"), field(fields, "emit")) {
            (Some(action), None) => Some(Hook {
                action: self.string(action, "`do`")?,
                with: self.arguments(field(fields, "with")),
                when,
                span,
            }),
            // `emit: { topic: build, ... }` is short for `do: emit` with those arguments.
            (None, Some(event)) => {
                let with = match &event.value {
                    Value::Str(topic) => serde_json::json!({ "topic": topic }),
                    _ => self.arguments(Some(event)),
                };
                Some(Hook { action: "emit".to_string(), with, when, span })
            }
            (Some(_), Some(extra)) => {
                self.error("conflict", "use either `do` or `emit` here, not both", extra.span);
                None
            }
            (None, None) => {
                self.error("missing-field", "expected `do: <action>` or `emit: { topic: ... }`", span);
                None
            }
        }
    }

    fn watches(&mut self, node: &Node) -> Vec<Watch> {
        let Some(items) = node.as_seq() else {
            self.error("wrong-type", "`watch` must be a list of rules", node.span);
            return Vec::new();
        };
        let mut watches = Vec::new();
        for item in items {
            let Some(fields) = self.fields(item, "a watch rule", &WATCH) else { continue };
            let conds: Vec<&Entry> =
                fields.iter().filter(|f| matches!(f.key.as_str(), "after" | "every" | "log")).collect();
            let [only] = conds.as_slice() else {
                self.error(
                    "missing-field",
                    "a watch rule needs exactly one of `after: <duration>`, `every: <duration>` or `log: <pattern>`",
                    item.span,
                );
                continue;
            };
            let cond = match only.key.as_str() {
                "after" => self.timing(&only.value, "`after`").map(WatchCond::After),
                "every" => self.timing(&only.value, "`every`").map(WatchCond::Every),
                _ => self.string(&only.value, "`log`").map(WatchCond::Log),
            };
            let fail = field(fields, "fail").is_some_and(|node| self.boolean(node, "`fail`"));
            let hook = if field(fields, "do").is_some() || field(fields, "emit").is_some() {
                self.hook_fields(fields, item.span)
            } else if fail {
                // A rule that only aborts needs no action of its own.
                Some(Hook {
                    action: "noop".into(),
                    with: Json::Object(Map::new()),
                    when: None,
                    span: item.span,
                })
            } else {
                self.hook_fields(fields, item.span)
            };
            if let (Some(cond), Some(hook)) = (cond, hook) {
                watches.push(Watch { cond, hook, fail });
            }
        }
        watches
    }
}

fn shorthand_step(kind: StepKind, with: Json, entry: &Entry) -> Step {
    Step {
        id: "run".to_string(),
        span: entry.key_span.to(entry.value.span),
        kind,
        with,
        needs: Vec::new(),
        needs_implicit: true,
        when: None,
        timeout: None,
        retry: None,
        allow_failure: false,
        again: false,
        watch: Vec::new(),
        on_start: Vec::new(),
        on_success: Vec::new(),
        on_finish: Vec::new(),
        on_fail: Vec::new(),
        on_timeout: Vec::new(),
        device: None,
        measure: None,
        labels: Map::new(),
        key: None,
        environment: None,
        for_each: None,
        description: None,
        env: Map::new(),
        asks: None,
        fail_code: None,
        inner: Flow::default(),
    }
}

/// Gives every step of `flow` the environment of the task it is in. A step
/// that names a variable itself keeps its own value for it.
fn inherit_env(flow: &mut Flow, env: &Map<String, Json>) {
    for step in &mut flow.steps {
        if step.kind == StepKind::Task {
            inherit_env(&mut step.inner, env);
            continue;
        }
        for (key, value) in env {
            step.env.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}
