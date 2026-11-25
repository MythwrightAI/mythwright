//! Checks that need the whole blueprint: the step graph, what expressions
//! read, and whether the actions exist and are allowed.
//!
//! The aim is that a blueprint which passes here cannot fail at run time for a
//! reason that was knowable from the file. In particular a step may only read
//! the output of a step it depends on, so what a step sees never depends on
//! scheduling.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::address;
use crate::catalog::{ActionSpec, Catalog, intrinsic};
use crate::diag::{Diagnostic, did_you_mean};
use crate::expr::Expr;
use crate::heard::{Heard, Where};
use crate::model::*;
use crate::syntax::Span;
use crate::template::{Template, each_template, has_holes};
use crate::vocabulary::{
    Choice, EVENT_VIEW, INSTANCE_VIEW, ROOTS, RUN_VIEW, SOURCE_VIEW, STEP_VIEW, TARGET_VIEW,
};

/// Topic prefixes only the system may emit under.
pub const RESERVED_TOPICS: &[&str] = &[
    "run",
    "step",
    "instance",
    "service",
    "router",
    "agent",
    "channel",
    "inbox",
    "theme",
    "arranged",
    "clock",
    "board",
    "record",
    "project",
    "device",
    "gateway",
    "search",
    "metric",
    "system",
    "identity",
    "log",
    "hook",
    "poll",
    "handler",
    "collection",
    "topic",
    "extension",
    "client",
    "secret",
    "caller",
    "trash",
];

/// Hook actions that decide what happens to a failed step.
pub const DECISIONS: &[&str] = &["retry", "continue", "fail"];

pub(crate) fn validate(
    source: &str,
    blueprint: &Blueprint,
    catalog: &dyn Catalog,
    found: &mut Vec<Diagnostic>,
) {
    let mut cx = Validator { source, blueprint, catalog, diagnostics: found, capabilities: BTreeMap::new() };
    for attachment in &blueprint.uses {
        cx.blueprint_exists(&attachment.blueprint, attachment.span);
        cx.reads(&attachment.with, &Reads::top(), attachment.span);
    }
    cx.flow(&blueprint.flow, &Around::flow(false));
    for trigger in &blueprint.triggers {
        // What the handler hears is settled first: its `for:`, its `when:`
        // and its steps are then held to what such an event carries.
        let heard = cx.heard(trigger);
        cx.trigger(trigger, heard.as_ref());
        let polled = matches!(trigger.source, TriggerSource::Poll { .. });
        cx.flow(&trigger.flow, &Around::handler(heard.as_ref(), polled));
    }

    let all_main: BTreeSet<&str> = blueprint.flow.steps.iter().map(|step| step.id.as_str()).collect();
    let after_run = Reads { visible: Some(&all_main), ..Reads::top() };
    for value in blueprint.outputs.values().chain(blueprint.state.values()) {
        cx.reads(value, &after_run, blueprint.span);
    }
    for (name, value) in &blueprint.env {
        if cx.variable(name, value, blueprint.span) {
            cx.reads(value, &after_run, blueprint.span);
        }
    }
    // What is done when the steps have ended reads how they went, as `run`.
    let ended = Reads { of_run: true, ..after_run.clone() };
    for hook in [&blueprint.on_success, &blueprint.on_fail, &blueprint.on_finish].into_iter().flatten() {
        cx.hook(hook, &ended, false);
    }
    for hook in &blueprint.on_stop {
        cx.hook(hook, &after_run, false);
    }
    // The steps are not run when an instance takes a text this way, so
    // there is nothing of theirs to read; what it was is read as `was`.
    let carried = Reads { in_update: true, ..Reads::top() };
    for hook in blueprint.on_update.iter().flatten() {
        cx.hook(hook, &carried, false);
    }
    cx.permissions();
}

pub(crate) struct Validator<'a> {
    /// The text, to find where a name is written ([`crate::placed`]).
    pub(crate) source: &'a str,
    pub(crate) blueprint: &'a Blueprint,
    pub(crate) catalog: &'a dyn Catalog,
    diagnostics: &'a mut Vec<Diagnostic>,
    /// Capabilities the blueprint's actions need, with where first seen.
    capabilities: BTreeMap<String, Span>,
}

