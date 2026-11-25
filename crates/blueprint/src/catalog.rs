//! The catalog of actions a blueprint may `do`.
//!
//! The language does not know what actions exist; the runtime tells it. That
//! keeps this crate free of I/O while still letting a blueprint be checked
//! completely before anything runs: an unknown action, a missing argument or
//! an undeclared capability is an error in the editor, not a failure at 3am.

use serde::Serialize;

use crate::events::{EventSpec, Said};
use crate::inside::{Row, Within};
use crate::types::ValueType;

pub(crate) mod asked;

#[derive(Clone, Debug, Serialize)]
pub struct ActionSpec {
    pub name: String,
    pub summary: String,
    pub args: Vec<ArgSpec>,
    /// Accept arguments beyond those listed.
    pub open: bool,
    /// The capability a blueprint must hold to use this action.
    pub capability: Option<String>,
    /// What the action returns, as `(field, meaning)` pairs.
    pub outputs: Vec<(String, String)>,
    /// Safe to run again after an interruption: done again from its
    /// start, or from what it saved, and told so, it leaves nothing
    /// twice (one that leaves something looks for it then). So its
    /// step's ending is held, and written with the next thing its run
    /// writes: a stop before that is taken up as one in the middle of
    /// its action is (`conductor::holds`).
    pub idempotent: bool,
    /// Says nothing, writes nothing and does nothing outside the factory:
    /// what it gives is worked out from what it is handed and what it
    /// reads. Done once more from its start after a stop, it comes to what
    /// it would have come to then, and nothing of it is owed a record of
    /// its own before whatever acts on what it gave is written down. No
    /// part of what an action is described as.
    #[serde(skip)]
    pub decides: bool,
    /// What it brings into being that outlasts the run that asked for it,
    /// where it makes such a thing: a table, a channel, a service. Left
    /// out of what is handed out where it says none, as `runs` is: an
    /// action that says neither is described as it always was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub makes: Option<Makes>,
    /// Its arguments that are a command it runs on the device, each by
    /// its name: a program and its arguments, or a line a shell reads.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<String>,
    /// What a stand-in for it answers where nobody says: for an action
    /// that reaches outside the factory's tables, which a sandbox test
    /// is never given, and whose usual answer has to be
    /// something for the texts that ask it to go on (a command that
    /// ended with 0). One that says none is answered with nothing. It
    /// is no part of what an action is handed out as: a sandbox is handed
    /// the description itself, and says what each stand-in usually
    /// answers where it says what one was asked.
    #[serde(skip)]
    pub stands: Option<serde_json::Value>,
    /// The blueprints it reaches by a name of its own, which no text
    /// that asks it writes: a rule it asks, something it starts. Whoever
    /// says which blueprints run which ([`crate::names`]) reads it from
    /// here, and so keeps no list of names.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reaches: Vec<String>,
    /// A whole blueprint that asks it, for the Reference and the editor to
    /// show it by; held to checking with no warning (the API suite's
    /// `examples.rs`). It is no part of what a blueprint depends on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// What an action makes, as it says so itself: the thing in a few words
/// with their article, written so that "called <a name>" reads after
/// them (`a table`, `the first rows of the table`), and the argument
/// that names it. Whoever says what a blueprint does in words
/// ([`crate::summary`]) reads it from here, and so lists no action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Makes {
    pub what: String,
    pub named: String,
    /// The argument that, where a step says it, makes what it makes one
    /// whose rows set what something does: a table that sits beside
    /// another (`beside`) is one that says something of that other, as a
    /// fleet's tables say who is in it and what is done with its cards,
    /// where a board's own rows are the cards. A summary offers to change
    /// the rows of what is made so, and of nothing else it makes. Left out
    /// of what is handed out where it says none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    /// How what it makes is read as a model, where it has one: a table's
    /// fields. Said in words with the rest of a summary, and handed out
    /// there; no part of what is handed out of the action.
    #[serde(skip)]
    pub model: Option<Modelled>,
}

