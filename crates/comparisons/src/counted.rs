//! What came of the things a comparison is of.
//!
//! What is measured under a way ([`crate::compared`]) is what the work
//! did: its steps, its runs, an agent's turns. What somebody decides by is
//! what came of it: whether the thing reached its end, how long that took,
//! how often it came back. That is not said by the work but by whatever
//! looks after the thing, as events about it; and which events those are
//! depends on what the thing is, which the conductor does not know.
//!
//! So whoever starts a comparison says which events are counted
//! ([`Counted`]): the event's topic, the field of it that says which
//! thing it is about, and the fields whose values say what it came to. An
//! event that is about a thing the comparison gave a way is measured,
//! under that way and the text the thing last ran, as
//! `system.comparison.came`: with what the count is called (`counted`),
//! what it came to (`came`), and for how long, the time since the thing
//! was given its way. Nothing here knows what a thing is or what any of
//! it means, and nothing is added up into a figure of merit: they are
//! counts and times, read beside each other.
//!
//! Beside what is asked for, every thing is counted once as given
//! ([`GIVEN`]) to each text of its way it is started under: how many
//! things a way had, by the text, which is what a total is divided by.
//!
//! Whoever starts it must be somebody who may be told of those events
//! ([`conductor::Engine::may_hear`]); one said of something kept from them is passed
//! by. The events are followed in the journal from a place that is kept,
//! so a daemon that was away counts what was said meanwhile. The place is
//! taken as a comparison is started, before whoever started it is
//! answered, and what is counted having changed is never lost on whoever
//! follows: every event said once a comparison has been started is
//! counted, and none said once it has been ended. One that ends keeps
//! where it ended ([`Comparison::until`]), and what was said before that
//! which whoever counts had not come to is counted under it then, from
//! the journal: ending a comparison as something is said of its thing
//! (a blueprint that ends it on hearing that) loses nothing said first.
//!
//! A count may say what of what it comes to is its thing being accepted
//! (`accepts`, [`kernel::compared::accepted`]): the work the thing was
//! given was taken. A thing is counted as accepted once, however often
//! that is said of it ([`ACCEPTED`]), and what is added up under a way is
//! laid beside how many of its things were ([`crate::beside`]). A thing
//! may be written down as one of a way's with nothing started for it
//! (`compare.went`): what something else goes on with of the way's work.
//!
//! And a count may say what makes an event one thing that happened to its
//! thing (`once`, [`Counted::once_by`]): what is said again of the same
//! is not counted again ([`ONCE`]), so that something a blueprint that
//! keeps nothing says, said a second time where it was done again after a
//! stop, is counted once.

use kernel::{Error, Event, Principal, Result};
use serde_json::Value;

pub use kernel::compared::{ACCEPTED, Counted, GIVEN};

use crate::Comparisons;
use crate::compared::{COMPARED, Comparison, TEXT, WAY, WENT};

/// The measurement a counted event is: one for each event about a thing.
pub const CAME: &str = "system.comparison.came";

/// Where the place in the journal that events are counted from is kept.
pub(crate) const NAMESPACE: &str = "compared.counted";

/// Where what made each event counted once one ([`Counted::once_by`]) is
/// kept, by the comparison, the thing and the topic (a count may be of
/// several, and one thing done of each is another): what is said of it
/// again is not counted again.
pub(crate) const ONCE: &str = "compared.once";
const CURSOR: &str = "cursor";

/// The place events are counted from, in each journal, where one is
/// kept: a number alone is what a program from before kept.
async fn place(journal: &journal::Journal) -> Option<journal::Places> {
    let kept = journal.kv_get(NAMESPACE, CURSOR).await.ok().flatten()?;
    journal::Places::from_kept(&kept)
}

impl Comparisons {
    /// What is wrong with what a comparison is to count, as `by` asks for
    /// it, if anything is.
    pub(crate) fn may_count(&self, counts: &[Counted], by: &Principal) -> Result<()> {
        for one in counts {
            if let Some(wrong) = one.wrong() {
                return Err(Error::invalid(wrong));
            }
            if !self.engine.may_hear(by, &one.topic) {
                return Err(Error::denied(format!(
                    "the count `{}` is of `{}`, which you may not be told of, so it cannot be counted for you",
                    one.name, one.topic
                )));
            }
        }
        Ok(())
    }

    /// The comparisons that count events now, those that ended and are
    /// still owed what was said before they ended ([`Comparison::until`]),
    /// and the topics either counts.
    async fn counting(&self) -> (Vec<Comparison>, Vec<Comparison>, Vec<String>) {
        let all = self.comparisons(None).await;
        let (counting, owed): (Vec<Comparison>, Vec<Comparison>) = all
            .into_iter()
            .filter(|one| (one.ended.is_none() || one.until.is_some()) && !one.counts.is_empty())
            .partition(|one| one.ended.is_none());
        let mut topics: Vec<String> = counting
            .iter()
            .chain(&owed)
            .flat_map(|one| one.counts.iter().map(|count| count.topic.clone()))
            .collect();
        topics.sort();
        topics.dedup();
        (counting, owed, topics)
    }

