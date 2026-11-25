//! What a running text comes to that is worked out from the text and
//! plain values alone, with no engine: how a step looks to the
//! expressions after it, when the rules that watch it are timed, how long
//! a try waits before the next, whether its action may be used, whether
//! somebody may apply a text over one that runs, which of a text could be
//! owed what is said on a topic, and which records of a step are held
//! until its run next writes. The runner asks each where it runs a text
//! (`conductor`, under the names it always gave), and keeps nothing here
//! but a count: how many texts in force could be owed each topic the
//! runner says of its own steps and runs ([`Owed`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU16, Ordering};

use blueprint::{
    ActionSpec, Blueprint, Flow, Hook, Retry, Step, StepKind, TriggerSource, duration, template,
};
use kernel::event::topic_matches_any;
use kernel::{Error, Fields, Principal, Result};
use runs::{InstanceInfo, StepStatus};
use serde_json::{Value, json};

/// How a step appears to expressions, as `steps.<id>` and `self`.
pub fn view(
    status: StepStatus,
    output: &Value,
    error: Option<&str>,
    attempts: u32,
    duration_ms: i64,
) -> Value {
    json!({
        "status": status.as_str(),
        "ok": status == StepStatus::Ok,
        "failed": status == StepStatus::Failed,
        "skipped": status == StepStatus::Skipped,
        "out": output,
        // Said here so that a text asks how many a loop went over by name,
        // and calls nothing to count them.
        "count": output.as_array().map(Vec::len),
        "error": error,
        "reason": error.map(kernel::why::reason),
        "attempts": attempts,
        "duration_ms": duration_ms,
    })
}

/// `scope` with `view` as the step `step` and as `self`.
pub fn with_self(scope: &Fields, step: &str, view: Value) -> Fields {
    let mut scope = scope.clone();
    // A step inside a task is `build.compile` to the run, and `compile` to
    // what is written beside it.
    let name = step.rsplit('.').next().unwrap_or(step);
    if let Some(Value::Object(steps)) = scope.get_mut("steps") {
        steps.insert(name.to_string(), view.clone());
    }
    scope.insert("self".into(), view);
    scope
}

/// How long a step waits before its try after `tries`.
pub fn backoff(retry: &Retry, tries: u32) -> u64 {
    let scaled = retry.backoff_ms as f64 * retry.factor.powi(tries.saturating_sub(1) as i32);
    (scaled as u64).min(retry.max_backoff_ms)
}

/// A length of time as written, its holes filled from `scope`.
pub fn duration_in(source: &str, scope: &Fields) -> std::result::Result<u64, String> {
    let rendered = template::render(&Value::String(source.to_string()), scope)?;
    duration::parse(rendered.as_str().ok_or("expected a duration such as `30s`")?)
}

/// When each rule that watches `step` is timed, in milliseconds: as its
/// `after` or `every` is written, or worked out as the step begins where
/// it has holes. None for a rule of what the step prints, and for one
/// whose time comes to nothing, which is never timed.
pub fn timed(step: &Step, scope: &Fields) -> std::result::Result<Vec<Option<u64>>, String> {
    let time = |rule: &blueprint::Watch| {
        let Some(timing) = rule.timing() else { return Ok(None) };
        if let Some(fixed) = timing.fixed {
            return Ok(Some(fixed));
        }
        let wrong = |why: String| format!("a watch rule's time, `{}`: {why}", timing.source);
        match template::render(&Value::String(timing.source.clone()), scope).map_err(wrong)? {
            Value::Null => Ok(None),
            Value::String(said) if said.trim().is_empty() => Ok(None),
            Value::String(said) => match duration::parse(&said).map_err(wrong)? {
                0 => Err(wrong("a rule is timed by more than nothing".to_string())),
                millis => Ok(Some(millis)),
            },
            other => Err(wrong(format!("`{other}` is no time; write it like `30s` or `1h30m`"))),
        }
    };
    step.watch.iter().map(time).collect()
}

/// Why `blueprint`, run by `principal`, may not use `capability`, if it
/// may not: the blueprint does not list it, or whoever runs it lacks it.
pub fn permit(
    blueprint: &Blueprint,
    principal: &Principal,
    capability: &str,
) -> std::result::Result<(), String> {
    if let Some(declared) = &blueprint.permissions
        && !declared.iter().any(|granted| blueprint::grants(granted, capability))
    {
        return Err(format!("`{}` does not list `{capability}` under `permissions`", blueprint.name));
    }
    if !principal.can(capability) {
        return Err(format!("{} may not use `{capability}`", principal.name));
    }
    Ok(())
}

/// Whether two bearers are the same one holding the same things. A name is
/// not authority: someone renamed has not been given or lost anything.
fn same_authority(one: &Principal, other: &Principal) -> bool {
    let held = |principal: &Principal| {
        let mut scopes = principal.scopes.clone();
        scopes.sort();
        scopes.dedup();
        scopes
    };
    one.kind == other.kind && one.id == other.id && held(one) == held(other)
}