/// How what an action makes is read as a model ([`ActionSpec::modelled`]):
/// the words the action itself reads, so that whoever says a blueprint in
/// words reads them from here and knows no action. Each is a key the
/// action takes: of a step's arguments, the one that lists the parts of
/// what is made, each by its name, and those that say how it stands
/// among others; of a part, the keys that say what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modelled {
    /// The argument that lists its parts, each by its name with what it is.
    pub parts: &'static str,
    /// Of a part: the key that says its kind, and the kind where it says none.
    pub kind: (&'static str, &'static str),
    /// Of a part: what it may be (a list), or what it names (a word).
    pub of: &'static str,
    /// The kinds of part whose `of` names another of what is made, by its name.
    pub naming: &'static [&'static str],
    /// Of a part: that a thing made cannot be without it.
    pub required: &'static str,
    /// Of a part: what giving it a value takes, and the word for a list of scopes each of which it gives, with what that means.
    pub takes: (&'static str, &'static str, &'static str),
    /// Of a part: what stands for it where nothing is said, and the words usually given it.
    pub unsaid: &'static str,
    pub suggests: &'static str,
    /// The arguments that say what it sits beside and what each of its things is about there.
    pub beside: (&'static str, &'static str),
    /// The argument that, said false, says its things are not written by hand.
    pub by_hand: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct ArgSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: ValueType,
    pub required: bool,
    pub help: String,
    /// What is written inside it, where it is a mapping with words of
    /// its own that the action says ([`Within`]). Left out of what is
    /// handed out where it says nothing, as `makes` is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub within: Option<Within>,
}

impl ActionSpec {
    pub fn new(name: &str, summary: &str) -> Self {
        Self {
            name: name.to_string(),
            summary: summary.to_string(),
            args: Vec::new(),
            open: false,
            capability: None,
            outputs: Vec::new(),
            idempotent: false,
            decides: false,
            makes: None,
            runs: Vec::new(),
            stands: None,
            reaches: Vec::new(),
            example: None,
        }
    }

    /// Says the whole blueprint it is shown by ([`ActionSpec::example`]).
    pub fn example(mut self, example: &str) -> Self {
        self.example = Some(example.to_string());
        self
    }

    pub fn arg(mut self, name: &str, ty: ValueType, help: &str) -> Self {
        self.args.push(ArgSpec {
            name: name.to_string(),
            ty,
            required: false,
            help: help.to_string(),
            within: None,
        });
        self
    }

    pub fn required(mut self, name: &str, ty: ValueType, help: &str) -> Self {
        self.args.push(ArgSpec {
            name: name.to_string(),
            ty,
            required: true,
            help: help.to_string(),
            within: None,
        });
        self
    }

    /// Says what is written inside `argument`, which is one already
    /// said: a table of rows, each where something is written, what kind
    /// of thing it is and a sentence ([`crate::inside`]). Whoever shows a
    /// blueprint explains and offers by it; nothing is checked by it.
    pub fn within(mut self, argument: &str, rows: &[Row]) -> Self {
        if let Some(arg) = self.args.iter_mut().find(|arg| arg.name == argument) {
            arg.within = Some(Within::of(rows));
        }
        self
    }

    pub fn open(mut self) -> Self {
        self.open = true;
        self
    }

    pub fn capability(mut self, capability: &str) -> Self {
        self.capability = Some(capability.to_string());
        self
    }

    pub fn output(mut self, field: &str, meaning: &str) -> Self {
        self.outputs.push((field.to_string(), meaning.to_string()));
        self
    }

    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Says that it only decides ([`ActionSpec::decides`]).
    pub fn deciding(mut self) -> Self {
        self.decides = true;
        self
    }

    /// Says that it makes something that is there after the run: `what`
    /// it is, and the argument that says what it is called. (Called
    /// `making` and not after the field it fills: a journal is told
    /// which tables it makes by a method of that name, and a test reads
    /// the source for every place one is asked.)
    pub fn making(mut self, what: &str, named: &str) -> Self {
        self.makes =
            Some(Makes { what: what.to_string(), named: named.to_string(), setting: None, model: None });
        self
    }

    /// Says that where a step says `argument`, what it makes is one whose
    /// rows set what something does ([`Makes::setting`]). Said after
    /// [`ActionSpec::making`], of what that says it makes.
    pub fn setting(mut self, argument: &str) -> Self {
        if let Some(makes) = self.makes.as_mut() {
            makes.setting = Some(argument.to_string());
        }
        self
    }