    /// For each journal by its name, the place events are counted from
    /// there, where a comparison that counts goes on counting any of
    /// `topics`: what is said after it is still to be counted, and is not
    /// taken out of the journal (`conductor::letgo`). Nothing where none
    /// counts any of them.
    pub async fn counted_from(&self, topics: &[&str]) -> std::collections::BTreeMap<String, u64> {
        let (_, _, counted) = self.counting().await;
        let journal = self.engine.journal();
        if !counted.iter().any(|topic| topics.contains(&topic.as_str())) {
            return Default::default();
        }
        let from = place(journal).await.unwrap_or_else(|| journal.now());
        journal.every().iter().map(|one| (one.name().to_string(), from.of(one.name()))).collect()
    }

    /// Has events counted for as long as a comparison counts any, and is
    /// what whoever changes what is counted says before they are answered.
    ///
    /// The place events are counted from is taken here, where there is
    /// none yet, and not by whoever follows the journal when it comes to
    /// be run: an event said in the moment after a comparison was started
    /// is after that place. Whoever follows is started where nobody is,
    /// and is told that what is counted has changed in a way that is kept
    /// for it ([`tokio::sync::Notify::notify_one`]): it is as likely in
    /// the middle of an event as waiting for one, and told only where it
    /// waits it went on with what it knew, counting nothing of a
    /// comparison started since and never putting its place away.
    pub(crate) async fn count_from_here(&self) {
        let kept = &self.kept;
        let journal = self.engine.journal();
        {
            // Held with whoever follows as it ends (`count`): it is there
            // to be told, or it is gone and another is started.
            let mut there = kept.counting.lock().await;
            let topics = self.counting().await.2;
            // What is counted is kept, before the place it is counted
            // from is taken: what the runner says of its own steps and
            // runs is otherwise passed on, and could not be read again.
            *kept.keeping.lock().expect("comparisons lock") =
                (!topics.is_empty()).then(|| self.engine.keeps_said(&topics));
            if !topics.is_empty() {
                if place(journal).await.is_none() {
                    let _ = journal.kv_set(NAMESPACE, CURSOR, &journal.now().kept()).await;
                }
                if !*there {
                    *there = true;
                    let this = self.clone();
                    self.engine.spawn(async move { this.count().await });
                }
            }
        }
        kept.changed.notify_one();
    }

