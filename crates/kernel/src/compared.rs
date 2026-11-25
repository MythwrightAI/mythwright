//! A comparison, as it is asked for, kept and shown.
//!
//! One blueprint started several ways, side by side, is the runner's
//! doing. What one is (its ways, what each starts and hands, what is
//! counted of its things) is read by the runner that does it, the node
//! that is asked for it and whatever shows it, so its shapes are here
//! with the few things that are a function of them alone: which way has
//! had least of its share, what a counted event came to, and what is
//! wrong with a count as it is written. Nothing here says which way is
//! better, and nothing here does anything.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Event, Fields};

pub mod rolled;

/// The label that says which comparison something was started under.
pub const COMPARED: &str = "compared";

/// The label that says which of its ways.
pub const WAY: &str = "way";

/// The label that says which text was run: the start of its revision. A
/// way whose text is another by now is counted apart from what the text
/// before it did.
pub const TEXT: &str = "text";

/// A text a way has run, as it is shown.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ran {
    /// What measurements under it carry as `text`.
    pub text: String,
    pub revision: String,
    /// What the text calls its own version, where it says.
    #[serde(default)]
    pub version: Option<String>,
    /// When the way first ran it.
    pub since: i64,
    /// Where the way reads other words in place of a file that is
    /// linked: for each such file, the revision of the words read with
    /// this text. Other words are another column.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prompts: BTreeMap<String, String>,
}

fn one() -> u32 {
    1
}

fn is_none(count: &u64) -> bool {
    *count == 0
}

/// One of the ways a comparison has the work done.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Way {
    pub name: String,
    /// Its part of the things: against the shares of the other ways.
    #[serde(default = "one")]
    pub share: u32,
    /// The blueprint started in place of the one the step names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blueprint: Option<String>,
    /// The revision of that blueprint's text when the comparison began.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// The version of what it starts that it is held to, where one is
    /// said: the text that calls itself that, as the factory kept it
    /// (of those running trees are held to or can be put back on), whatever is under the name since and
    /// however texts are come by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The text it starts, as whoever started the comparison sent it:
    /// for a text that has no name where the work is (what ships under a
    /// name a project's fork stands in for, a text not saved). It is
    /// taken as the comparison begins, kept by its revision as any way's
    /// own text is, and is never kept here or handed out.
    #[serde(default, skip_serializing)]
    pub source: Option<String>,
    /// Its text was sent: it is that text and no other, however texts
    /// are come by, as one held to a version is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sent: bool,
    /// What it is handed, over what the step hands.
    #[serde(default, skip_serializing_if = "Fields::is_empty")]
    pub with: Fields,
    /// The files it reads in place of ones a step links, each by the
    /// name the linked file has in the library: two prompts for one step,
    /// with nothing else changed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prompts: BTreeMap<String, Reads>,
    /// How many things have gone this way.
    #[serde(default)]
    pub went: u64,
    /// How many had gone this way when the shares were last set
    /// ([`rolled`]): what has gone since is what is held to them, so that a
    /// share raised gives this way its part of what comes next, and not
    /// everything until it has caught up with what went before.
    #[serde(default, skip_serializing_if = "is_none")]
    pub before: u64,
    /// The texts it has run, the one it runs now last: one where texts
    /// are held, and each its file came to be where they are followed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub texts: Vec<Ran>,
}

/// The file a way reads in place of one a step links.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Reads {
    /// What it is called in the library (`prompts/strict.md`).
    pub name: String,
    /// The revision of its words: as they were when the comparison began,
    /// and where texts are followed the last that fitted since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// What every step that links the file it stands in for hands, which
    /// is what its blanks may be.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handed: Vec<String>,
}

/// Said as the file's name alone by whoever starts a comparison, and kept
/// with what was found of it.
impl<'de> Deserialize<'de> for Reads {
    fn deserialize<D: serde::Deserializer<'de>>(from: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Said {
            Name(String),
            Kept {
                name: String,
                #[serde(default)]
                revision: Option<String>,
                #[serde(default)]
                handed: Vec<String>,
            },
        }
        Ok(match Said::deserialize(from)? {
            Said::Name(name) => Reads { name, ..Default::default() },
            Said::Kept { name, revision, handed } => Reads { name, revision, handed },
        })
    }
}