/// What an expression at some place in the file is allowed to read.
#[derive(Clone)]
struct Reads<'a> {
    /// Step ids whose results are settled here. `None` hides `steps`.
    visible: Option<&'a BTreeSet<&'a str>>,
    /// The step a hook belongs to, readable as `self` and by name.
    own: Option<&'a str>,
    /// The steps beside the one this is in, settled or not.
    beside: Option<&'a BTreeSet<&'a str>>,
    /// For what is inside a task: the steps outside it, as in [`Around`].
    outside: Option<&'a BTreeMap<&'a str, &'a str>>,
    in_handler: bool,
    in_loop: bool,
    /// In `on_update`, where `was` can be read.
    in_update: bool,
    /// In a handler whose events are known: what they carry.
    heard: Option<&'a Heard>,
    /// The steps this is among, where it is among steps: what one of them
    /// gives is held to what it is.
    graph: Option<&'a Flow>,
    /// In a `poll:` handler, where `event.previous` can be read.
    polled: bool,
    /// In what a blueprint does when its steps have ended, where `run`
    /// can be read.
    of_run: bool,
}

impl Reads<'_> {
    fn top() -> Self {
        Reads {
            visible: None,
            own: None,
            beside: None,
            outside: None,
            in_handler: false,
            in_loop: false,
            in_update: false,
            heard: None,
            graph: None,
            polled: false,
            of_run: false,
        }
    }
}

/// What is around a graph of steps: nothing for a blueprint's own steps or
/// a handler's, the task and everything outside it for the steps of a task.
struct Around<'a> {
    task: Option<&'a str>,
    /// Steps outside that have settled before any step here starts.
    settled: BTreeSet<&'a str>,
    /// Every step outside, settled or not, with the task that stands
    /// beside it: the one whose `needs` would make this graph wait for it.
    outside: BTreeMap<&'a str, &'a str>,
    in_handler: bool,
    in_loop: bool,
    heard: Option<&'a Heard>,
    /// In a handler that asks on a timer (`poll:`), whose event also has
    /// what was read the time before.
    polled: bool,
}

impl<'a> Around<'a> {
    fn flow(in_handler: bool) -> Self {
        Around {
            task: None,
            settled: BTreeSet::new(),
            outside: BTreeMap::new(),
            in_handler,
            in_loop: false,
            heard: None,
            polled: false,
        }
    }

    /// The steps of a handler, which read the event that fired it.
    fn handler(heard: Option<&'a Heard>, polled: bool) -> Self {
        Around { heard, polled, ..Around::flow(true) }
    }
}