/// Refuses an apply over `existing` by someone who does not hold what the
/// blueprint says it needs.
///
/// A step that needs what its instance's bearer lacks fails when it runs,
/// and that is what holds the rule. This is said earlier, before anything
/// is changed, because here there is something working to lose: the
/// instance would stop what it is doing, start on text that cannot do what
/// it lists, and have to be put back. Someone applying again what already
/// runs as them is not asked: nothing about its authority would change.
pub fn may_apply_over(existing: &InstanceInfo, blueprint: &Blueprint, by: &Principal) -> Result<()> {
    if same_authority(&existing.principal, by) {
        return Ok(());
    }
    let lacking: Vec<String> = blueprint
        .permissions
        .iter()
        .flatten()
        .filter(|listed| !by.can(listed))
        .map(|listed| format!("`{listed}`"))
        .collect();
    if lacking.is_empty() {
        return Ok(());
    }
    let lacking = lacking.join(", ");
    Err(Error::denied(format!(
        "{name} may not use {lacking}, which `{blueprint}` lists under `permissions`, so {name} cannot \
         apply it over `{key}`, which is running, and nothing was changed. \
         Someone who holds {lacking} can apply it.",
        name = by.name,
        blueprint = blueprint.name,
        key = existing.key,
    )))
}

/// Whether an instance running `blueprint` could come to be owed what is
/// said on `topic`: one of its handlers hears it, or a `wait` anywhere in
/// it (a step, a step of a task, a hook) is for it.
pub fn hears(blueprint: &Blueprint, topic: &str) -> bool {
    let (handlers, waits) = heard_by(blueprint, topic);
    handlers || waits
}

/// Which of what an instance running `blueprint` does could come to be
/// owed what is said on `topic`: one of its handlers, which keeps its
/// place among the events where it is said, and a `wait` anywhere in it,
/// which keeps its own with its step.
pub fn heard_by(blueprint: &Blueprint, topic: &str) -> (bool, bool) {
    let named = |pattern: &str| pattern.contains("${") || topic_matches_any(pattern, topic);
    let waits = |action: &str, with: &Value| {
        action == "wait" && with.get("event").and_then(Value::as_str).is_some_and(named)
    };
    let hook = |hook: &Hook| waits(&hook.action, &hook.with);
    fn within(flow: &Flow, waits: &dyn Fn(&str, &Value) -> bool) -> bool {
        flow.steps.iter().any(|step| {
            let hooks = [&step.on_start, &step.on_success, &step.on_fail, &step.on_timeout, &step.on_finish];
            matches!(&step.kind, StepKind::Do(action) if waits(action, &step.with))
                || hooks.into_iter().flatten().any(|hook| waits(&hook.action, &hook.with))
                || step.watch.iter().any(|watch| waits(&watch.hook.action, &watch.hook.with))
                || within(&step.inner, waits)
        })
    }
    let own = [&blueprint.on_success, &blueprint.on_fail, &blueprint.on_finish, &blueprint.on_stop];
    let handlers = blueprint
        .triggers
        .iter()
        .any(|trigger| matches!(&trigger.source, TriggerSource::Event(pattern) if named(pattern)));
    let waited = blueprint.triggers.iter().any(|trigger| within(&trigger.flow, &waits))
        || within(&blueprint.flow, &waits)
        || own.into_iter().flatten().any(hook)
        || blueprint.on_update.iter().flatten().any(hook);
    (handlers, waited)
}

/// Which of a step's records are held until its run next writes, by what
/// its action says of itself, are what `runs` holds them by
/// ([`runs::held`]): here under the names they had.
pub use runs::held::{Quiet, holds, own};

/// Why `args`, as a step filled them in, do not fit `spec`, if they do
/// not. Checking a text finds what is written out; this finds what a
/// `${ }` came to.
pub fn checked(spec: &ActionSpec, args: &Fields) -> std::result::Result<(), String> {
    for arg in &spec.args {
        match args.get(&arg.name) {
            None | Some(Value::Null) if arg.required => {
                return Err(format!("`{}` needs `{}` ({})", spec.name, arg.name, arg.help));
            }
            Some(value) if !value.is_null() && !arg.ty.accepts(value) => {
                return Err(format!(
                    "`{}`: `{}` must be {}, got {value}",
                    spec.name,
                    arg.name,
                    arg.ty.name()
                ));
            }
            _ => {}
        }
    }
    if !spec.open
        && let Some(extra) = args.keys().find(|key| !spec.args.iter().any(|arg| &arg.name == *key))
    {
        return Err(format!("`{}` has no argument `{extra}`", spec.name));
    }
    Ok(())
}