    /// Follows the journal for the events comparisons count, from the
    /// place kept, until none counts any.
    async fn count(&self) {
        let journal = self.engine.journal().clone();
        let kept = &self.kept;
        let mut cursor = match place(&journal).await {
            Some(at) => at,
            None => journal.now(),
        };
        loop {
            let (counting, owed, _) = self.counting().await;
            // What was said before each that ended, which this had not come
            // to as it ended, is counted under it.
            for one in &owed {
                self.count_owed(one, &cursor).await;
            }
            let mut topics: Vec<String> =
                counting.iter().flat_map(|one| one.counts.iter().map(|count| count.topic.clone())).collect();
            topics.sort();
            topics.dedup();
            if topics.is_empty() {
                // Nothing is counted: the place is put away, so that what
                // is counted next begins from where it is started. Looked
                // at again with the turn held, so that a comparison
                // started in the moment between is this one's to count,
                // and one started after finds nobody and starts another.
                let mut there = kept.counting.lock().await;
                if !self.counting().await.2.is_empty() {
                    continue;
                }
                let _ = journal.kv_set(NAMESPACE, CURSOR, &Value::Null).await;
                *kept.keeping.lock().expect("comparisons lock") = None;
                *there = false;
                return;
            }
            let mut follow = journal.follow_every(cursor.clone(), topics.join(","));
            loop {
                tokio::select! {
                    // In this order: that what is counted has changed is
                    // taken in before the next event is, so that nothing
                    // said after a comparison was ended is counted under it.
                    biased;
                    () = kept.changed.notified() => break,
                    event = follow.next() => {
                        let Ok(event) = event else {
                            // The journal can be followed no longer: nobody
                            // counts, and whoever next changes what is
                            // counted starts another.
                            *kept.counting.lock().await = false;
                            return;
                        };
                        let counted = self.count_one(&counting, &event).await;
                        if event.is_durable() {
                            cursor = follow.places().clone();
                            // Kept only where something was counted: what
                            // was about nothing here is cheap to read again.
                            if counted {
                                let _ = journal.kv_set(NAMESPACE, CURSOR, &cursor.kept()).await;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Counts one event for each comparison it is about a thing of.
    /// Whether anything was counted.
    async fn count_one(&self, counting: &[Comparison], event: &Event) -> bool {
        let mut any = false;
        for one in counting {
            // Another project's, or kept from whoever started the comparison.
            let elsewhere = match (&one.project, event.data.get("project").and_then(Value::as_str)) {
                (Some(here), Some(there)) => here != there,
                _ => false,
            };
            let kept_from = !self.engine.journal().may_tell_as(&one.by, &one.held, event);
            if elsewhere || kept_from {
                continue;
            }
            // An event said of several things is counted for each.
            let counts = one.counts.iter().filter(|count| count.topic == event.topic);
            let each =
                counts.flat_map(|count| count.things(event).into_iter().map(move |thing| (count, thing)));
            for (count, thing) in each {
                let Some(went) = self.went(&one.name, &thing).await else { continue };
                // What was said of a thing before it was given its way did
                // not come of it. An event from before is met again where
                // the journal is read from a place kept earlier (a daemon
                // that was away), by when the thing has a way.
                if went.at.is_some_and(|at| event.ts < at) {
                    continue;
                }
                // One thing that happened to it, said again (where it was
                // done again after a stop), is counted once.
                if let Some(by) = count.once_by(event) {
                    let key = format!("{}\n{thing}\n{}\n{by}", one.name, event.topic);
                    let journal = self.engine.journal();
                    if journal.kv_get(ONCE, &key).await.ok().flatten().is_some() {
                        continue;
                    }
                    let _ = journal.kv_set(ONCE, &key, &Value::from(event.ts)).await;
                }
                // How long since the thing was given its way.
                let took = (event.ts - went.at.unwrap_or(event.ts)).max(0) as f64;
                // A word or a few, as a label is, however much the fields it is read from hold.
                let came: String = count.came(event).chars().take(blueprint::labels::MAX_VALUE).collect();
                let mut labels = vec![
                    (COMPARED, one.name.as_str()),
                    (WAY, went.way.as_str()),
                    ("counted", count.name.as_str()),
                    ("came", came.as_str()),
                ];
                if let Some(text) = &went.text {
                    labels.push((TEXT, text.as_str()));
                }
                self.engine.journal().measure(CAME, &labels, took, true);
                any = true;
                // What the comparison's rule reads, where it goes by this count.
                if one.rule.as_ref().is_some_and(|rule| rule.count == count.name) {
                    self.roll(&one.name, &thing, &went.way, &count.name, &came, event.ts).await;
                }
                if kernel::compared::accepted(&count.accepts, &came) && went.accepted.is_none() {
                    self.accept(one, &thing, event.ts).await;
                }
            }
        }
        any
    }

    /// Counts `thing` of `one` as accepted, where it was not already: once
    /// for each thing, however often that is said of it, under its way and
    /// the text it last ran, with how long since it was given its way.
    async fn accept(&self, one: &Comparison, thing: &str, at: i64) {
        let _one = self.kept.choosing.lock().await;
        let Some(went) = self.went(&one.name, thing).await else { return };
        if went.accepted.is_some() {
            return;
        }
        let key = format!("{}\n{thing}", one.name);
        let now = crate::compared::Went { accepted: Some(at), ..went.clone() };
        if self.engine.journal().kv_set(WENT, &key, &serde_json::json!(now)).await.is_err() {
            return;
        }
        let took = (at - went.at.unwrap_or(at)).max(0) as f64;
        let mut labels = vec![(COMPARED, one.name.as_str()), (WAY, went.way.as_str()), ("counted", ACCEPTED)];
        if let Some(text) = &went.text {
            labels.push((TEXT, text.as_str()));
        }
        self.engine.journal().measure(CAME, &labels, took, true);
    }

    /// Counts under `one`, which has ended, what was said of its things
    /// from `cursor`, where whoever counts had come to, up to where it
    /// ended; and then that it is owed nothing more.
    async fn count_owed(&self, one: &Comparison, cursor: &journal::Places) {
        let journal = self.engine.journal();
        let topics: Vec<&str> = one.counts.iter().map(|count| count.topic.as_str()).collect();
        let owed = std::slice::from_ref(one);
        let until = one.until.as_ref().and_then(journal::Places::from_kept);
        for each in until.as_ref().map(|_| journal.every()).unwrap_or_default() {
            let Some(until) = &until else { break };
            let (mut from, upto) = (journal.place_in(cursor, &each), journal.place_in(until, &each));
            while from < upto {
                let query = journal::EventQuery {
                    after: from,
                    before: upto.saturating_add(1),
                    topics: topics.join(","),
                    limit: 500,
                    ..Default::default()
                };
                let Ok(read) = each.read(query).await else { break };
                for event in &read {
                    self.count_one(owed, event).await;
                }
                match read.last() {
                    Some(last) if read.len() == 500 => from = last.seq,
                    _ => break,
                }
            }
        }
        let _one = self.kept.choosing.lock().await;
        if let Ok(mut now) = self.comparison(&one.name).await
            && now.until.is_some()
        {
            now.until = None;
            let _ = self.keep_compared(&now).await;
        }
    }
}