/// A comparison as it is kept and shown.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub name: String,
    #[serde(default)]
    pub project: Option<String>,
    /// The blueprint whose starting it is about.
    pub of: String,
    /// The param that says which thing a start is for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per: Option<String>,
    /// Blueprints that carry a thing's way when started for it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub also: Vec<String>,
    /// The param that says which thing a start of one of `also` is for,
    /// where it is another than `per`: what the work of a thing is known
    /// by where the thing's way was given to something else of it (a
    /// piece of work on an item, and the item it then goes on as).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub also_per: Option<String>,
    pub ways: Vec<Way>,
    /// The events about its things that are counted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counts: Vec<Counted>,
    /// Who started it, as an actor, and what they held then.
    pub by: String,
    pub held: Vec<String>,
    pub started: i64,
    #[serde(default)]
    pub ended: Option<i64>,
    /// Once ended: the way everything goes from then on, if one was said.
    #[serde(default)]
    pub goes: Option<String>,
    /// Its ways are named by the start ([`named`]): it speaks only for a
    /// start that names it and one of its ways, gives that start the way
    /// it names whatever the shares, and is not "the one in force" for
    /// its blueprint, so any number of such comparisons of one blueprint
    /// may be in force side by side.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub asked: bool,
    /// Once ended, and until what was said before it ended is counted:
    /// the place among the journals it ended at, as a place is kept.
    /// What was said up to there is counted under it, and nothing after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<Value>,
    /// The rule its shares change by, where one was given ([`rolled`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<rolled::Rule>,
    /// What the rule has read since the shares were last set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolling: Option<rolled::Rolling>,
    /// Each time its shares were said again, by the rule or by somebody,
    /// and why: the oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared: Vec<rolled::Shared>,
}

impl Comparison {
    /// Whether it still says how anything is started.
    pub fn in_force(&self) -> bool {
        self.ended.is_none() || self.goes.is_some()
    }

    /// Whether it says that any of what it counts is a thing being
    /// accepted ([`Counted::accepts`]).
    pub fn accepts(&self) -> bool {
        self.counts.iter().any(|count| !count.accepts.is_empty())
    }

    /// Its way called `name`, where it has one.
    pub fn way(&self, name: &str) -> Option<&Way> {
        self.ways.iter().find(|way| way.name == name)
    }
}

/// What is asked for when a comparison is started.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Compare {
    pub name: String,
    #[serde(default)]
    pub project: Option<String>,
    pub of: String,
    #[serde(default)]
    pub per: Option<String>,
    #[serde(default)]
    pub also: Vec<String>,
    #[serde(default)]
    pub also_per: Option<String>,
    pub ways: Vec<Way>,
    /// The events about its things that are to be counted.
    #[serde(default)]
    pub counts: Vec<Counted>,
    /// Its ways are named by the start ([`Comparison::asked`]).
    #[serde(default)]
    pub asked: bool,
    /// A rule its shares change by ([`rolled`]).
    #[serde(default)]
    pub rule: Option<rolled::Rule>,
}

/// The params a start names an asked comparison and one of its ways by:
/// the names of the two labels, so that what a start is handed and what
/// it is measured with read alike.
pub const NAMED: [&str; 2] = [COMPARED, WAY];

/// The comparison and the way a start's params name, where both are
/// said as text that is something: a start that says neither, or only
/// one, names nothing and is not an asked comparison's.
pub fn named(args: &Fields) -> Option<(String, String)> {
    let said = |name: &str| args.get(name).and_then(Value::as_str).filter(|text| !text.is_empty());
    Some((said(COMPARED)?.to_string(), said(WAY)?.to_string()))
}

/// Which of `one`'s ways a start of `of` in `project` that names `one`
/// (called `name`) and its way `way` is given, or why it is left as it is
/// written: there is no asked comparison of that name for that blueprint
/// here, it has ended, or it has no such way. `one` is the comparison
/// called `name`, where there is one.
pub fn asked_way(
    one: Option<&Comparison>,
    name: &str,
    of: &str,
    project: Option<&str>,
    way: &str,
) -> Result<usize, String> {
    let here = |one: &&Comparison| one.asked && one.of == of && one.project.as_deref() == project;
    let Some(one) = one.filter(here) else {
        return Err(format!(
            "this start of `{of}` names the comparison `{name}`, and no comparison of that name has its ways \
             named by the start of `{of}` here; it was started as it is written"
        ));
    };
    if one.ended.is_some() {
        return Err(format!(
            "this start of `{of}` names the comparison `{name}`, which has ended; it was started as it is written"
        ));
    }
    one.ways.iter().position(|kept| kept.name == way).ok_or_else(|| {
        let ways: Vec<&str> = one.ways.iter().map(|kept| kept.name.as_str()).collect();
        format!(
            "this start of `{of}` names the way `{way}` of `{name}`, which has no such way (its ways are {}); \
             it was started as it is written",
            ways.join(", ")
        )
    })
}

/// What is wrong with a thing given the way `way` of `name` by a start
/// that names it, where the thing went `before` already: a thing keeps
/// to the one way it went, and is counted once, under it.
pub fn kept_to(name: &str, thing: &str, way: &str, before: Option<&str>) -> Option<String> {
    let before = before.filter(|before| *before != way)?;
    Some(format!(
        "`{thing}` went the way `{before}` of `{name}`, and this start names `{way}`; a thing keeps to the \
         way it went, so it was started as it is written"
    ))
}