/// How many instances there are whose text could be owed each of the
/// topics the runner says of its own steps and runs (`conductor::PASSED_ON`,
/// which it is made with), and whether everything is being taken up
/// again. It came out of the runner, which offers it under the names it
/// always gave (`crate::owed` there) and says what it keeps by it.
pub struct Owed {
    /// The topics, as the runner says them.
    own: &'static [&'static str],
    hearing: Vec<AtomicIsize>,
    /// How many conductors are taking everything up again: each
    /// project's has a start of its own, and they share this.
    resuming: AtomicIsize,
    /// Everything is kept, whatever hears it: where what is said is read
    /// back as what a person is shown (the runner's `keeps_all_it_says`).
    all: AtomicBool,
}

impl Owed {
    /// Counts nothing yet of `own`, the topics the runner says of its own
    /// steps and runs: sixteen at most.
    pub fn new(own: &'static [&'static str]) -> Owed {
        let hearing = own.iter().map(|_| AtomicIsize::new(0)).collect();
        Owed { own, hearing, resuming: AtomicIsize::new(0), all: AtomicBool::new(false) }
    }

    /// Whether what is said on `topic` is kept as it is said: any topic
    /// that is not one of its own is.
    pub fn keeps(&self, topic: &str) -> bool {
        let Some(at) = self.own.iter().position(|own| *own == topic) else { return true };
        self.all.load(Ordering::SeqCst)
            || self.resuming.load(Ordering::SeqCst) > 0
            || self.hearing[at].load(Ordering::SeqCst) > 0
    }

    /// Everything is kept from now on.
    pub fn keep_all(&self) {
        self.all.store(true, Ordering::SeqCst);
    }

    /// Whether everything is being taken up again, by any conductor.
    pub fn is_resuming(&self) -> bool {
        self.resuming.load(Ordering::SeqCst) > 0
    }

    /// Everything is being taken up again, until what is given back is
    /// dropped.
    pub fn resuming(self: &Arc<Self>) -> Resuming {
        self.resuming.fetch_add(1, Ordering::SeqCst);
        Resuming(self.clone())
    }

    /// Has what is said on each of its own topics that `named` says yes
    /// to kept for as long as what is given back is held ([`Keeping`]).
    pub fn keeping(self: &Arc<Self>, named: impl Fn(&str) -> bool) -> Keeping {
        let bits = self
            .own
            .iter()
            .enumerate()
            .filter(|(_, own)| named(own))
            .fold(0u16, |bits, (at, _)| bits | 1 << at);
        for at in (0..self.own.len()).filter(|at| bits >> at & 1 == 1) {
            self.hearing[at].fetch_add(1, Ordering::SeqCst);
        }
        Keeping { of: self.clone(), bits }
    }
}

pub struct Resuming(Arc<Owed>);

impl Drop for Resuming {
    fn drop(&mut self) {
        self.0.resuming.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One instance's part in that: which of the runner's own topics the
/// text it runs could be owed. Counted from when the instance is there
/// until nothing holds it any longer, which is no sooner than it is
/// over: counted a moment too long, an event is kept that nobody reads;
/// never the other way.
pub struct Hearing {
    of: Arc<Owed>,
    /// A bit for each of the topics.
    hears: AtomicU16,
}

impl Hearing {
    pub fn of(owed: &Arc<Owed>, blueprint: &Blueprint) -> Hearing {
        let hearing = Hearing { of: owed.clone(), hears: AtomicU16::new(0) };
        hearing.hear(blueprint);
        hearing
    }

    /// The instance runs `blueprint` from now.
    pub fn hear(&self, blueprint: &Blueprint) {
        let own = self.of.own;
        let now = own
            .iter()
            .enumerate()
            .filter(|(_, topic)| hears(blueprint, topic))
            .fold(0u16, |bits, (at, _)| bits | 1 << at);
        let was = self.hears.swap(now, Ordering::SeqCst);
        for at in 0..own.len() {
            let (now, was) = (now >> at & 1, was >> at & 1);
            self.of.hearing[at].fetch_add(now as isize - was as isize, Ordering::SeqCst);
        }
    }
}

impl Drop for Hearing {
    fn drop(&mut self) {
        let was = self.hears.load(Ordering::SeqCst);
        for at in (0..self.of.own.len()).filter(|at| was >> at & 1 == 1) {
            self.of.hearing[at].fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// What has everything said on some of the runner's own topics kept, for
/// as long as it is held, whatever text there is: whoever counts what is
/// said from a place it keeps, and reads it again from there (the
/// runner's `keeps_said`).
pub struct Keeping {
    of: Arc<Owed>,
    /// A bit for each of the topics.
    bits: u16,
}

impl Drop for Keeping {
    fn drop(&mut self) {
        for at in (0..self.of.own.len()).filter(|at| self.bits >> at & 1 == 1) {
            self.of.hearing[at].fetch_sub(1, Ordering::SeqCst);
        }
    }
}