impl Validator<'_> {
    pub(crate) fn error(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(code, message, span));
    }

    pub(crate) fn warning(&mut self, code: &'static str, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::warning(code, message, span));
    }

    /// Notes that the blueprint uses `capability`, to be held to what it
    /// lists under `permissions`.
    pub(crate) fn needs(&mut self, capability: &str, span: Span) {
        self.capabilities.entry(capability.to_string()).or_insert(span);
    }

    fn checks_actions(&self) -> bool {
        self.catalog.is_complete()
    }

    fn spec(&self, name: &str) -> Option<ActionSpec> {
        intrinsic(name).or_else(|| self.catalog.action(name).cloned())
    }

    fn flow(&mut self, flow: &Flow, place: &Around<'_>) {
        let ids: BTreeSet<&str> = flow.steps.iter().map(|step| step.id.as_str()).collect();
        for step in &flow.steps {
            // Inside a task `steps.<name>` is a step of the task or one
            // outside it. One name for both would make that a guess.
            if let (Some(task), true) = (place.task, place.outside.contains_key(step.id.as_str())) {
                self.error(
                    "conflict",
                    format!(
                        "step `{}` in the task `{task}` has the name of a step outside it, so \
                         `steps.{}` could mean either; give it another name",
                        step.id, step.id
                    ),
                    step.span,
                );
            }
            for need in &step.needs {
                if need == &step.id {
                    self.error("bad-needs", format!("step `{}` cannot need itself", step.id), step.span);
                } else if !ids.contains(need.as_str()) {
                    let hint = did_you_mean(need, ids.iter().copied())
                        .map(|close| format!("; did you mean `{close}`?"))
                        .unwrap_or_default();
                    self.error(
                        "bad-needs",
                        format!("step `{}` needs `{need}`, which is not a step here{hint}", step.id),
                        step.span,
                    );
                }
            }
        }
        if let Some(cycle) = find_cycle(flow) {
            self.error(
                "cycle",
                format!(
                    "these steps wait on each other and could never start: {}. \
                     A blueprint is a graph without loops; use `retry` or `for_each` to repeat work",
                    cycle.join(" -> ")
                ),
                flow.step(&cycle[0]).map(|step| step.span).unwrap_or_default(),
            );
            return;
        }

        let ancestors = ancestors(flow);
        for step in &flow.steps {
            let upstream: BTreeSet<&str> =
                ancestors[step.id.as_str()].union(&place.settled).copied().collect();
            let before = Reads {
                visible: Some(&upstream),
                own: None,
                beside: Some(&ids),
                outside: place.task.map(|_| &place.outside),
                in_handler: place.in_handler,
                in_loop: place.in_loop,
                in_update: false,
                heard: place.heard,
                graph: Some(flow),
                polled: place.polled,
                of_run: false,
            };
            if let Some(cond) = &step.when {
                self.expr(&cond.expr, &before, step.span);
            }
            if let Some(cond) = &step.for_each {
                self.expr(&cond.expr, &before, step.span);
            }
            let during = Reads { in_loop: place.in_loop || step.for_each.is_some(), ..before.clone() };
            self.reads(&step.with, &during, step.span);
            for (name, value) in &step.env {
                if self.variable(name, value, step.span) {
                    self.reads(value, &during, step.span);
                }
            }
            for value in step.labels.values() {
                self.reads(value, &during, step.span);
            }
            let watched = step.watch.iter().filter_map(|rule| rule.timing());
            for text in step.device.iter().chain(step.timeout.iter().chain(watched).map(|t| &t.source)) {
                self.reads(&Value::String(text.clone()), &during, step.span);
            }
            self.step_target(step);

            let around = Reads { own: Some(&step.id), ..during.clone() };
            for hook in &step.on_start {
                self.hook(hook, &around, false);
            }
            for hook in step.on_success.iter().chain(&step.on_finish) {
                self.hook(hook, &around, false);
            }
            for hook in step.on_fail.iter().chain(&step.on_timeout) {
                self.hook(hook, &around, true);
            }
            for watch in &step.watch {
                self.hook(&watch.hook, &around, false);
            }
            if step.kind == StepKind::Task {
                let mut outside = place.outside.clone();
                outside.extend(ids.iter().map(|id| (*id, step.id.as_str())));
                let inside = Around {
                    task: Some(&step.id),
                    settled: upstream.clone(),
                    outside,
                    in_handler: before.in_handler,
                    in_loop: during.in_loop,
                    heard: place.heard,
                    polled: place.polled,
                };
                self.flow(&step.inner, &inside);
            }
        }
    }

    fn step_target(&mut self, step: &Step) {
        match &step.kind {
            StepKind::Do(action) => {
                if DECISIONS.contains(&action.as_str()) && action != "fail" {
                    self.error(
                        "misplaced-decision",
                        format!("`{action}` decides what happens to a failed step; it belongs in `on_fail`, not as a step"),
                        step.span,
                    );
                    return;
                }
                self.action(action, &step.with, step.span);
            }
            StepKind::Call(name) | StepKind::Spawn(name) => {
                // A text may call itself where the step says when, so
                // that it ends where that no longer holds (a list gone
                // through a thing at a time: `std/through`). Begun and not
                // waited for, or with nothing to end it, it never would.
                let ends = matches!(step.kind, StepKind::Call(_)) && step.when.is_some();
                if name == &self.blueprint.name && !ends {
                    let why = match step.kind {
                        StepKind::Call(_) => ": a step that calls its own text says `when`, which ends it",
                        _ => "",
                    };
                    self.error(
                        "recursion",
                        format!("`{name}` cannot {} itself{why}", step.kind.verb()),
                        step.span,
                    );
                }
                if !has_holes(name) {
                    self.blueprint_exists(name, step.span);
                }
                // What `as` says is the child's name, with nothing added.
                // One that does not tell the elements of a `for_each`
                // apart would be one child, applied over for each of them.
                if let (Some(_), Some(key)) = (&step.for_each, &step.key) {
                    let tells_apart = exprs(&Value::String(key.clone()))
                        .iter()
                        .flat_map(Expr::paths)
                        .any(|path| matches!(path[0].as_str(), "item" | "index"));
                    if !tells_apart {
                        self.error(
                            "same-name",
                            format!(
                                "`as: {key}` is the same for every element of `for_each`, so each would be \
                                 applied over the one before; put what tells them apart in it \
                                 (`${{ item }}`, `${{ index }}`), or leave `as` out and each is named \
                                 `{}#<position>`",
                                step.id
                            ),
                            step.span,
                        );
                    }
                }
                self.capabilities.entry("blueprints:run".to_string()).or_insert(step.span);
            }
            // What a task runs is its steps, and each of them is checked.
            StepKind::Task => {}
        }
    }

    fn blueprint_exists(&mut self, name: &str, span: Span) {
        if self.catalog.has_blueprint(name) == Some(false) {
            self.warning("unknown-blueprint", format!("no blueprint named `{name}` was found"), span);
        }
    }

    fn hook(&mut self, hook: &Hook, reads: &Reads<'_>, may_decide: bool) {
        if let Some(cond) = &hook.when {
            self.expr(&cond.expr, reads, hook.span);
        }
        self.reads(&hook.with, reads, hook.span);
        if DECISIONS.contains(&hook.action.as_str()) {
            if !may_decide {
                self.error(
                    "misplaced-decision",
                    format!("`{}` only makes sense in `on_fail` or `on_timeout`", hook.action),
                    hook.span,
                );
            }
            return;
        }
        self.action(&hook.action, &hook.with, hook.span);
    }

    fn action(&mut self, name: &str, with: &Value, span: Span) {
        let Some(spec) = self.spec(name) else {
            if let Some(why) = self.catalog.absent(name) {
                self.error("unknown-action", why, span);
            } else if self.checks_actions() {
                let names = self.catalog.action_names();
                let hint = did_you_mean(name, names.iter().copied())
                    .map(|close| format!("; did you mean `{close}`?"))
                    .unwrap_or_default();
                self.error("unknown-action", format!("there is no action called `{name}`{hint}"), span);
            }
            return;
        };
        if let Some(capability) = &spec.capability {
            self.capabilities.entry(capability.clone()).or_insert(span);
        }
        // `with: ${ ... }` is filled in at run time and checked then.
        let Some(given) = with.as_object() else { return };
        // What `...` hands on is known when the step runs, and is checked
        // then: what is written beside it is held to what the action takes,
        // and what it needs may be among what is handed on.
        let handed_on = given.contains_key(crate::template::HANDED_ON);
        for key in given.keys().filter(|key| *key != crate::template::HANDED_ON) {
            if !spec.open && !spec.args.iter().any(|arg| &arg.name == key) {
                let names: Vec<&str> = spec.args.iter().map(|arg| arg.name.as_str()).collect();
                let hint = match did_you_mean(key, names.iter().copied()) {
                    Some(close) => format!("did you mean `{close}`?"),
                    None if names.is_empty() => "it takes no arguments".to_string(),
                    None => format!("it takes: {}", names.join(", ")),
                };
                self.error("unknown-argument", format!("`{name}` has no argument `{key}`; {hint}"), span);
            }
        }
        for arg in &spec.args {
            match given.get(&arg.name) {
                None if arg.required && !handed_on => {
                    self.error(
                        "missing-argument",
                        format!("`{name}` needs `{}` ({})", arg.name, arg.help),
                        span,
                    );
                }
                Some(value) if !has_any_holes(value) && !value.is_null() && !arg.ty.accepts(value) => {
                    self.error(
                        "wrong-type",
                        format!("`{name}`: `{}` must be {}, got {value}", arg.name, arg.ty.name()),
                        span,
                    );
                }
                _ => {}
            }
        }
        if name == "emit" {
            self.topic(given.get("topic"), span);
        }
        // What `keep` keeps in, and `drop` takes out of, is a list the
        // text declares: said here, where it is written, and not when a
        // step first runs into it.
        let list = match name {
            "keep" => given.get("in"),
            "drop" => given.get("from"),
            _ => None,
        };
        if let Some(list) = list.and_then(Value::as_str).filter(|list| !has_holes(list)) {
            match self.blueprint.state.get(list) {
                None => self.error(
                    "unknown-state",
                    format!(
                        "`{name}` names the list `{list}`, and there is no state key `{list}`; declare it under \
                         `state` as a list, `{list}: []`"
                    ),
                    span,
                ),
                Some(begins) if !begins.is_array() => self.error(
                    "not-a-list",
                    format!(
                        "`{name}` names `{list}`, which begins as {begins} and is no list; declare it under \
                         `state` as a list, `{list}: []`"
                    ),
                    span,
                ),
                Some(_) => {}
            }
        }
        // What a `wait` says once it is listening is said as an `emit`
        // says it.
        if let ("wait", Some(say)) = (name, given.get("say")) {
            match say.get("topic") {
                Some(topic) => self.topic(Some(topic), span),
                None => self.error(
                    "bad-topic",
                    "`say` needs a `topic`: it is an event, said as `emit` says one".to_string(),
                    span,
                ),
            }
        }
    }

    fn topic(&mut self, topic: Option<&Value>, span: Span) {
        let Some(topic) = topic.and_then(Value::as_str).filter(|topic| !has_holes(topic)) else { return };
        if !is_topic(topic) {
            self.error(
                "bad-topic",
                format!("`{topic}` is not a topic; use lowercase words joined by dots, like `build.slow`"),
                span,
            );
        } else if let Some(reserved) =
            RESERVED_TOPICS.iter().find(|prefix| topic.split('.').next() == Some(**prefix))
        {
            self.error(
                "reserved-topic",
                format!("topics starting with `{reserved}.` are emitted by the system; pick another name"),
                span,
            );
        }
    }

    fn trigger(&mut self, trigger: &Trigger, heard: Option<&Heard>) {
        let reads = Reads { in_handler: true, heard, ..Reads::top() };
        if let Some(cond) = &trigger.when {
            self.expr(&cond.expr, &reads, trigger.span);
        }
        // What it takes turns by is worked out from the event, as its
        // guard is.
        if let Some(turn) = &trigger.turn {
            for expr in exprs(&Value::String(turn.clone())) {
                self.expr(&expr, &reads, trigger.span);
            }
        }
        // What an event is addressed by has to be known before there is an
        // event: a name that is not, is refused here with why, and the
        // rest are checked as they are anywhere.
        for expr in exprs(&Value::Object(trigger.address.clone())) {
            match expr.paths().iter().find_map(|path| address::refused(&path[0])) {
                Some(why) => self.error("misplaced-name", why, trigger.span),
                None => self.expr(&expr, &Reads::top(), trigger.span),
            }
        }
        match &trigger.source {
            TriggerSource::Event(pattern) => {
                if !is_pattern(pattern) {
                    self.error(
                        "bad-topic",
                        format!(
                            "`{pattern}` is not a topic pattern; use dotted words where `*` matches one word and `**` any number, like `board.item.*`"
                        ),
                        trigger.span,
                    );
                }
                // A handler does not hear what its own runs say. One that
                // listens for a group and answers within it is relying on
                // that; one that says the very topic it listens for is
                // most likely trying to wake itself, and will not.
                if !pattern.contains('*') && crate::events::said_in(&trigger.flow).contains(pattern) {
                    let name = &trigger.name;
                    self.warning(
                        "own-event",
                        format!(
                            "`{name}` says `{pattern}`, which is what it listens for, and a handler does not \
                             hear what its own runs say: saying it does not run `{name}` again. Have another \
                             handler listen for it, or run this one again with `every:`."
                        ),
                        trigger.span,
                    );
                }
            }
            TriggerSource::Poll { action, with, .. } => {
                self.reads(with, &Reads::top(), trigger.span);
                self.action(action, with, trigger.span);
            }
            TriggerSource::Every(_) | TriggerSource::At(_) => {}
        }
    }

    /// Checks every expression inside `value`.
    fn reads(&mut self, value: &Value, reads: &Reads<'_>, span: Span) {
        for expr in exprs(value) {
            self.expr(&expr, reads, span);
        }
    }

    fn expr(&mut self, expr: &Expr, reads: &Reads<'_>, span: Span) {
        for path in expr.paths() {
            let root = path[0].as_str();
            let name = path.get(1).map(String::as_str);
            let span = self.written(&path, span);
            match root {
                "params" => {
                    if let Some(name) = name.filter(|n| !self.blueprint.params.iter().any(|p| p.name == *n)) {
                        let known = self.blueprint.params.iter().map(|p| p.name.as_str());
                        let hint = did_you_mean(name, known)
                            .map(|close| format!("; did you mean `{close}`?"))
                            .unwrap_or_else(|| "; declare it under `params`".to_string());
                        self.error("unknown-param", format!("there is no param `{name}`{hint}"), span);
                    }
                }
                "state" => {
                    if let Some(name) = name.filter(|n| !self.blueprint.state.contains_key(*n)) {
                        let hint = self.close_state(name);
                        self.error("unknown-state", format!("there is no state key `{name}`{hint}"), span);
                    }
                }
                "steps" => {
                    self.step_read(name, reads, span);
                    if let (Some(step), Some(member)) = (name, path.get(2)) {
                        let written = format!("steps.{step}.{member}");
                        if self.member(&written, "a step", STEP_VIEW, span) && member == "out" {
                            self.gives(step, &format!("{written}."), path.get(3), reads, span);
                        }
                    }
                }
                "self" if reads.own.is_none() => {
                    self.error(
                        "misplaced-name",
                        "`self` is only available in a step's hooks and watch rules",
                        span,
                    );
                }
                "self" => {
                    if let (Some(step), Some(member)) = (reads.own, name)
                        && self.member(&format!("self.{member}"), "a step", STEP_VIEW, span)
                        && member == "out"
                    {
                        self.gives(step, "self.out.", path.get(2), reads, span);
                    }
                }
                "event" if !reads.in_handler => {
                    self.error("misplaced-name", "`event` is only available inside an `on` handler", span);
                }
                "event" => {
                    // What a poll read the time before is its event's alone.
                    if let Some(member) = name.filter(|member| !(reads.polled && *member == "previous"))
                        && self.member(&format!("event.{member}"), "an event", EVENT_VIEW, span)
                        && let ("source", Some(part)) = (member, path.get(2))
                    {
                        self.member(&format!("event.source.{part}"), "an event's source", SOURCE_VIEW, span);
                    }
                    if let (Some(heard), Some("data"), Some(field)) = (reads.heard, name, path.get(2)) {
                        self.carried(heard, field, Where::Read, span);
                    }
                }
                "instance" | "target" => {
                    let (what, has) = match root {
                        "instance" => ("an instance", INSTANCE_VIEW),
                        _ => ("what this is attached to", TARGET_VIEW),
                    };
                    if let Some(member) = name {
                        self.member(&format!("{root}.{member}"), what, has, span);
                    }
                }
                "run" if !reads.of_run => {
                    self.error(
                        "misplaced-name",
                        "`run` is only available in the blueprint's own `on_success`, `on_fail` and `on_finish`: it is \
                         the run of its steps, once they have ended",
                        span,
                    );
                }
                "run" => {
                    if let Some(member) = name {
                        self.member(&format!("run.{member}"), "a run", RUN_VIEW, span);
                    }
                }
                "was" if !reads.in_update => {
                    self.error(
                        "misplaced-name",
                        "`was` is only available in `on_update`: it is what a running instance was before it \
                         took this text, and there is no such thing anywhere else",
                        span,
                    );
                }
                "was" => {
                    let has = crate::vocabulary::WAS_VIEW.iter().map(|member| member.name);
                    if let Some(name) = name.filter(|name| !has.clone().any(|known| known == *name)) {
                        let has: Vec<String> = has.map(|known| format!("`was.{known}`")).collect();
                        self.error(
                            "unknown-member",
                            format!("`was.{name}` is not something `was` has; there are {}", has.join(", ")),
                            span,
                        );
                    }
                }
                "item" | "index" if !reads.in_loop => {
                    self.error(
                        "misplaced-name",
                        format!("`{root}` is only available in a step with `for_each`"),
                        span,
                    );
                }
                "environment" | "profile" => {
                    // Checked, unlike what follows `instance`: either may come to
                    // hold more, and a blueprint written against what it does not
                    // hold yet should be told so before it runs. What follows `vars`
                    // is not: those names are somebody's own, and one not set is nothing.
                    let has = crate::vocabulary::members(root).iter().map(|member| member.name);
                    if let Some(name) = name.filter(|name| !has.clone().any(|known| known == *name)) {
                        let has: Vec<String> = has.map(|known| format!("`{known}`")).collect();
                        let one = if root == "environment" { "an environment" } else { "a profile" };
                        self.error(
                            "unknown-member",
                            format!(
                                "`{root}.{name}` is not something {one} has yet; it has {}",
                                has.join(", ")
                            ),
                            span,
                        );
                    }
                }
                _ if ROOTS.iter().any(|known| known.name == root) => {}
                _ => {
                    let hint = did_you_mean(root, ROOTS.iter().map(|known| known.name))
                        .map(|close| format!("did you mean `{close}`?"))
                        .unwrap_or_else(|| {
                            let roots: Vec<&str> = ROOTS.iter().map(|known| known.name).collect();
                            format!("expressions start with one of: {}", roots.join(", "))
                        });
                    self.error("unknown-name", format!("`{root}` is not defined; {hint}"), span);
                }
            }
        }
    }

    /// Holds the last name of `written` to what `what` has: one it has
    /// not got reads as nothing, and was said to be right. True when it
    /// has it.
    fn member(&mut self, written: &str, what: &str, has: &[Choice], span: Span) -> bool {
        let given = written.rsplit('.').next().unwrap_or_default();
        if has.iter().any(|known| known.name == given) {
            return true;
        }
        let hint = match did_you_mean(given, has.iter().map(|known| known.name)) {
            Some(close) => format!("; did you mean `{close}`?"),
            None => ".".to_string(),
        };
        let all: Vec<String> = has.iter().map(|known| format!("`{}`", known.name)).collect();
        let mut subject = what.to_string();
        if let Some(first) = subject.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        self.error(
            "unknown-member",
            format!("`{written}` is not something {what} has{hint} {subject} has {}", all.join(", ")),
            span,
        );
        false
    }

    /// Holds what is read of what a step gave (`steps.<id>.out.<name>`) to
    /// what the step is. Of a task, a step it has not got is refused: its
    /// steps are written there. Of an action, a name it does not say it
    /// gives is warned of and no more, since one may give more than it
    /// says. Nothing is said where what a step gives is a list (it is done
    /// once for each of one), or is not said of its action at all.
    fn gives(&mut self, step: &str, before: &str, name: Option<&String>, reads: &Reads<'_>, span: Span) {
        let (Some(name), Some(step)) = (name, reads.graph.and_then(|graph| graph.step(step))) else { return };
        if step.for_each.is_some() {
            return;
        }
        let said = |known: Vec<&str>| {
            let hint = match did_you_mean(name, known.iter().copied()) {
                Some(close) => format!("; did you mean `{close}`?"),
                None => ".".to_string(),
            };
            let all: Vec<String> = known.iter().map(|known| format!("`{known}`")).collect();
            (hint, all.join(", "))
        };
        match &step.kind {
            StepKind::Do(action) => {
                let Some(spec) = self.spec(action) else { return };
                let gives: Vec<&str> = spec.outputs.iter().map(|(name, _)| name.as_str()).collect();
                if gives.is_empty() || gives.contains(&name.as_str()) {
                    return;
                }
                let (hint, all) = said(gives);
                self.warning(
                    "unknown-output",
                    format!(
                        "`{before}{name}`: `{action}` does not say it gives `{name}`{hint} It gives {all}"
                    ),
                    span,
                );
            }
            StepKind::Task => {
                let inside: Vec<&str> = step.inner.steps.iter().map(|inner| inner.id.as_str()).collect();
                if inside.contains(&name.as_str()) {
                    return;
                }
                let (hint, all) = said(inside);
                self.error(
                    "unknown-output",
                    format!(
                        "`{before}{name}`: the task `{}` has no step `{name}`{hint} It has {all}",
                        step.id
                    ),
                    span,
                );
            }
            StepKind::Call(_) | StepKind::Spawn(_) => {}
        }
    }

    fn step_read(&mut self, name: Option<&str>, reads: &Reads<'_>, span: Span) {
        let Some(name) = name else { return };
        if reads.own == Some(name) {
            return;
        }
        let Some(visible) = reads.visible else {
            self.error("misplaced-name", "step results are not available here", span);
            return;
        };
        if visible.contains(name) {
            return;
        }
        let beside = reads.beside.is_some_and(|beside| beside.contains(name));
        let outside = reads.outside.and_then(|outside| outside.get(name)).filter(|_| !beside);
        let (exists, held) = self.blueprint.flows().fold((false, None), |found, (_, flow)| {
            let (exists, held) = find_step(flow, name);
            (found.0 || exists, found.1.or(held))
        });
        let message = match (outside, held, reads.own) {
            (Some(task), _, _) if *task == name => format!(
                "`steps.{name}` is the task this is inside, and it has not finished; the steps of a task \
                 read each other by their own names, as `steps.<name>`"
            ),
            (Some(task), _, _) => format!(
                "`steps.{name}` may not have finished when the task `{task}` starts; add `{name}` to the \
                 `needs` of `{task}`"
            ),
            // Only what a task gave out can be seen from outside it.
            (None, Some(task), _) if !beside => format!(
                "`{name}` is a step inside the task `{task}`; outside the task, what it gave out is \
                 `steps.{task}.out.{name}`"
            ),
            (None, None, _) if !exists => format!("there is no step `{name}`{}", self.close_step(name)),
            (_, _, Some(own)) => format!(
                "`steps.{name}` may not have finished when this runs; add `{name}` to the `needs` of `{own}`"
            ),
            (_, _, None) => format!(
                "`steps.{name}` may not have finished when this runs; add `{name}` to this step's `needs`"
            ),
        };
        self.error("unordered-read", message, span);
    }

    fn permissions(&mut self) {
        let needed = std::mem::take(&mut self.capabilities);
        match &self.blueprint.permissions {
            Some(granted) => {
                for (name, span) in needed {
                    if !granted.iter().any(|g| grants(g, &name)) {
                        self.error(
                            "missing-permission",
                            format!("this uses `{name}`, which is not listed under `permissions`"),
                            span,
                        );
                    }
                }
            }
            None if !needed.is_empty() => {
                let names: Vec<&str> = needed.keys().map(String::as_str).collect();
                self.warning(
                    "no-permissions",
                    format!(
                        "this blueprint declares no `permissions`, so it runs with everything its caller has. \
                         It needs: [{}]",
                        names.join(", ")
                    ),
                    self.blueprint.span,
                );
            }
            None => {}
        }
    }
}