/// What is wrong with asking again for the asked comparison `kept`, as
/// a blueprint that starts one asks each time it is done: it is answered
/// as it is, with the ways that are new added, where what is asked is of
/// the same blueprint and project, says the same of what a thing is, and
/// says nothing other of a way it has already. Nothing, where it can be.
/// (One that has ended is answered as it is where nothing new is asked
/// of it, and has nothing added: that is whoever asks's to say.)
pub fn again(kept: &Comparison, asked: &Compare) -> Option<String> {
    let name = &kept.name;
    if !kept.asked || !asked.asked {
        return Some(format!(
            "there has already been a comparison called `{name}`, and what was measured under that name is \
             still kept; call this one something else"
        ));
    }
    let project = asked.project.as_deref().filter(|project| !project.is_empty());
    let per = asked.per.as_deref().filter(|per| !per.is_empty());
    let also_per = asked.also_per.as_deref().filter(|per| !per.is_empty());
    if kept.also_per.as_deref().zip(also_per).is_some_and(|(kept, asked)| kept != asked) {
        return Some(format!(
            "the comparison `{name}` tells what else is started for a thing by `{}`; asked again it is \
             asked the same, or called something else",
            kept.also_per.as_deref().unwrap_or_default()
        ));
    }
    if kept.of != asked.of || kept.project.as_deref() != project || kept.per.as_deref() != per {
        return Some(format!(
            "the comparison `{name}` is of `{}`{}, with a thing {}; asked again it is asked the same, or \
             called something else",
            kept.of,
            kept.project.as_deref().map(|project| format!(" in {project}")).unwrap_or_default(),
            kept.per.as_deref().map_or("by where it is started".to_string(), |per| format!("by `{per}`")),
        ));
    }
    for way in &asked.ways {
        let Some(there) = kept.way(&way.name) else { continue };
        let blueprint = way.blueprint.as_ref().filter(|blueprint| **blueprint != kept.of);
        let theirs = there.blueprint.as_ref().filter(|blueprint| **blueprint != kept.of);
        let prompts = |reads: &BTreeMap<String, Reads>| -> Vec<(String, String)> {
            reads.iter().map(|(linked, reads)| (linked.clone(), reads.name.clone())).collect()
        };
        let same = (blueprint == theirs || way.source.is_some() && there.sent)
            && way.version == there.version
            && way.with == there.with
            && prompts(&way.prompts) == prompts(&there.prompts);
        if !same {
            return Some(format!(
                "the comparison `{name}` has a way `{}` already, and this asks for one of that name that \
                 starts or hands something else; call it something else",
                way.name
            ));
        }
    }
    None
}
/// Whether `name` can be what a comparison or a way is called: it is a
/// label's value and part of a command, so a word.
pub fn word(name: &str) -> bool {
    let mut letters = name.chars();
    letters.next().is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && name.len() <= 40
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// The way that has had least of its share since the shares were last
/// set: the first of those, as listed.
pub fn next(ways: &[Way]) -> usize {
    let since = |way: &Way| way.went.saturating_sub(way.before);
    let mut least = 0;
    for (at, way) in ways.iter().enumerate() {
        // went / share, compared without dividing.
        let (a, b) = (since(way) * u64::from(ways[least].share), since(&ways[least]) * u64::from(way.share));
        if a < b {
            least = at;
        }
    }
    least
}

/// What a thing being given to a text of its way is counted as, with no
/// word for what it came to: the one count every comparison has, and so
/// not a name somebody's own count may have.
pub const GIVEN: &str = "given";

/// What a thing being accepted is counted as, once for each thing,
/// however often it is said of it: what is added up under a way is also
/// divided by this. Not a name somebody's own count may have.
pub const ACCEPTED: &str = "accepted";

/// Whether a counted event that came to `came` says its thing was
/// accepted, by what its count `accepts`: a word of it is what the event
/// came to, or the last of what it came to where several fields said
/// (`review>done` is accepted by `done`). A count that accepts nothing
/// says nothing of it, and neither does an event that came to nothing.
pub fn accepted(accepts: &[String], came: &str) -> bool {
    let last = came.rsplit('>').next().unwrap_or_default();
    !came.is_empty() && accepts.iter().any(|word| !word.is_empty() && (word == came || word == last))
}

/// One kind of event a comparison counts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counted {
    /// What the count is called: several kinds of event may be one count.
    pub name: String,
    /// The event's topic, whole.
    pub topic: String,
    /// The field of the event that says which thing it is about: a key of
    /// its data, or a dotted path through it ([`at`]); one that holds a
    /// list says several, each counted ([`Counted::things`]).
    pub thing: String,
    /// The fields whose values say what it came to, joined with `>` where
    /// there are several, each read as `thing` is. With none, the last
    /// word of the topic says.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by: Vec<String>,
    /// What it came to that says the thing was accepted ([`accepted`]):
    /// the work it was given was taken, as whoever starts the comparison
    /// says (a review that passed; a card moved to its last column).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepts: Vec<String>,
    /// The fields that make an event of this kind one thing that happened
    /// to its thing, each read as `thing` is: one that says the same of
    /// each as one counted before is not counted again ([`Counted::once_by`]).
    /// So something said again where it is done again after a stop (what a
    /// blueprint that keeps nothing says) is counted once. With none, every
    /// event is counted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub once: Vec<String>,
}

