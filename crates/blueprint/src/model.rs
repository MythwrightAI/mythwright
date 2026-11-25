//! A blueprint as the executor sees it.
//!
//! The shape in one paragraph: a blueprint has `steps` (a graph that runs once
//! when an instance starts) and `on` handlers (smaller graphs that run each
//! time something happens while the instance is alive). A step does one thing:
//! an action, or another blueprint. Around that sit the lifecycle behaviours
//! every step gets for free: a guard, a timeout, retries, watchers that fire
//! while it runs, and hooks for when it starts, finishes or fails. A task is
//! a step whose one thing is a graph of steps of its own, so that those
//! behaviours are said once for all of them.

use serde_json::{Map, Value};

use crate::syntax::Span;
use crate::template::Cond;
use crate::types::ValueType;

#[derive(Clone, Debug)]
pub struct Blueprint {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub params: Vec<Param>,
    /// Durable properties of an instance, with their initial values.
    pub state: Map<String, Value>,
    /// Capabilities the blueprint may use. `None` means it did not say.
    pub permissions: Option<Vec<String>>,
    pub env: Map<String, Value>,
    /// Other blueprints attached to every instance of this one.
    pub uses: Vec<Attachment>,
    pub flow: Flow,
    pub triggers: Vec<Trigger>,
    pub outputs: Map<String, Value>,
    /// Done when the steps went well.
    pub on_success: Vec<Hook>,
    pub on_fail: Vec<Hook>,
    /// Done after either, when the steps have ended.
    pub on_finish: Vec<Hook>,
    pub on_stop: Vec<Hook>,
    /// What a running instance does when it takes this text, in place of
    /// running the steps again. `None` for a text that does not say: its
    /// steps are run again. Written with nothing under it, there is
    /// nothing to do and the steps are still not run.
    pub on_update: Option<Vec<Hook>>,
    pub max_parallel: Option<u32>,
    /// Its handlers take turns: one run of any of them at a time.
    pub one_at_a_time: bool,
    /// What an instance of it is doing is written down as it goes, so that
    /// it is picked up after a restart. True unless the text says not.
    pub durable: bool,
    pub span: Span,
}

/// A graph of steps, in file order.
#[derive(Clone, Debug, Default)]
pub struct Flow {
    pub steps: Vec<Step>,
}

impl Flow {
    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|step| step.id == id)
    }
}

#[derive(Clone, Debug)]
pub struct Step {
    pub id: String,
    pub span: Span,
    pub kind: StepKind,
    /// Arguments; strings may contain `${ }` holes.
    pub with: Value,
    /// Steps that must settle first. Filled in with the previous step when the
    /// file did not say, so a plain list of steps runs top to bottom.
    pub needs: Vec<String>,
    pub needs_implicit: bool,
    pub when: Option<Cond>,
    pub timeout: Option<Timing>,
    pub retry: Option<Retry>,
    pub allow_failure: bool,
    /// Done again from its start where a restart came in the middle of it
    /// (`again: true`), as the step's own word for what its action does
    /// not say of itself: that it is the same done twice.
    pub again: bool,
    pub watch: Vec<Watch>,
    pub on_start: Vec<Hook>,
    /// Done when the step went well.
    pub on_success: Vec<Hook>,
    pub on_fail: Vec<Hook>,
    pub on_timeout: Vec<Hook>,
    /// Done after any of those, when the step has ended.
    pub on_finish: Vec<Hook>,
    /// Run on another device, through the gateway.
    pub device: Option<String>,
    /// Name under which this step's duration and outcome are reported.
    pub measure: Option<String>,
    /// For `call`, `spawn` and a task: what everything measured under the
    /// step also carries, by name. The values may have `${ }` holes, filled
    /// when the step starts (see [`crate::labels`]).
    pub labels: Map<String, Value>,
    /// For `call`/`spawn`: the child's key within its parent. Re-applying a
    /// blueprint reuses the child with the same key.
    pub key: Option<String>,
    /// For `call`/`spawn`: one of the project's environments the child is
    /// started under, where the instance the step runs in is under none
    /// (`environment:`). Its name may have `${ }` holes.
    pub environment: Option<String>,
    pub for_each: Option<Cond>,
    pub description: Option<String>,
    pub env: Map<String, Value>,
    /// The agent that is asked to put the step right when it fails
    /// (`if_fail_ask_agent`). The asking itself is the last of `on_fail`.
    pub asks: Option<String>,
    /// What the step fails with when nobody put it right (`else`): a name
    /// for the failure that whatever comes after can tell it by.
    pub fail_code: Option<String>,
    /// For a task: the steps it groups. Empty for every other step.
    pub inner: Flow,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    /// Run an action from the catalog.
    Do(String),
    /// Run another blueprint and wait for its steps to finish.
    Call(String),
    /// Start another blueprint and carry on.
    Spawn(String),
    /// Run the steps in [`Step::inner`] as one.
    Task,
}