    /// Says how what it makes is read as a model ([`Makes::model`]). Said
    /// after [`ActionSpec::making`], of what that says it makes.
    pub fn modelled(mut self, model: Modelled) -> Self {
        if let Some(makes) = self.makes.as_mut() {
            makes.model = Some(model);
        }
        self
    }

    /// Says that `argument` is a command it runs on the device.
    pub fn runs(mut self, argument: &str) -> Self {
        self.runs.push(argument.to_string());
        self
    }

    /// Says what a stand-in for it answers where nobody says ([`ActionSpec::stands`]).
    pub fn stands(mut self, answer: serde_json::Value) -> Self {
        self.stands = Some(answer);
        self
    }

    /// Says that it reaches the blueprint called `name` by itself: a
    /// text that asks this action runs that blueprint, though it writes
    /// its name nowhere.
    pub fn reaching(mut self, name: &str) -> Self {
        self.reaches.push(name.to_string());
        self
    }
}

/// What the checker needs to know about the world outside the file.
pub trait Catalog {
    fn action(&self, name: &str) -> Option<&ActionSpec>;

    fn action_names(&self) -> Vec<&str>;

    /// Whether this catalog lists every action there is, so that a name it
    /// does not know is a mistake. False for a catalog that knows nothing.
    fn is_complete(&self) -> bool {
        true
    }

    /// What to say of an action this catalog does not have, when it knows
    /// more than that it is missing: that it comes with something not
    /// installed here, say, and how to get it. Asked before anything else
    /// is said of an unknown name, and by a catalog that is not complete
    /// too: one that knows only some actions can still say that a name
    /// which ought to be among them is not.
    fn absent(&self, _name: &str) -> Option<String> {
        None
    }

    /// Whether a blueprint with this name can be found. `None` means the
    /// catalog cannot tell, and the checker says nothing.
    fn has_blueprint(&self, _name: &str) -> Option<bool> {
        None
    }

    /// Every event this catalog describes.
    fn events(&self) -> Vec<&EventSpec> {
        Vec::new()
    }

    /// Whether every event this program emits is among them, so that a
    /// topic of one of its own families which is not is a mistake. False
    /// for a catalog that describes no events.
    fn knows_events(&self) -> bool {
        false
    }

    /// What the blueprints this catalog can see say on the topics
    /// `pattern` takes, in their `emit` steps. `None` means it cannot
    /// look, and the checker holds a handler to nothing.
    fn said(&self, _pattern: &str) -> Option<Said> {
        None
    }

