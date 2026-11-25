//! What a handler hears: its topic held to the events there are, and what
//! it reads of an event held to what such an event carries.
//!
//! Two kinds of topic. One of a family this program keeps for itself
//! (`board.item.moved`) is held to what the program says it emits: the
//! catalog describes every one. Any other is something blueprints say to
//! each other with `emit`, and what an `emit` step writes is what the
//! event carries; the blueprints that can be seen are read for it.
//!
//! A handler on a group of topics (`board.item.*`) may read what any of
//! them carries. Such a handler usually tells its events apart by
//! `event.topic` and reads of each what that one has, so asking for a
//! field to be in all of them would refuse what is right; one that is in
//! none of them is wrong whichever event comes.

use std::collections::BTreeSet;

use crate::diag::did_you_mean;
use crate::events::{Said, emitted, matches};
use crate::model::{Trigger, TriggerSource};
use crate::syntax::Span;
use crate::validate::{RESERVED_TOPICS, Validator, is_pattern};

/// The faults that are about what a handler hears. A text that was right
/// when it was applied can come to have these without having changed: the
/// events did.
pub const HEARD: &[&str] = &["unknown-event", "unheard-event", "unknown-field"];

/// What the events of one handler carry.
pub(crate) struct Heard {
    /// The topic, or the group of them, as the handler wrote it.
    topic: String,
    /// Every field one of its events may carry. `None` when that cannot
    /// be said: one of them carries what is not listed.
    fields: Option<BTreeSet<String>>,
}

/// Where a field of an event was named.
#[derive(Clone, Copy)]
pub(crate) enum Where {
    /// Under `for:`, as what an event must have to be this handler's.
    Address,
    /// As `event.data.<field>`.
    Read,
}

impl Validator<'_> {
    /// Checks what a handler listens to, and says what its events carry
    /// when that is known.
    pub(crate) fn heard(&mut self, trigger: &Trigger) -> Option<Heard> {
        let TriggerSource::Event(topic) = &trigger.source else { return None };
        if !is_pattern(topic) {
            return None;
        }
        let family = topic.split('.').next().unwrap_or_default();
        // A group that takes any family takes what nobody can list.
        if family == "*" || family == "**" {
            return None;
        }
        let fields = match RESERVED_TOPICS.contains(&family) {
            true if trigger.outside => {
                self.error(
                    "outside-ours",
                    format!(
                        "`{topic}` is the system's own to say, and is never said from outside; leave `outside` out"
                    ),
                    trigger.span,
                );
                return None;
            }
            true => self.heard_of_ours(topic, family, trigger.span)?,
            // Said from outside, by whoever holds a key for it: no
            // blueprint has to say it, and what it carries is the
            // sender's to choose.
            false if trigger.outside => None,
            false => self.heard_of_blueprints(topic, trigger.span)?,
        };
        let heard = Heard { topic: topic.clone(), fields };
        // A field inside one is carried where the one it is inside is.
        for field in trigger.address.keys().map(|field| field.split('.').next().unwrap_or(field)) {
            self.carried(&heard, field, Where::Address, trigger.span);
        }
        Some(heard)
    }

    /// An event of this program's own.
    fn heard_of_ours(&mut self, topic: &str, family: &str, span: Span) -> Option<Option<BTreeSet<String>>> {
        if !self.catalog.knows_events() {
            return None;
        }
        let events = self.catalog.events();
        let taken: Vec<_> = events.iter().filter(|spec| matches(topic, &spec.topic)).collect();
        let group = topic.split('.').any(|word| word == "*" || word == "**");
        if taken.is_empty() {
            let near: Vec<&str> = events
                .iter()
                .map(|spec| spec.topic.as_str())
                .filter(|known| known.split('.').next() == Some(family))
                .collect();
            let hint = match did_you_mean(topic, events.iter().map(|spec| spec.topic.as_str())) {
                Some(close) => format!("; did you mean `{close}`?"),
                None if near.is_empty() => format!("; nothing is emitted under `{family}.`"),
                None => format!("; under `{family}.` there are: {}", near.join(", ")),
            };
            let message = match group {
                true => format!("`{topic}` takes no event there is{hint}"),
                false => format!("there is no event called `{topic}`{hint}"),
            };
            self.error("unknown-event", message, span);
            return None;
        }
        if taken.iter().all(|spec| !spec.kept) {
            self.error(
                "unheard-event",
                format!(
                    "`{topic}` is passed on to whoever is watching and is not kept, so no handler hears it"
                ),
                span,
            );
            return None;
        }
        if taken.iter().any(|spec| spec.open) {
            return Some(None);
        }
        let fields = taken.iter().flat_map(|spec| spec.fields.iter().map(|field| field.name.clone()));
        Some(Some(fields.collect()))
    }

    /// Something blueprints say to each other.
    fn heard_of_blueprints(&mut self, topic: &str, span: Span) -> Option<Option<BTreeSet<String>>> {
        let mut said = Said::default();
        for (written, one) in emitted(self.blueprint) {
            if matches(topic, &written) {
                said.add(&one);
            }
        }
        // Without the other blueprints nothing can be said: one of them
        // may write what this one does not.
        said.add(&self.catalog.said(topic)?);
        if said.by.is_empty() {
            // What was probably meant may be one of the program's own
            // events with a word misspelt (`bord.item.moved`): a topic no
            // family of its own takes is looked for among those too.
            let known = self.catalog.said_topics();
            let ours = self.catalog.events();
            let all = known.iter().map(String::as_str).chain(ours.iter().map(|spec| spec.topic.as_str()));
            let hint =
                did_you_mean(topic, all).map(|close| format!(" Did you mean `{close}`?")).unwrap_or_default();
            self.warning(
                "unsaid-topic",
                format!(
                    "nothing that can be seen here says `{topic}`: no blueprint emits it. If something \
                     outside does, say so with `outside: true`; if not, this handler never runs.{hint}"
                ),
                span,
            );
            return None;
        }
        Some((!said.open).then_some(said.fields))
    }

    /// Holds a field named of an event to what the handler's events carry.
    pub(crate) fn carried(&mut self, heard: &Heard, field: &str, at: Where, span: Span) {
        let Some(fields) = &heard.fields else { return };
        if fields.contains(field) {
            return;
        }
        let topic = &heard.topic;
        let hint = did_you_mean(field, fields.iter().map(String::as_str))
            .map(|close| format!("; did you mean `{close}`?"))
            .unwrap_or_default();
        let has = match fields.is_empty() {
            true => "it carries nothing".to_string(),
            false => format!("it carries: {}", fields.iter().cloned().collect::<Vec<_>>().join(", ")),
        };
        let message = match at {
            Where::Address => format!(
                "`for:` names `{field}`, which `{topic}` does not carry, so no event would ever be this \
                 handler's{hint}; {has}"
            ),
            Where::Read => format!("`event.data.{field}` is not something `{topic}` carries{hint}; {has}"),
        };
        self.error("unknown-field", message, span);
    }
}