/// The field of an event's data at `path`: the key as it is written
/// where the data has one of that name, and else the dotted path through
/// it (`chose.verdict`), so that an event that carries the row it is
/// about can be counted by one of its cells.
pub fn at<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    data.get(path).or_else(|| {
        path.contains('.').then(|| path.split('.').try_fold(data, |within, key| within.get(key))).flatten()
    })
}

/// A value of an event as it is counted by: text as it is, anything else
/// as it is written.
fn said(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        None | Some(Value::Null) => String::new(),
        Some(other) => other.to_string(),
    }
}

impl Counted {
    /// Which thing an event of this kind is about; nothing where it does not say.
    pub fn about(&self, event: &Event) -> String {
        said(at(&event.data, &self.thing))
    }

    /// Which things an event of this kind is about: each the field holds
    /// where it is a list (one said of several things, which is counted for
    /// each), and else the one it says; none where it says nothing.
    pub fn things(&self, event: &Event) -> Vec<String> {
        let mut all: Vec<String> = match at(&event.data, &self.thing) {
            Some(Value::Array(several)) => several.iter().map(|one| said(Some(one))).collect(),
            one => vec![said(one)],
        };
        all.retain(|one| !one.is_empty());
        all.dedup();
        all
    }

    /// What an event of this kind came to.
    pub fn came(&self, event: &Event) -> String {
        match self.by.is_empty() {
            true => event.topic.rsplit('.').next().unwrap_or_default().to_string(),
            false => self.by.iter().map(|field| said(at(&event.data, field))).collect::<Vec<_>>().join(">"),
        }
    }

    /// What makes an event of this kind one thing that happened, where the
    /// count says (`once`): the values of those fields, each as it is
    /// counted by. Nothing where every event is counted; an event that
    /// says none of them is one of its own, and is counted.
    pub fn once_by(&self, event: &Event) -> Option<String> {
        if self.once.is_empty() {
            return None;
        }
        let values: Vec<String> = self.once.iter().map(|field| said(at(&event.data, field))).collect();
        values.iter().any(|value| !value.is_empty()).then(|| values.join("\n"))
    }

    /// What is wrong with it as it is written, if anything is: what it is
    /// called, the event it is of, and the field that says the thing.
    pub fn wrong(&self) -> Option<String> {
        let word = |word: &str| {
            !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        };
        if !word(&self.name) {
            return Some(format!(
                "`{}` cannot be what a count is called: it is a lowercase word, such as `moved`",
                self.name
            ));
        }
        if self.name == GIVEN {
            return Some(format!(
                "a count cannot be called `{GIVEN}`: that is how many things each way was given, which is \
                 always counted"
            ));
        }
        if self.name == ACCEPTED {
            return Some(format!(
                "a count cannot be called `{ACCEPTED}`: that is how many things of each way were accepted, \
                 which is counted where a count says what it accepts"
            ));
        }
        if let Some(wrong) = self.accepts.iter().find(|word| word.trim().is_empty() || word.contains('>')) {
            return Some(format!(
                "the count `{}` accepts `{wrong}`, which is not what an event can come to; say a value of \
                 the field it is counted by, such as `passed`",
                self.name
            ));
        }
        if !is_topic(&self.topic) {
            return Some(format!(
                "the count `{}` is of `{}`, which is not an event's topic; one is lowercase words joined \
                 by dots, such as `shop.done`",
                self.name, self.topic
            ));
        }
        if self.thing.is_empty() {
            return Some(format!(
                "the count `{}` does not say which field of `{}` says the thing an event is about",
                self.name, self.topic
            ));
        }
        if self.once.iter().any(|field| field.trim().is_empty()) {
            return Some(format!(
                "the count `{}` says an event is one by a field with no name; say the field of `{}` that \
                 makes one, such as `by`",
                self.name, self.topic
            ));
        }
        None
    }
}

/// Whether `topic` is written as a topic is: lowercase words joined by dots.
fn is_topic(topic: &str) -> bool {
    let word = |word: &str| {
        !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    topic.contains('.') && topic.split('.').all(word)
}