impl StepKind {
    pub fn target(&self) -> &str {
        match self {
            StepKind::Do(name) | StepKind::Call(name) | StepKind::Spawn(name) => name,
            // A task runs nothing of its own; this is what it is counted as.
            StepKind::Task => "task",
        }
    }

    pub fn verb(&self) -> &'static str {
        match self {
            StepKind::Do(_) => "do",
            StepKind::Call(_) => "call",
            StepKind::Spawn(_) => "spawn",
            StepKind::Task => "task",
        }
    }
}

/// How a step inside a task is named, everywhere it is named: in a run's
/// record of it, in the events that say how it went, and in the graph the
/// client draws, which is matched to the run by these names. It is the
/// task's name, a dot, and the step's own: `build.compile`, and
/// `release.build.compile` where a task is inside another.
///
/// This is the one place that is decided. Whatever writes such a name or
/// takes one apart does it through here, so that what a blueprint is drawn
/// as and what the conductor says it ran cannot come to differ.
pub mod within {
    /// What goes before the names of the steps of the task called `task`,
    /// itself named in full: `build.`.
    pub fn task(task: &str) -> String {
        format!("{task}.")
    }

    /// The full name of `step`, which is directly under `before`. With
    /// nothing before it, a step of the blueprint itself, that is its own.
    pub fn name(before: &str, step: &str) -> String {
        format!("{before}{step}")
    }

    /// The own name of the step called `full`, if it is directly under
    /// `before`: not one that is somewhere else, and not one further in,
    /// which is some task's to answer for.
    pub fn own<'a>(before: &str, full: &'a str) -> Option<&'a str> {
        full.strip_prefix(before).filter(|own| !own.contains('.'))
    }
}