    /// Every topic those blueprints say something on.
    fn said_topics(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A catalog that knows nothing: syntax and structure are still checked, but
/// actions are not.
pub struct NoCatalog;

impl Catalog for NoCatalog {
    fn action(&self, _name: &str) -> Option<&ActionSpec> {
        None
    }

    fn action_names(&self) -> Vec<&str> {
        Vec::new()
    }

    fn is_complete(&self) -> bool {
        false
    }
}

/// A catalog backed by a list of specs.
#[derive(Clone, Default)]
pub struct ActionList {
    specs: Vec<ActionSpec>,
}

impl ActionList {
    pub fn new(specs: Vec<ActionSpec>) -> Self {
        Self { specs }
    }

    pub fn push(&mut self, spec: ActionSpec) {
        self.specs.retain(|existing| existing.name != spec.name);
        self.specs.push(spec);
    }

    pub fn specs(&self) -> &[ActionSpec] {
        &self.specs
    }
}

impl Catalog for ActionList {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.specs.iter().find(|spec| spec.name == name)
    }

    fn action_names(&self) -> Vec<&str> {
        self.specs.iter().map(|spec| spec.name.as_str()).collect()
    }
}

/// Actions that are part of the language itself, because they act on the run
/// rather than on the outside world.
pub fn intrinsics() -> Vec<ActionSpec> {
    use ValueType::*;
    asked::shown(vec![
        ActionSpec::new("wait", "Pause for a while, or until an event arrives.")
            .arg("for", Duration, "how long to pause")
            .arg("event", String, "a topic pattern to wait for instead")
            .arg(
                "ended",
                List,
                "what this instance started, each by the name it was started under (`as`): waited for until none \
                 of them is alive, however many there are and in whatever order they end. For things spawned side \
                 by side, waited for together",
            )
            .arg("where", String, "an expression over `event` that must hold")
            .arg("timeout", Duration, "give up waiting for the event after this long")
            .arg(
                "scope",
                String,
                "whose events it hears: `project`, its own project's and the factory's own, which are no project's (the usual); or `fleet`, every project's",
            )
            .arg(
                "say",
                Object,
                "an event to say once the wait is listening, as `emit` says one: `topic`, and what it carries. For \
                 asking something and waiting for the answer, which cannot then come too soon to be heard",
            )
            .within(
                "say",
                &[("topic", "string", "where it is published; whatever else is written is what the event carries")],
            )
            .output("timed_out", "true when the timeout passed before the event")
            .output("event", "the event that arrived")
            .output("ended", "with `ended`: how each ended, by its name; nothing for one that was never started")
            .output("gave", "with `ended`: what each gave out, in the order they are named; nothing for one that never ran")
            .idempotent(),
        ActionSpec::new("emit", "Publish an event for anything listening.")
            .required("topic", String, "where to publish, e.g. `build.slow`")
            .open()
            .idempotent(),
        ActionSpec::new("set", "Change the instance's durable state.").open().idempotent(),
        ActionSpec::new(
            "keep",
            "Keep something in a list of the instance's state, once: an entry that is the same is replaced, and \
             kept twice it is there once.",
        )
        .required("in", String, "the list it is kept in: a key of the state that is declared as a list")
        .required("what", Any, "the entry: a word, a number, or a mapping")
        .arg(
            "by",
            String,
            "the field an entry is the same by, where entries are mappings: one whose field is what this \
             entry's is, is this entry. Left out, an entry is the same when all of it is",
        )
        .output("kept", "how many the list holds after it")
        .output("was", "whether an entry that is the same was there before")
        .idempotent(),
        ActionSpec::new(
            "drop",
            "Take out of a list of the instance's state every entry that is the one said; none being there is \
             no failure.",
        )
        .required("from", String, "the list it is taken out of: a key of the state that is declared as a list")
        .required("is", Any, "what an entry is, or with `by`, what that field of it is")
        .arg(
            "by",
            String,
            "the field of an entry that is looked at, where entries are mappings; an entry that is no mapping \
             is looked at whole. Left out, an entry is dropped when all of it is what `is` says",
        )
        .output("dropped", "how many were taken out")
        .output("left", "how many the list holds after it")
        .idempotent(),
        ActionSpec::new(
            "measure",
            "Keep a measurement of your own: that something happened, whether it went well, and a number for it.",
        )
        .required("name", String, "what it is called, such as `checkout.done`; not a name the system measures under")
        .arg("value", Number, "a number for it: how long it took, how much it came to (0 if left out)")
        .arg("ok", Bool, "whether it went well (true if left out)")
        .arg("labels", Object, "names and values to tell this from others by, beside those it runs under")
        .idempotent(),
        ActionSpec::new("end", "Stop this run now.")
            .arg("status", String, "`ok` (default) or `failed`")
            .arg("reason", String, "why")
            .idempotent(),
        ActionSpec::new("fail", "Fail this step on purpose.").arg("message", String, "why").idempotent(),
        ActionSpec::new("noop", "Do nothing.").idempotent(),
        ActionSpec::new("log", "Write a line to the run log.")
            .required("message", String, "the line")
            .idempotent(),
        ActionSpec::new("stop", "Stop an instance and everything it started.")
            .arg("instance", String, "an instance id, or the key of a child; this instance if omitted")
            .arg(
                "missing_ok",
                Bool,
                "with the key of a child: nothing alive under this instance being called that is no failure, and `stopped` is then false",
            )
            .idempotent(),
    ])
}

pub fn intrinsic(name: &str) -> Option<ActionSpec> {
    intrinsics().into_iter().find(|spec| spec.name == name)
}