/// Whether holding `granted` allows `needed`. A grant is `area` or
/// `area:verb`; the bare area covers every verb in it.
pub fn grants(granted: &str, needed: &str) -> bool {
    granted == "*" || granted == needed || needed.split(':').next() == Some(granted)
}

/// Every expression in the `${ }` holes of `value`.
fn exprs(value: &Value) -> Vec<Expr> {
    let mut exprs = Vec::new();
    each_template(value, &mut |text| {
        if let Ok(template) = Template::parse(text) {
            exprs.extend(template.exprs().cloned());
        }
    });
    exprs
}

fn has_any_holes(value: &Value) -> bool {
    let mut found = false;
    each_template(value, &mut |_| found = true);
    found
}

/// `build`, `build.slow`, `lane.42.ready`.
pub fn is_topic(text: &str) -> bool {
    !text.is_empty()
        && text.split('.').all(|word| {
            !word.is_empty()
                && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
        })
}

pub(crate) fn is_pattern(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(|word| word == "*" || word == "**" || is_topic(word))
}

/// Whether `name` is a step of `flow` itself, and the task that holds a step
/// of that name, if one in `flow` does.
fn find_step<'a>(flow: &'a Flow, name: &str) -> (bool, Option<&'a str>) {
    let task = flow.steps.iter().find_map(|step| match find_step(&step.inner, name) {
        (true, _) => Some(step.id.as_str()),
        (false, deeper) => deeper,
    });
    (flow.step(name).is_some(), task)
}

