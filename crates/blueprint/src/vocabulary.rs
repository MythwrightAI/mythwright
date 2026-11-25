//! The language's own words: every mapping a blueprint can contain, field by
//! field, with what goes after each and a sentence saying what it is for.
//!
//! This is the one list. The reader (`lower`) refuses a field that is not
//! here, and an editor offers the ones that are, with these sentences, so
//! what is accepted and what is suggested cannot come apart. An editor is
//! handed this and walks it; it knows nothing about blueprints of its own.

use serde::Serialize;
use serde::ser::SerializeStruct;

/// What may be written after a field's colon.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "of", rename_all = "snake_case")]
pub enum Kind {
    Text,
    Flag,
    Number,
    /// A length of time: `30s`, `5m`, `1h30m`.
    Duration,
    /// One `${ ... }`, or its inside written bare.
    Expression,
    /// The name of an action.
    Action,
    /// What happens to a step that failed (`retry`, `continue` or `fail`),
    /// or the name of an action.
    Decision,
    /// The name of a blueprint.
    Blueprint,
    /// The name of an agent.
    Agent,
    /// The name of a device.
    Device,
    /// An event's topic, or a pattern over topics.
    Topic,
    /// Names of other steps in the same graph.
    Steps,
    /// Names of capabilities.
    Capabilities,
    /// One of a few words.
    OneOf(&'static [Choice]),
    /// A mapping with the fields of the shape named.
    Shape(&'static str),
    /// A mapping from names of the writer's choosing to the shape named.
    MapOf(&'static str),
    /// A list of the shape named, or one of them.
    ListOf(&'static str),
    /// The arguments of the action named, or with no name, of the action
    /// or blueprint named beside this field.
    Arguments(Option<&'static str>),
    /// A mapping of the writer's own: names and what they stand for.
    Data,
    /// Any value at all: text, a number, a list, a mapping.
    Any,
}

/// A word that may be written, and what it means.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Choice {
    pub name: &'static str,
    pub help: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub name: &'static str,
    pub value: Kind,
    /// What it is for, and when somebody reaches for it.
    pub help: &'static str,
    /// A whole blueprint written with it, to be read and copied: held to
    /// reading, checking with no warning and being laid out as the
    /// formatter lays it out (the blueprint suite's `vocabulary.rs`, and
    /// against every action there is, the API suite's `examples.rs`).
    pub example: Option<&'static str>,
}

impl Field {
    /// The word, shown by a whole blueprint written with it ([`shown`]).
    pub const fn example(self, example: &'static str) -> Field {
        Field { example: Some(example), ..self }
    }
}

/// A field as it is handed out: with what goes after it said in words
/// ([`after`]), beside the kind it is, so that whoever shows the language
/// writes no words of its own for a kind; and with its example.
impl Serialize for Field {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut field = serializer.serialize_struct("Field", 5)?;
        field.serialize_field("name", self.name)?;
        field.serialize_field("value", &self.value)?;
        field.serialize_field("help", self.help)?;
        field.serialize_field("after", &after(self.value))?;
        match self.example {
            Some(example) => field.serialize_field("example", example)?,
            None => field.skip_field("example")?,
        }
        field.end()
    }
}

/// What goes after a field of this kind, in a few words: `a length of
/// time, such as 30s or 5m`; for a field that holds a mapping, what a
/// bare value in its place is, or what each thing in it is. Said once,
/// here, for an editor and for the page that lists the language.
pub fn after(kind: Kind) -> String {
    let what = |name: &str| SHAPES.iter().find(|shape| shape.name == name).map(|shape| shape.what);
    match kind {
        Kind::Shape(name) => {
            match SHAPES.iter().find(|shape| shape.name == name).and_then(|shape| shape.short) {
                Some(short) => format!("{}, or a mapping", plainly(short)),
                None => plainly(kind).to_string(),
            }
        }
        Kind::MapOf(name) => {
            what(name).map_or_else(|| plainly(kind).to_string(), |what| format!("names, each {what}"))
        }
        Kind::ListOf(name) => {
            what(name).map_or_else(|| plainly(kind).to_string(), |what| format!("a list, each {what}"))
        }
        other => plainly(other).to_string(),
    }
}

/// A kind by itself, with nothing said of what it holds.
fn plainly(kind: Kind) -> &'static str {
    match kind {
        Kind::Text => "text",
        Kind::Flag => "true or false",
        Kind::Number => "a number",
        Kind::Duration => "a length of time, such as 30s or 5m",
        Kind::Expression => "an expression",
        Kind::Action => "an action",
        Kind::Decision => "an action, or a decision",
        Kind::Blueprint => "a blueprint",
        Kind::Agent => "an agent",
        Kind::Device => "a device",
        Kind::Topic => "a topic",
        Kind::Steps => "a list of steps",
        Kind::Capabilities => "a list of capabilities",
        Kind::OneOf(_) => "one of a few words",
        Kind::Shape(_) => "a mapping",
        Kind::MapOf(_) => "a mapping of names",
        Kind::ListOf(_) => "a list",
        Kind::Arguments(_) => "arguments",
        Kind::Data => "a mapping of your own",
        Kind::Any => "any value",
    }
}

/// A mapping with a closed set of fields.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Shape {
    pub name: &'static str,
    /// What it is called in a sentence: "a step".
    pub what: &'static str,
    /// What it is for, and when somebody writes one.
    pub help: &'static str,
    /// A whole blueprint written with one, held as a field's is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<&'static str>,
    /// What a bare value in its place means, when one is allowed: a hook
    /// may be just an action's name, a param just its default.
    pub short: Option<Kind>,
    /// Fields that stand in for each other: a mapping has one of each
    /// group and never two. A step has one of `do`, `call`, `spawn` and
    /// `task`.
    pub alternatives: &'static [&'static [&'static str]],
    pub fields: &'static [Field],
}

impl Shape {
    pub fn has(&self, name: &str) -> bool {
        self.fields.iter().any(|field| field.name == name)
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> {
        self.fields.iter().map(|field| field.name)
    }
}

mod called;
mod scenarios;
mod shown;

pub use called::CALLED;
pub use scenarios::{SCENARIOS, Scenario};

const fn field(name: &'static str, value: Kind, help: &'static str) -> Field {
    Field { name, value, help, example: None }
}

const fn choice(name: &'static str, help: &'static str) -> Choice {
    Choice { name, help }
}

const HOOKS: Kind = Kind::ListOf("hook");
const FAILURE_HOOKS: Kind = Kind::ListOf("failure_hook");
const EMIT: &str =
    "Say an event when this is done: short for `do: emit`, with its topic and what it carries.";
const WITH: Field = field(
    "with",
    Kind::Arguments(None),
    "What the action named beside it is handed, or the params the blueprint named beside it is started with: \
     each by its name.",
);

pub const BLUEPRINT: Shape = Shape {
    name: "blueprint",
    what: "a blueprint",
    help: "The text a piece of work is written as: what it is started with, the steps it runs once, and what it \
           does each time something happens while it is alive. Applied, it is an instance, which runs it.",
    example: Some(shown::NIGHTLY),
    short: None,
    alternatives: &[],
    fields: &[
        field(
            "blueprint",
            Kind::Text,
            "Its name, which is how it is applied, called and found again. Lowercase letters, digits, `-`, `_`, `.` and `/`.",
        )
        .example(shown::NIGHTLY),
        field(
            "version",
            Kind::Text,
            "A version of your own choosing, kept with it: what is said of an instance running an earlier text \
             names it, so it goes up when what the text does changes.",
        )
        .example(shown::VERSIONED),
        field(
            "description",
            Kind::Text,
            "What it is for, in a sentence: shown where blueprints are listed, and first in what it says in plain words.",
        )
        .example(shown::VERSIONED),
        field(
            "params",
            Kind::MapOf("param"),
            "What an instance is started with. `port: 8080` gives a default; a name with nothing after it must be given.",
        )
        .example(shown::GREETING),
        field(
            "state",
            Kind::Data,
            "Durable properties of an instance, with the values they start at. Change them with `do: set`.",
        )
        .example(shown::LAST_SEEN),
        field(
            "permissions",
            Kind::Capabilities,
            "What it may use, checked before anything runs. With none listed it runs with everything its caller has.",
        )
        .example(shown::DISK),
        field(
            "env",
            Kind::Data,
            "Environment for every action it runs: each variable text, or a secret's name as `{ secret: <name> }`.",
        )
        .example(shown::BUILD_ENV),
        field(
            "uses",
            Kind::MapOf("attachment"),
            "Other blueprints attached to every instance of this one, each started as a child.",
        )
        .example(shown::ATTACHED),
        field(
            "steps",
            Kind::MapOf("step"),
            "The work, as a graph of steps. Runs once, when an instance starts.",
        )
        .example(shown::FETCH),
        field(
            "on",
            Kind::MapOf("handler"),
            "Handlers: what to do each time something happens while the instance is alive.",
        )
        .example(shown::DONE_NEWS),
        field(
            "outputs",
            Kind::Data,
            "What a blueprint that calls this one is given back once its steps have run: names, and what each comes to.",
        )
        .example(shown::OUTPUTS),
        field("on_success", HOOKS, "Done when the steps have gone well, to tell of it or go on from it.")
            .example(shown::RELEASE),
        field("on_fail", HOOKS, "Done when the steps have failed, to tell of it or undo what they began.")
            .example(shown::RELEASE),
        field(
            "on_finish",
            HOOKS,
            "Done when the steps have ended, however they went, after `on_success` or `on_fail`; `run.status` says which.",
        )
        .example(shown::RELEASE),
        field(
            "on_stop",
            HOOKS,
            "Done when the instance is stopped, before what it holds is given back: where it undoes what it set up \
             outside the factory.",
        )
        .example(shown::STOPPED),
        field(
            "on_update",
            HOOKS,
            "Done when a running instance takes this text, in place of running its steps again: it carries on from where it is. `was` is what it was before: `was.version`, `was.revision`, `was.state`, `was.params`. If one of these fails, the instance goes back to the text and the state it had.",
        )
        .example(shown::CARRIED),
        field(
            "max_parallel",
            Kind::Number,
            "How many of its steps may run side by side, 8 unless said: for steps that each take a share of \
             something there is only so much of.",
        )
        .example(shown::PARALLEL),
        field(
            "durable",
            Kind::Flag,
            "Whether what an instance is doing is written down as it goes, to be picked up after a restart: true \
             unless said. With `false` it keeps nothing as it goes: what each step did is written once, when the run \
             ends, its state is its own while it lives, and a restart ends it. For work that is short and can \
             simply be done again.",
        )
        .example(shown::SHORT_LIVED),
        field(
            "one_at_a_time",
            Kind::Flag,
            "Its handlers take turns: one run of any of them at a time, so that one which looks at `state` and then \
             changes it is not overtaken by another. A handler that waits holds up the rest.",
        )
        .example(shown::LOCK),
    ],
};

pub const STEP: Shape = Shape {
    name: "step",
    what: "a step",
    help: "One thing a blueprint does: an action, another blueprint called or started, or a task of steps, with \
           what is said around it: when it runs, what it waits for, how long it may take and what happens if it \
           fails.",
    example: Some(shown::FETCH),
    short: None,
    alternatives: &[&["do", "call", "spawn", "task"]],
    fields: &[
        field(
            "do",
            Kind::Action,
            "The action the step runs, by its name, with what it is handed under `with`: what most steps are.",
        )
        .example(shown::FETCH),
        field(
            "call",
            Kind::Blueprint,
            "Run another blueprint as a child and wait for its steps. Its outputs become this step's.",
        )
        .example(shown::CALLED),
        field(
            "spawn",
            Kind::Blueprint,
            "Start another blueprint as a child and carry on, for something that is to go on beside this one.",
        )
        .example(shown::SPAWNED),
        field(
            "task",
            Kind::MapOf("step"),
            "Run the steps under it as one: if one fails the rest do not run and the task has failed. What is said beside it (`when`, `retry`, `timeout`, `on_fail`) is said once, for all of them.",
        )
        .example(shown::TASK),
        WITH.example(shown::FETCH),
        field(
            "needs",
            Kind::Steps,
            "Steps that must settle first. Without it a step follows the one above it; `[]` starts at once.",
        )
        .example(shown::NEEDS),
        field(
            "when",
            Kind::Expression,
            "A guard. When false the step is skipped, and what follows still runs.",
        )
        .example(shown::WHEN),
        field("timeout", Kind::Duration, "Fail the step if it runs longer, so that one that hangs does not hold the run up.")
            .example(shown::TIMED),
        field(
            "retry",
            Kind::Shape("retry"),
            "Try again after a failure: a count, or `max`, `backoff` and `factor`. For a step that may fail for a \
             moment, such as one that asks something over the network.",
        )
        .example(shown::FETCH),
        field(
            "allow_failure",
            Kind::Flag,
            "A failure here does not fail the run; later steps see `steps.<id>.failed`.",
        )
        .example(shown::LINT),
        field(
            "again",
            Kind::Flag,
            "Run the step again from its start where the daemon was stopped in the middle of it, in place of \
             failing it as not safe to repeat: for a command that does no harm done twice, such as a build or a \
             test. A failure of its own is not tried again; that is `retry`.",
        )
        .example(shown::REBUILT),
        field(
            "watch",
            Kind::ListOf("watch"),
            "Rules looked at while the step is still running: after so long, every so often, or when a line it prints matches.",
        )
        .example(shown::WATCHED),
        field("on_start", HOOKS, "Done as the step starts, before its own action begins.").example(shown::BUILD_HOOKS),
        field("on_success", HOOKS, "Done when the step has gone well, with `self` the step it belongs to.")
            .example(shown::BUILD_HOOKS),
        field(
            "on_finish",
            HOOKS,
            "Done when the step has ended, however it went, after `on_success`, `on_fail` or `on_timeout`; `self.status` says which.",
        )
        .example(shown::BUILD_HOOKS),
        field(
            "on_fail",
            FAILURE_HOOKS,
            "Done when the step fails. It may decide what happens next: `retry`, `continue` or `fail`.",
        )
        .example(shown::FAILED),
        field(
            "on_timeout",
            FAILURE_HOOKS,
            "Done when the step runs out of time, in place of `on_fail`. It may decide what happens next.",
        )
        .example(shown::TIMED),
        field(
            "device",
            Kind::Device,
            "Run on another device, through the gateway: for work that has to be done where something is.",
        )
        .example(shown::ELSEWHERE),
        field(
            "measure",
            Kind::Text,
            "Also report how long the step took under this name, to be looked at beside every other time it ran.",
        )
        .example(shown::MEASURED),
        field(
            "labels",
            Kind::Data,
            "For `call`, `spawn` and `task`: names and values that everything measured under the step carries, as in `{ experiment: cart, variant: a }`, so that two ways of doing the same work can be told apart and compared.",
        )
        .example(shown::LABELLED),
        field(
            "as",
            Kind::Text,
            "For `call` and `spawn`: the child's name within its parent, by which it is found again when the \
             blueprint is applied again.",
        )
        .example(shown::SPAWNED),
        field(
            "environment",
            Kind::Text,
            "For `call` and `spawn`, where the blueprint runs under none of its project's environments: the one its \
             child is started under, as `prod`, held there to that environment's rules (who approves what goes \
             under it, which branches it lets run). Under one, a child is under that one and no other.",
        )
        .example(shown::DEPLOYED),
        field(
            "if_fail_ask_agent",
            Kind::Agent,
            "When the step fails, tell this agent what failed and let it put it right; it says whether the step is run again.",
        )
        .example(shown::ASKED),
        field(
            "else",
            Kind::Text,
            "With `if_fail_ask_agent`: the name the step fails under when the agent could not put it right.",
        )
        .example(shown::ASKED),
        field(
            "for_each",
            Kind::Expression,
            "Run once for each element of a list, in order, or so many times where it is handed a number (`item` \
             then goes from 1 to it); `item` and `index` are available.",
        )
        .example(shown::EACH),
        field(
            "description",
            Kind::Text,
            "What the step is for, in a sentence. Shown for the step in the node view and in the steps of a run.",
        )
        .example(shown::TASK),
        field(
            "env",
            Kind::Data,
            "Environment for the action, added to the blueprint's: each variable text, or a secret's name as \
             `{ secret: <name> }`.",
        )
        .example(shown::STEP_ENV),
    ],
};

pub const HOOK: Shape = Shape {
    name: "hook",
    what: "a hook",
    help: "Something done at a point in the life of a step or of the blueprint (as it starts, goes well, fails, \
           ends or is stopped): an action, or an event said. Written as the action's name alone where it needs \
           nothing.",
    example: Some(shown::BUILD_HOOKS),
    short: Some(Kind::Action),
    alternatives: &[&["do", "emit"]],
    fields: &[
        field(
            "do",
            Kind::Action,
            "The action done at that point, by its name, with what it is handed under `with`.",
        )
        .example(shown::BUILD_HOOKS),
        field("emit", Kind::Arguments(Some("emit")), EMIT).example(shown::BUILD_HOOKS),
        WITH.example(shown::BUILD_HOOKS),
        field(
            "when",
            Kind::Expression,
            "Only when this holds is the hook done; it is passed over otherwise.",
        )
        .example(shown::TOLD),
    ],
};

/// A hook on a step's failure. It has a hook's fields; what it runs may
/// also be a decision about the step.
pub const FAILURE_HOOK: Shape = Shape {
    name: "failure_hook",
    what: "a hook",
    help: "Something done when a step has failed or run out of time, which may also decide what becomes of the \
           step: tried again (`retry`), let pass (`continue`) or failed (`fail`).",
    example: Some(shown::FAILED),
    short: Some(Kind::Decision),
    alternatives: &[&["do", "emit"]],
    fields: &[
        field(
            "do",
            Kind::Decision,
            "Run an action, or decide what happens to the step: `retry`, `continue` or `fail`.",
        )
        .example(shown::FAILED),
        field("emit", Kind::Arguments(Some("emit")), EMIT).example(shown::FAILED),
        WITH.example(shown::FAILED),
        field(
            "when",
            Kind::Expression,
            "Only when this holds is it done or decided, as `self.attempts` says how often the step was tried.",
        )
        .example(shown::FAILED),
    ],
};

pub const WATCH: Shape = Shape {
    name: "watch",
    what: "a watch rule",
    help: "A rule looked at while a step is still running: after so long, every so often, or when a line the step \
           prints matches, something is done, and the step may be stopped as failed.",
    example: Some(shown::WATCHED),
    short: None,
    alternatives: &[&["after", "every", "log"], &["do", "emit"]],
    fields: &[
        field(
            "after",
            Kind::Duration,
            "Once, when the step has run this long. Written with `${ }` it is worked out as the step begins, and \
             where it comes to nothing the rule is never timed.",
        )
        .example(shown::WATCHED),
        field(
            "every",
            Kind::Duration,
            "Again and again, this far apart, while the step runs: worked out as the step begins where it is \
             written with `${ }`, and never timed where that comes to nothing.",
        )
            .example(shown::WATCHED),
        field("log", Kind::Text, "When a line the step prints matches this pattern, once per attempt.")
            .example(shown::WATCHED),
        field(
            "do",
            Kind::Action,
            "The action done when the rule fires, with what it is handed under `with`.",
        )
        .example(shown::WATCHED),
        field("emit", Kind::Arguments(Some("emit")), EMIT).example(shown::WATCHED),
        WITH.example(shown::WATCHED),
        field("when", Kind::Expression, "Only when this holds does the rule fire.").example(shown::WATCHED),
        field("fail", Kind::Flag, "Also stop the step, as failed.").example(shown::WATCHED),
    ],
};

pub const SCOPES: &[Choice] = &[
    choice(
        "project",
        "events of its own project and the factory's own, which are no project's (the default)",
    ),
    choice("fleet", "every event in the factory, whichever project's"),
    choice("own", "only events from this instance and its children"),
    choice("target", "only events from the instance this one is attached to"),
];

pub const HANDLER: Shape = Shape {
    name: "handler",
    what: "a handler",
    help: "What an instance does each time something happens while it is alive: an event it hears, a timer, or \
           an answer that changes, and the steps it runs then. Written under `on`, each by a name of your own.",
    example: Some(shown::DONE_NEWS),
    short: None,
    alternatives: &[&["event", "every", "poll", "at"], &["steps", "do", "call"]],
    fields: &[
        field(
            "event",
            Kind::Topic,
            "Fires on an event whose topic matches. `*` is one word, `**` any number.",
        )
        .example(shown::DONE_NEWS),
        field(
            "every",
            Kind::Duration,
            "Fires on a timer, this far apart: for what is done again and again while the instance is alive.",
        )
        .example(shown::HOURLY),
        field(
            "at",
            Kind::Text,
            "Fires at a time of day in the zone written after it, `02:00 Europe/Zagreb`, with the days before it where it is not every day (`mon,thu 09:30 UTC`). A time that came while the factory was stopped is fired once, as it is back.",
        )
        .example(shown::BEFORE_DAWN),
        field(
            "poll",
            Kind::Shape("poll"),
            "Fires when an action's result changes. For things that cannot say when they change.",
        )
        .example(shown::POLLED),
        field(
            "for",
            Kind::Data,
            "Which events are addressed to it: fields of the event's data and the value each must have, as in `{ id: \"${params.item}\" }`, a field inside one named by its path (`fields.sprint`). The conductor decides that, so a handler is not woken for what is another's.",
        )
        .example(shown::ONE_ORDER),
        field(
            "when",
            Kind::Expression,
            "A guard over `event`: the handler runs only for an event of which it holds.",
        )
        .example(shown::DONE_NEWS),
        field(
            "scope",
            Kind::OneOf(SCOPES),
            "Whose events it hears: its own project's unless said, or every project's, or only those of what it started.",
        )
        .example(shown::EVERY_FAILURE),
        field("debounce", Kind::Duration, "Makes a burst of events one run, with the latest of them.")
            .example(shown::BURST),
        field(
            "outside",
            Kind::Flag,
            "Its event is said from outside: by a person, a key or a caller, and by no blueprint. The check then does not look for a blueprint that says it.",
        )
        .example(shown::LOCK),
        field(
            "turn",
            Kind::Text,
            "What its events take turns by, worked out from each as it comes: `turn: \"${event.data.id}\"`. What comes to the same key, here or at another handler of this instance that says `turn`, is handled one at a time, and what one handler hears of it in the order it was said; what comes to another key is handled beside it. For an instance that looks after many things of a kind, so that one of them holds up none of the others.",
        )
        .example(shown::ORDERS),
        field("steps", Kind::MapOf("step"), "What to do, as a graph of steps, where it is more than one thing.")
            .example(shown::DONE_STEPS),
        field("do", Kind::Action, "What to do, when it is one action.").example(shown::HOURLY),
        field("call", Kind::Blueprint, "What to do, when it is one blueprint.").example(shown::ON_ASKING),
        WITH.example(shown::ON_ASKING),
    ],
};

pub const POLL: Shape = Shape {
    name: "poll",
    what: "`poll`",
    help: "What a handler asks again and again of something that cannot say when it changes: an action and how \
           often, the handler running each time the answer is other than the time before. It is first asked as the \
           handler begins to listen, and that answer is only what the next is told from.",
    example: Some(shown::POLLED),
    short: None,
    alternatives: &[],
    fields: &[
        field("every", Kind::Duration, "How often to ask, after the first time, which is as the handler begins to listen. Never more often than every 5s.")
            .example(shown::POLLED),
        field("do", Kind::Action, "The action whose result is watched.").example(shown::POLLED),
        WITH.example(shown::POLLED),
    ],
};

/// The types a param may have. The same names as [`crate::ValueType`].
pub const TYPES: &[Choice] = &[
    choice("string", "text"),
    choice("int", "a whole number"),
    choice("number", "a number"),
    choice("bool", "true or false"),
    choice("list", "a list"),
    choice("object", "a mapping"),
    choice("duration", "a length of time, such as `30s` or `5m`"),
    choice("any", "anything"),
];

pub const PARAM: Shape = Shape {
    name: "param",
    what: "a param",
    help: "One thing an instance is started with, under `params`: what type it is, what it is when nobody says, \
           and whether it must be given. Read in the text as `params.<name>`.",
    example: Some(shown::GREETING),
    short: Some(Kind::Any),
    alternatives: &[],
    fields: &[
        field("type", Kind::OneOf(TYPES), "What it must be. Without it, the type follows from the default.")
            .example(shown::BUILD_PARAMS),
        field("default", Kind::Any, "Its value when none is given, which also says its type.")
            .example(shown::BUILD_PARAMS),
        field("required", Kind::Flag, "Whether it must be given. A param with no default must be.")
            .example(shown::BUILD_PARAMS),
        field(
            "description",
            Kind::Text,
            "What it is, in a sentence, shown to whoever applies the blueprint and fills it in.",
        )
        .example(shown::BUILD_PARAMS),
        field("enum", Kind::Any, "The values it may have, as a list; any other is refused.")
            .example(shown::BUILD_PARAMS),
    ],
};

pub const RETRY: Shape = Shape {
    name: "retry",
    what: "`retry`",
    help: "How a step that failed is tried again: a count alone, or how many times and how long to wait before \
           each, the wait growing each time.",
    example: Some(shown::FETCH),
    short: Some(Kind::Number),
    alternatives: &[],
    fields: &[
        field("max", Kind::Number, "How many times to try again after the first.").example(shown::FETCH),
        field("backoff", Kind::Duration, "How long to wait before trying again. 1s unless said.")
            .example(shown::FETCH),
        field("factor", Kind::Number, "What each wait is multiplied by to make the next. 2 unless said.")
            .example(shown::FETCH),
        field("max_backoff", Kind::Duration, "The longest a wait may grow to. 5m unless said.")
            .example(shown::FETCH),
    ],
};

pub const ATTACHMENT: Shape = Shape {
    name: "attachment",
    what: "an attachment",
    help: "Another blueprint started as a child of every instance of this one, under `uses`, and stopped with it: \
           for what is always to be there beside it.",
    example: Some(shown::ATTACHED),
    short: Some(Kind::Blueprint),
    alternatives: &[],
    fields: &[
        field("blueprint", Kind::Blueprint, "The blueprint that is attached, by its name.")
            .example(shown::ATTACHED),
        WITH.example(shown::ATTACHED),
    ],
};

/// Every shape there is. The first is the file itself.
pub const SHAPES: &[Shape] =
    &[BLUEPRINT, PARAM, ATTACHMENT, STEP, RETRY, HOOK, FAILURE_HOOK, WATCH, HANDLER, POLL];

/// The words of the language a text is written with, each as the shape
/// it is a word of and its name, in the order they are first met. Read
/// from the text as the reader reads it, so that whatever holds a word (a
/// test's case, an example) says so by being written with it and by
/// nothing kept beside it. A text that does not read is written with
/// none.
pub fn written(source: &str) -> Vec<(&'static str, &'static str)> {
    use crate::syntax::Node;
    fn under(node: &Node, of: &'static str, found: &mut Vec<(&'static str, &'static str)>) {
        let Some(shape) = SHAPES.iter().find(|shape| shape.name == of) else { return };
        for entry in node.as_map().unwrap_or_default() {
            let Some(field) = shape.fields.iter().find(|field| field.name == entry.key) else { continue };
            if !found.contains(&(shape.name, field.name)) {
                found.push((shape.name, field.name));
            }
            let each = |inside: &'static str, found: &mut Vec<_>| {
                for one in entry.value.as_map().unwrap_or_default() {
                    under(&one.value, inside, found);
                }
            };
            match field.value {
                Kind::Shape(inside) => under(&entry.value, inside, found),
                Kind::MapOf(inside) => each(inside, found),
                Kind::Steps => each("step", found),
                Kind::ListOf(inside) => {
                    for one in entry.value.as_seq().unwrap_or_default() {
                        under(one, inside, found);
                    }
                }
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    if let Ok(root) = crate::syntax::parse(source) {
        under(&root, "blueprint", &mut found);
    }
    found
}

/// What follows `steps.<id>` and `self` in an expression.
pub const STEP_VIEW: &[Choice] = &[
    choice("ok", "true when it finished without failing"),
    choice("failed", "true when it failed"),
    choice("skipped", "true when its `when` was false"),
    choice("status", "how it went, as a word"),
    choice("out", "what its action returned"),
    choice("count", "how many things that is, where it is a list: one for each element of a `for_each`"),
    choice("error", "why it failed, when it did"),
    choice(
        "reason",
        "why it failed, alone: what the innermost step said, without the steps it came up through",
    ),
    choice("attempts", "how many times it was tried"),
    choice("duration_ms", "how long it took"),
];

/// What follows `event` in an expression.
pub const EVENT_VIEW: &[Choice] = &[
    choice("topic", "what happened, as dotted words"),
    choice("data", "what the event carries"),
    choice("source", "where it came from: `.instance`, `.run`, `.step`, `.device`, `.actor`"),
    choice("ts", "when, in milliseconds since 1970"),
    choice("seq", "its place in the journal"),
];

/// What follows `event.source` in an expression.
pub const SOURCE_VIEW: &[Choice] = &[
    choice("instance", "the instance that said it"),
    choice("run", "the run that said it"),
    choice("step", "the step that said it"),
    choice("device", "the device it was said on"),
    choice("actor", "who caused it: `user:<id>`, `agent:<slot>`, `device:<name>` or `system`"),
];

/// What follows `instance` in an expression.
pub const INSTANCE_VIEW: &[Choice] = &[
    choice("id", "this instance's id"),
    choice("name", "the blueprint it is an instance of"),
    choice("key", "its name within its parent"),
    choice("parent", "the instance that started it, if one did"),
    choice("project", "the project it belongs to"),
    choice("device", "the device it runs on"),
];

/// What follows `was` in an expression: an instance as it was before it
/// took the text `on_update` is written in.
pub const WAS_VIEW: &[Choice] = &[
    choice("version", "what the text it ran before called its own version; nothing if it did not say"),
    choice("revision", "the revision of the text it ran before"),
    choice("state", "its state as it was, with the keys this text no longer declares: `was.state.<key>`"),
    choice("params", "the params it had before: `was.params.<name>`"),
];

/// What follows `run` in an expression: the run of a blueprint's steps,
/// in what the blueprint says is done when they have ended.
pub const RUN_VIEW: &[Choice] = &[
    choice("status", "how its steps went: `ok` or `failed`"),
    choice("error", "why they failed, when they did"),
    choice("id", "the run's id"),
];

/// What follows `target` in an expression.
pub const TARGET_VIEW: &[Choice] = &[choice("id", "the id of the instance this one is attached to")];

/// What follows `environment` in an expression: the environment of its
/// project's (dev, staging, prod) that the instance was applied under. It is
/// a map so that it can come to hold more than what it is called; what it
/// holds is listed here, and the checker refuses a member that is not.
/// What follows `vars` is not checked: those names are the project's.
pub const ENVIRONMENT_VIEW: &[Choice] = &[
    choice(
        "name",
        "what the environment of its project's it was applied under is called, as text; empty where it was applied \
         under none",
    ),
    choice(
        "vars",
        "what that environment says, by name: `environment.vars.<name>` is text, a number, or true or false, and \
         nothing where it says none. Not for secrets: those are named in `env` as `{ secret: <name> }`",
    ),
];

/// What follows `profile` in an expression: the profile it runs in, which
/// is the copy of the factory (`prod`, `dev1`), held as `environment` is.
pub const PROFILE_VIEW: &[Choice] = &[
    choice("name", "what this profile, the copy of the factory, is called, as text; empty for one with none"),
    choice(
        "vars",
        "what the owner has said is true of this profile, by name: `profile.vars.<name>` is text, a number, or \
         true or false, and nothing if it was not set. Not for secrets",
    ),
];

/// A name an expression may start with: what it is, in words, and a
/// whole blueprint that reads it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Root {
    pub name: &'static str,
    pub meaning: &'static str,
    pub example: &'static str,
}

const fn root(name: &'static str, meaning: &'static str, example: &'static str) -> Root {
    Root { name, meaning, example }
}

/// Names an expression may start with.
pub const ROOTS: &[Root] = &[
    root(
        "params",
        "the values this instance was started with, each by its name: `params.<name>`",
        shown::GREETING,
    ),
    root(
        "state",
        "what the instance keeps between one run and the next, as declared under `state` and changed with `set`: \
         `state.<key>`",
        shown::LOCK,
    ),
    root(
        "steps",
        "results of earlier steps: steps.<id>.ok, .failed, .skipped, .out, .error, .duration_ms",
        shown::LINT,
    ),
    root(
        "self",
        "inside a hook or watch rule, the step it belongs to: how it went and what it gave",
        shown::BUILD_HOOKS,
    ),
    root(
        "event",
        "inside an `on` handler, the event that fired it: event.topic, event.data",
        shown::DONE_NEWS,
    ),
    root("item", "inside a `for_each` step, the current element", shown::EACH),
    root("index", "inside a `for_each` step, the current position, counted from 0", shown::EACH),
    root("instance", "this instance: instance.id, .name, .key, .device", shown::WHO),
    root(
        "target",
        "the instance this one is attached to, in a blueprint that is attached under another's `uses`",
        shown::SHADOW,
    ),
    root(
        "environment",
        "the environment of its project's it was applied under (dev, staging, prod): environment.name, \
         environment.vars.<name>",
        shown::STAGED,
    ),
    root(
        "profile",
        "the profile it runs in, the copy of the factory (prod, dev1): profile.name, profile.vars.<name>",
        shown::CAREFUL,
    ),
    root(
        "was",
        "inside `on_update`, the instance as it was before it took this text: was.version, .revision, .state, .params",
        shown::CARRIED,
    ),
    root(
        "run",
        "inside the blueprint's own `on_success`, `on_fail` and `on_finish`, the run of its steps: run.status, .error, .id",
        shown::RELEASE,
    ),
];

/// What may follow a name an expression starts with. Nothing for a name
/// whose inside is the writer's own (`params`, `state`), or is other names
/// (`steps`).
pub fn members(root: &str) -> &'static [Choice] {
    match root {
        "self" => STEP_VIEW,
        "event" => EVENT_VIEW,
        "instance" => INSTANCE_VIEW,
        "target" => TARGET_VIEW,
        "environment" => ENVIRONMENT_VIEW,
        "profile" => PROFILE_VIEW,
        "was" => WAS_VIEW,
        "run" => RUN_VIEW,
        _ => &[],
    }
}