/// A duration that is either fixed or computed from params.
#[derive(Clone, Debug, PartialEq)]
pub struct Timing {
    pub source: String,
    /// Milliseconds, when the source has no holes.
    pub fixed: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Retry {
    /// Attempts after the first.
    pub max: u32,
    pub backoff_ms: u64,
    pub factor: f64,
    pub max_backoff_ms: u64,
}

/// Something to do at a lifecycle point.
#[derive(Clone, Debug)]
pub struct Hook {
    pub action: String,
    pub with: Value,
    pub when: Option<Cond>,
    pub span: Span,
}

/// A rule checked while a step is running.
#[derive(Clone, Debug)]
pub struct Watch {
    pub cond: WatchCond,
    pub hook: Hook,
    /// Abort the step when the rule fires.
    pub fail: bool,
}

impl Watch {
    /// When it is timed, where it is: as written, worked out as the step
    /// begins where it has holes. None for a rule of what the step prints.
    pub fn timing(&self) -> Option<&Timing> {
        match &self.cond {
            WatchCond::After(timing) | WatchCond::Every(timing) => Some(timing),
            WatchCond::Log(_) => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WatchCond {
    /// Fires once, when the step has been running this long.
    After(Timing),
    /// Fires repeatedly at this interval.
    Every(Timing),
    /// Fires when a line of the step's output matches this pattern.
    Log(String),
}

#[derive(Clone, Debug)]
pub struct Trigger {
    pub name: String,
    pub source: TriggerSource,
    /// `for:`: fields of an event's data and the value each must have for
    /// the event to be this handler's. The values may have `${ }` holes,
    /// filled once when the handler starts to listen (see
    /// [`crate::address`]). Empty when everything on the topic is its.
    pub address: Map<String, Value>,
    pub when: Option<Cond>,
    pub scope: TriggerScope,
    /// Collapse a burst of events into one run after this quiet period.
    pub debounce_ms: Option<u64>,
    /// `outside: true`: its event is said from outside (a person, a key, a
    /// caller), so no blueprint need say it and what it carries is not
    /// known here.
    pub outside: bool,
    /// `turn:`: what an event that comes to this handler takes its turn by,
    /// as text with `${ }` holes filled from the event. Events that come
    /// to the same key, at any handler of the instance that says `turn`,
    /// are handled one at a time, and those of one handler in order;
    /// others beside them. None where the handler handles one event at a
    /// time, whatever it is of.
    pub turn: Option<String>,
    pub flow: Flow,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum TriggerSource {
    /// An event whose topic matches this pattern (`*` is one segment, `**`
    /// is any number).
    Event(String),
    /// A timer.
    Every(u64),
    /// A time of day with its zone (`at:`, [`crate::at`]), as written: it
    /// may say `${ }` holes, filled from `params` when the handler starts.
    At(String),
    /// Something outside the system that cannot push: run an action on a
    /// timer and fire when its result changes.
    Poll { every_ms: u64, action: String, with: Value },
}

/// Which events a trigger sees.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerScope {
    /// What is its own project's and what is nobody's, the factory's own:
    /// what a handler hears where it says nothing.
    Project,
    /// Anything said in the factory, whichever project's it is.
    Fleet,
    /// Only events from this instance and its children.
    Own,
    /// Only events from the instance this one is attached to.
    Target,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: ValueType,
    pub default: Option<Value>,
    pub required: bool,
    pub description: Option<String>,
    pub choices: Vec<Value>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub key: String,
    pub blueprint: String,
    pub with: Value,
    pub span: Span,
}

impl Blueprint {
    /// Every flow in the blueprint with a label: the main one, then one per
    /// trigger.
    pub fn flows(&self) -> impl Iterator<Item = (Option<&Trigger>, &Flow)> {
        std::iter::once((None, &self.flow))
            .chain(self.triggers.iter().map(|trigger| (Some(trigger), &trigger.flow)))
    }

    /// Whether an instance stays alive after its steps finish.
    pub fn is_long_lived(&self) -> bool {
        !self.triggers.is_empty()
    }

    /// Checks `given` against the declared params and fills in defaults.
    pub fn resolve_params(&self, given: &Map<String, Value>) -> Result<Map<String, Value>, String> {
        if let Some(unknown) = given.keys().find(|key| !self.params.iter().any(|p| &p.name == *key)) {
            let known: Vec<&str> = self.params.iter().map(|p| p.name.as_str()).collect();
            let hint = if known.is_empty() {
                "this blueprint takes no params".to_string()
            } else {
                format!("it takes: {}", known.join(", "))
            };
            return Err(format!("`{}` has no param `{unknown}`; {hint}", self.name));
        }
        let mut resolved = Map::new();
        for param in &self.params {
            let value = match given.get(&param.name).or(param.default.as_ref()) {
                Some(value) => value.clone(),
                None if param.required => {
                    return Err(format!("`{}` needs param `{}`", self.name, param.name));
                }
                None => Value::Null,
            };
            if !value.is_null() && !param.ty.accepts(&value) {
                return Err(format!(
                    "param `{}` of `{}` must be {}, got {value}",
                    param.name,
                    self.name,
                    param.ty.name()
                ));
            }
            if !param.choices.is_empty() && !value.is_null() && !param.choices.contains(&value) {
                let choices: Vec<String> = param.choices.iter().map(Value::to_string).collect();
                return Err(format!(
                    "param `{}` of `{}` must be one of {}, got {value}",
                    param.name,
                    self.name,
                    choices.join(", ")
                ));
            }
            resolved.insert(param.name.clone(), value);
        }
        Ok(resolved)
    }
}