/// For each step, every step that must settle before it.
pub fn ancestors(flow: &Flow) -> BTreeMap<&str, BTreeSet<&str>> {
    fn visit<'a>(flow: &'a Flow, id: &'a str, out: &mut BTreeSet<&'a str>) {
        let Some(step) = flow.step(id) else { return };
        for need in &step.needs {
            if out.insert(need.as_str()) {
                visit(flow, need, out);
            }
        }
    }
    flow.steps
        .iter()
        .map(|step| {
            let mut found = BTreeSet::new();
            visit(flow, &step.id, &mut found);
            (step.id.as_str(), found)
        })
        .collect()
}

/// One cycle through `needs`, if there is any.
fn find_cycle(flow: &Flow) -> Option<Vec<String>> {
    fn visit<'a>(
        flow: &'a Flow,
        id: &'a str,
        path: &mut Vec<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> Option<Vec<String>> {
        if let Some(at) = path.iter().position(|seen| *seen == id) {
            let mut cycle: Vec<String> = path[at..].iter().map(|s| s.to_string()).collect();
            cycle.push(id.to_string());
            return Some(cycle);
        }
        if !done.insert(id) {
            return None;
        }
        path.push(id);
        let step = flow.step(id)?;
        for need in &step.needs {
            if flow.step(need).is_some()
                && let Some(cycle) = visit(flow, need, path, done)
            {
                return Some(cycle);
            }
        }
        path.pop();
        None
    }
    let mut done = BTreeSet::new();
    flow.steps.iter().find_map(|step| visit(flow, &step.id, &mut Vec::new(), &mut done))
}
