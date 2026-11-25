//! The history of a record: what happened to it, in order, and for good.
//!
//! A blueprint that declares a collection says which events are part of the
//! history of its records, and may say how each is put into words:
//!
//! ```text
//! history:
//!   board.item.moved: "{actor} moved it from {from} to {to}"   # one event, in its own words
//!   board.item.*: ""                                           # a group of them, in the usual words
//! ```
//!
//! A part of a sentence in square brackets is said only when the event has
//! what the part names: `"{actor} moved it from {from} to {to}[, saying:
//! {note}]"` ends at the column for a move nobody said anything about.
//!
//! or, where the usual words will do for all of them, only the groups:
//! `history: [board.item.*, lane.*]`. The more exact of two that fit an
//! event is the one that speaks for it.
//!
//! Every event of the journal is looked at as it happens. One that a
//! collection counts as history, and that names one of its records, is
//! written down against that record: when, who, and what was said of it
//! then. That is all that is ever done to a history. A line is never
//! changed and never taken away, not when the record is, and not when the
//! blueprint later words the same event differently: what it said is what
//! was true of the record then.

use journal::{EventQuery, Journal, Places};
use kernel::{Event, Result, Source};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Value, json};

// How a line is worded is a function of plain values, and the kernel's.
pub use kernel::worded::{NAMES, Rule, as_called, rule_for, rules, said};

use crate::collections::{Collection, Collections, Record};
use crate::identity::Identity;

/// What is said when a line is added to a record's history.
pub const TOPIC: &str = "collection.history";

/// Where the keeper of histories has read to, in the small durable state
/// of the journal that keeps it.
const PLACE: (&str, &str) = ("history", "seq");

/// A line of a record's history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Line {
    /// Where the event is in the journal.
    pub seq: u64,
    pub ts: i64,
    pub topic: String,
    /// Who did it, as the journal has them: `agent:builder`.
    pub actor: Option<String>,
    /// Who did it, by name, as it was then.
    pub who: String,
    /// What was said of it then.
    pub said: String,
    /// What the event said of itself, as it was kept: for whatever shows
    /// a line as more than its sentence (something said on a card, drawn
    /// as what was said and by whom).
    pub data: Value,
}

/// The record of `collection` an event is about, if it names one.
pub fn about(collection: &Collection, data: &Value) -> Option<String> {
    NAMES
        .iter()
        .filter_map(|name| data.get(name).and_then(Value::as_str))
        .find(|id| collection.owns(id))
        .map(str::to_string)
}

/// Whether an event says only that something changed which is not news of
/// the record: where it sits among the others, or a field its collection
/// declared `quiet`, which is there to show what an event of its own has
/// already said.
fn not_news(collection: &Collection, data: &Value) -> bool {
    let Some(changed) = data.get("changed").and_then(Value::as_array) else {
        return false;
    };
    !changed.is_empty()
        && changed.iter().all(|name| {
            name.as_str().is_some_and(|name| {
                name == "position" || collection.field(name).is_some_and(|field| field.quiet)
            })
        })
}

/// What is said of something said on a record that whoever said it marked
/// public: the same, and that it was. A line is kept for good and is read
/// by itself (in a channel, from a terminal), so it says so whatever words
/// its collection has for something said, where those do not name the mark
/// themselves; the usual words say everything the event does, the mark
/// with it.
fn marked(line: String, rule: &Rule, event: &Event) -> String {
    let public = event.topic == crate::records::COMMENTED
        && event.data.get(crate::records::PUBLIC).and_then(Value::as_bool) == Some(true);
    match &rule.says {
        Some(sentence) if public && !sentence.contains("{public}") => format!("{line} (public)"),
        _ => line,
    }
}

/// Who did something, by name.
pub async fn who(identity: &Identity, source: &Source) -> String {
    match source.actor.as_deref().map(|actor| actor.split_once(':').unwrap_or((actor, ""))) {
        // By the name on their account; one that is no longer there is
        // somebody, unless what is left of it is a name.
        Some(("user", id)) => match identity.account(id).await {
            Ok(account) => account.name,
            Err(_) if id.starts_with("acct_") || id.is_empty() => "someone".to_string(),
            Err(_) => id.to_string(),
        },
        // A key is somebody's: by the name on their account, and the name
        // they gave the key.
        Some(("key", id)) => match identity.holder(&format!("key:{id}")).await {
            Ok(Some(key)) => match identity.account(&key.id).await {
                Ok(account) => format!("{} (with the key {})", account.name, key.name),
                Err(_) => format!("the key {}", key.name),
            },
            _ => "a key that is withdrawn".to_string(),
        },
        Some(("system", _)) => "the factory".to_string(),
        Some((_, name)) if !name.is_empty() => name.to_string(),
        Some((actor, _)) => actor.to_string(),
        None if source.instance.is_some() => "a blueprint".to_string(),
        None => "the factory".to_string(),
    }
}

/// A line of a record's history, ready to be written with the news of
/// it.
struct Noted {
    /// The project of the record it is a line of.
    project: String,
    record: String,
    who: String,
    said: String,
    told: Value,
    source: Source,
    /// What the event said, with which blueprint's step said it where
    /// that was found and not said (`crate::through`).
    data: Value,
}

impl Collections {
    /// The journals that may keep what is kept of a record called `id`:
    /// that of each project the id may be of, each once. An id begins
    /// with its project, and a project's own id may hold a dash, so
    /// `shop-2-5` may be `shop`'s or `shop-2`'s: only what is kept says
    /// which, and a line of history is kept by its record whether or not
    /// the record still is.
    fn may_keep(&self, id: &str) -> Vec<Journal> {
        let mut asked: Vec<Journal> = Vec::new();
        let dashes = id.match_indices('-').map(|(at, _)| at);
        for project in dashes.map(|at| &id[..at]).chain(std::iter::once("")) {
            let journal = match project {
                "" => self.factory().clone(),
                project => self.rows(project),
            };
            if !asked.iter().any(|one| one.name() == journal.name()) {
                asked.push(journal);
            }
        }
        asked
    }

    /// The history of a record, oldest first. It is there whether or not
    /// the record still is.
    pub async fn history(&self, record: &str) -> Result<Vec<Line>> {
        let mut lines: Vec<Line> = Vec::new();
        for journal in self.may_keep(record) {
            let record = record.to_string();
            let kept: Vec<Line> = journal
                .db()
                .call(move |connection| {
                connection
                    .prepare("SELECT seq, ts, topic, actor, who, said, data FROM record_history WHERE record = ?1 ORDER BY seq")?
                    .query_map([&record], |row| {
                        Ok(Line {
                            seq: row.get::<_, i64>(0)? as u64,
                            ts: row.get(1)?,
                            topic: row.get(2)?,
                            actor: row.get(3)?,
                            who: row.get(4)?,
                            said: row.get(5)?,
                            data: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(Value::Null),
                        })
                    })?
                    .collect()
                })
                .await?;
            lines.extend(kept);
        }
        lines.sort_by_key(|line| line.seq);
        // What a blueprint's step did is said as the bot is called now.
        if let Some(bot) = self.bot.get().map(|speaker| speaker.name()) {
            for line in &mut lines {
                (line.who, line.said) = as_called(&line.who, &line.said, &line.data, &bot);
            }
        }
        Ok(lines)
    }

    /// The line an event is in the history of the record of `collection`
    /// it is about, ready to be written: nothing, if it is no part of
    /// that collection's history.
    async fn line(&self, collection: &Collection, event: &Event, identity: &Identity) -> Option<Noted> {
        let rule = rule_for(&collection.history, &event.topic)?;
        let record = about(collection, &event.data)?;
        if not_news(collection, &event.data) {
            return None;
        }
        // What a blueprint's step did is the factory's doing, said as its
        // bot: the event says which blueprint, which is kept with the line,
        // and whoever applied it only lent the authority. Where the step was
        // told whose work it did it from (an agent's, `wrote`), that is said
        // after the sentence (`as_called`).
        let bot =
            self.bot.get().map(|speaker| speaker.name()).unwrap_or_else(|| talk::speaker::DEFAULT.into());
        // What a step said for itself says no `through`: it is found from
        // the run that said it (`crate::through`), and kept with the line.
        let mut data = event.data.clone();
        if let (Some(blueprint), Value::Object(said)) =
            (self.said_by.of(event, &self.factory().every()).await, &mut data)
        {
            said.insert("through".into(), json!(blueprint));
        }
        let who = match (kernel::worded::through(&data), kernel::worded::decided(&data)) {
            (Some(_), Some(agent)) => agent.to_string(),
            (Some(_), None) => bot.clone(),
            (None, _) => who(identity, &event.source).await,
        };
        let said = match event.data.get("restored").and_then(Value::as_bool) == Some(true) {
            // Given back from the Trash, and not added: in the words a table
            // gives that (`<topic>.restored`), else in the usual ones.
            true => {
                match collection.history.iter().find(|one| one.topic == format!("{}.restored", event.topic)) {
                    Some(rule) => said(rule, event, &who),
                    None => format!("{who} gave it back from the Trash"),
                }
            }
            false => marked(said(rule, event, &who), rule, event),
        };
        let (who, mut said) = as_called(&who, &said, &data, &bot);
        // A change somebody asked for and another approved says who approved it.
        if let Some(by) = event.data.get("approved_by").and_then(Value::as_str) {
            let by = self::who(identity, &Source::actor(by.to_string())).await;
            said.push_str(&format!(" · approved by {by}"));
        }
        // What was written is said with the news of it, so that whatever
        // follows a record's history has the line and need not read for it.
        let told = json!({
            "project": collection.project, "name": collection.name, "record": record,
            "seq": event.seq, "topic": event.topic, "who": who, "said": said,
        });
        // The line says what the event said, in words: of a record of
        // a table that is kept private it is heard by whoever may
        // read the table, as the event was, and by nobody else.
        let source = Source::actor("system").taking(collection.reads.as_deref());
        Some(Noted { project: collection.project.clone(), record, who, said, told, source, data })
    }

    /// Writes the lines an event came to, each against its record, once:
    /// the same event looked at again changes nothing. A line is written
    /// in the journal that keeps its record's project's rows, with the
    /// news of it, as one write; and where whoever follows the events
    /// for this has read to (`read`: a place in each journal, this event
    /// included, and the journal the event was heard in) is kept in that
    /// write, in that journal's own small durable state: the factory's
    /// own keeps the place in every journal, as it always did, and a
    /// project's keeps the place in itself, for a line of an event of
    /// its own, which is nearly every line it has. So a line of a
    /// project whose journal keeps its records asks nothing of the
    /// factory's. Stopped anywhere, a line that is there was said to be,
    /// and what is read next is what comes after the event. Says whether
    /// anything was written, and whether the place in every journal was
    /// kept.
    async fn note(
        &self,
        event: &Event,
        lines: Vec<Noted>,
        read: Option<(&Places, &str)>,
    ) -> Result<(bool, bool)> {
        let mut by_project: std::collections::BTreeMap<String, Vec<Noted>> = Default::default();
        for line in lines {
            by_project.entry(line.project.clone()).or_default().push(line);
        }
        let (mut wrote, mut placed, mut whole) = (false, false, false);
        for (project, lines) in by_project {
            let journal = self.rows(&project);
            let of_all = journal.name() == self.factory().name();
            let here = read.and_then(|(places, heard)| match journal.name() {
                _ if of_all => Some(places.kept()),
                own if own == heard => Some(json!(places.of(own))),
                _ => None,
            });
            let (seq, ts, topic, actor) =
                (event.seq as i64, event.ts, event.topic.clone(), event.source.actor.clone());
            let written = journal.write(move |writing| {
                let mut wrote = false;
                for Noted { record, who, said, told, source, data, .. } in lines {
                    let data = data.to_string();
                    let written = writing.transaction().execute(
                        "INSERT OR IGNORE INTO record_history (record, seq, ts, topic, actor, who, said, data)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![record, seq, ts, topic, actor, who, said, data],
                    )?;
                    if written == 1 {
                        writing.say(TOPIC, source, told)?;
                        wrote = true;
                    }
                }
                let placed = wrote && here.is_some();
                if let Some(place) = here.as_ref().filter(|_| wrote) {
                    writing.keep(PLACE.0, PLACE.1, place)?;
                }
                Ok((wrote, placed))
            });
            let (did, kept) = written.await?.0;
            wrote |= did;
            placed |= kept;
            whole |= kept && of_all;
        }
        // Written where the place is not kept: the place after it.
        if let Some((places, _)) = read.filter(|_| wrote && !placed) {
            whole = self.factory().kv_set(PLACE.0, PLACE.1, &places.kept()).await.is_ok();
        }
        Ok((wrote, whole))
    }

    /// What a comment on `record` is to be told as, where it names anybody:
    /// whose doing it was is asked of who said it (a key's maker). What a
    /// blueprint's step said is the factory's doing, said by its bot, or by
    /// the agent whose words they were (`decided`), as its line of the
    /// history says: whoever applied the blueprint only lent the authority,
    /// and is told where it names them, as anybody is.
    async fn named(
        &self,
        collection: &Collection,
        record: &str,
        event: &Event,
        identity: &Identity,
    ) -> Option<talk::commented::Comment> {
        // Only a page of a wiki is followed: no other record is read for it.
        let follow = match collection.model.as_deref() == Some(kernel::topics::MODEL) {
            true => match self.record(record).await {
                Ok((_, kept)) => self.following(collection, &kept, event.seq).await,
                Err(_) => Default::default(),
            },
            false => Default::default(),
        };
        let whose = self.whose(&event.data, event, identity).await;
        crate::named::comment(collection, record, event, whose, follow)
    }

    /// What a change of `record` is to whoever follows it, or a page above
    /// it with its pages, where somebody does and it is a change they are
    /// told of: of the record as it is now, the line of it (`noted`)
    /// saying whose doing it was.
    async fn followed(
        &self,
        collection: &Collection,
        noted: &Noted,
        event: &Event,
        identity: &Identity,
    ) -> Option<talk::followed::Change> {
        // Of a page of a wiki, and of what is said of one as such, or of its
        // making: what else is said of any record tells no follower.
        let of_page = event.topic.starts_with("topic.") || event.topic == "record.created";
        if collection.model.as_deref() != Some(kernel::topics::MODEL) || !of_page {
            return None;
        }
        let (_, kept) = self.record(&noted.record).await.ok()?;
        let under = kernel::following::parent_of(&kept.data).is_some();
        let what = kernel::following::page_change(&event.topic, &event.data, under)?;
        let follow = self.following(collection, &kept, event.seq).await;
        let whose = self.whose(&noted.data, event, identity).await;
        crate::named::change(collection, &noted.record, event, whose, follow, what)
    }

    /// Who followed the page `kept` as what is `seq` was said, and the pages
    /// above it, the nearest first, each with who follows it: whoever
    /// follows those with their pages is told too.
    async fn following(&self, collection: &Collection, kept: &Record, seq: u64) -> crate::named::Followed {
        let followers = self.followed_at(collection, kept, seq).await;
        let (mut above, mut seen) = (Vec::new(), vec![kept.id.clone()]);
        let mut parent = kernel::following::parent_of(&kept.data).map(str::to_string);
        // A ring of pages, which the tree reads as none, ends where it comes round.
        while let Some(page) = parent.take().filter(|page| !seen.contains(page)) {
            let Ok((_, up)) = self.record(&page).await else { break };
            parent = kernel::following::parent_of(&up.data).map(str::to_string);
            above.push((page.clone(), crate::named::followers(collection, &up.data)));
            seen.push(page);
        }
        (followers, above)
    }

    /// Who followed the page `kept` as what is `seq` in the factory's
    /// journal was said, and follows it still: whoever follows it now, but
    /// whoever began since (`kernel::topics::followed_then`), which this
    /// may have come to well after. Nobody, where nobody follows it now,
    /// and then nothing is read.
    async fn followed_at(&self, collection: &Collection, kept: &Record, seq: u64) -> Vec<String> {
        let now = crate::named::followers(collection, &kept.data);
        if now.is_empty() {
            return now;
        }
        let (mut since, mut after) = (Vec::new(), seq);
        loop {
            let query =
                EventQuery { topics: "topic.followed".into(), after, limit: 500, ..Default::default() };
            let Ok(page) = self.factory().read(query).await else { break };
            let Some(last) = page.last().map(|event| event.seq) else { break };
            for event in
                page.iter().filter(|event| event.data.get("record").and_then(Value::as_str) == Some(&kept.id))
            {
                if let (Some(by), Some(follows)) =
                    (event.data["by"].as_str(), event.data["follows"].as_bool())
                {
                    since.push((by.to_string(), follows));
                }
            }
            after = last;
        }
        kernel::topics::followed_then(&now, &since)
    }

    /// Who said what `event` says, as a line of it is told from (`said`,
    /// what the line of it holds), and whose doing it was: a key's maker
    /// for a key; for a blueprint's step the bot, or the agent whose
    /// words they were.
    async fn whose(&self, said: &Value, event: &Event, identity: &Identity) -> (String, String) {
        if kernel::worded::through(said).is_some() {
            let speaker = match kernel::worded::decided(said) {
                Some(agent) => format!("agent:{agent}"),
                None => {
                    let bot = self.bot.get().map(|speaker| speaker.name());
                    talk::channels::bot_actor(&bot.unwrap_or_else(|| talk::speaker::DEFAULT.into()))
                }
            };
            return (speaker.clone(), speaker);
        }
        let said = event.source.actor.clone().unwrap_or_default();
        let by = match identity.holder(&said).await {
            Ok(Some(holder)) => holder.actor(),
            _ => said.clone(),
        };
        (said, by)
    }

    /// Where the keeper of histories goes on from: the place in every
    /// journal that the factory's own keeps (a number alone, which a
    /// program from before kept, is the place above it in each), and in
    /// each project's journal the place that journal keeps of itself
    /// where that is further on.
    async fn read_to(&self) -> Places {
        let kept = self.factory().kv_get(PLACE.0, PLACE.1).await.ok().flatten();
        let mut places = kept.and_then(|kept| Places::from_kept(&kept)).unwrap_or_default();
        for own in self.factory().every().into_iter().skip(1) {
            let kept = own.kv_get(PLACE.0, PLACE.1).await.ok().flatten();
            if let Some(own_to) = kept.as_ref().and_then(Value::as_u64) {
                places.put(own.name(), places.of(own.name()).max(own_to));
            }
        }
        places
    }

    /// Goes back over the journals for what a collection counts as
    /// history, up to `upto`, a place in each: for one that has only now
    /// said what its history is, or said it differently. Each journal is
    /// gone over by itself, in its own order: what is noted is kept by
    /// the record and the event's number, which no other event has.
    async fn recall(&self, collection: &Collection, upto: &Places, identity: &Identity) -> Result<()> {
        let topics = collection.history.iter().map(|rule| rule.topic.as_str()).collect::<Vec<_>>().join(",");
        for journal in self.factory().every() {
            let (mut after, upto) = (0, self.factory().place_in(upto, &journal));
            while after < upto {
                let query = EventQuery { topics: topics.clone(), after, limit: 500, ..Default::default() };
                let page = journal.read(query).await?;
                let Some(last) = page.last().map(|event| event.seq) else { break };
                for event in page.iter().filter(|event| event.seq <= upto && event.topic != TOPIC) {
                    let lines = self.line(collection, event, identity).await.into_iter().collect();
                    self.note(event, lines, None).await?;
                }
                after = last;
            }
        }
        Ok(())
    }

    /// Keeps the histories from here on: follows every journal from where
    /// each was last read to, and writes down what each collection counts
    /// as the history of its records. It is told of every event and looks
    /// for none. Where it has read to is a place in each journal
    /// ([`Collections::read_to`]).
    ///
    /// What it hands back ends it. It holds the journal it follows, so
    /// that follower is never told the journal has shut, and nothing else
    /// ends it: left, it held a node's journal and tables for the life of
    /// the process, some two megabytes for every node a test started.
    ///
    /// Something said on a record that names somebody is told of here
    /// too ([`crate::named`]), in their box, once its line is written and
    /// before where this has read to is kept. `tables` is every table as
    /// it is read here, for what answers for such a line.
    pub fn keep_history(
        &self,
        identity: Identity,
        channels: talk::Channels,
        tables: crate::named::Declared,
    ) -> tokio::task::AbortHandle {
        let _ = self.bot.set(channels.speaker.clone());
        let this = self.clone();
        let keeper = tokio::spawn(async move {
            let journal: Journal = this.factory().clone();
            let mut follow = journal.follow_every(this.read_to().await, "");
            let mut declared = this.list(None).await.unwrap_or_default();
            *tables.write().expect("declared lock") = declared.clone();
            let mut since = 0;
            while let Ok(event) = follow.next().await {
                if event.seq == 0 || event.topic == TOPIC {
                    continue;
                }
                if event.topic == "collection.ensured" || event.topic == crate::forgotten::FORGOTTEN {
                    let now = this.list(None).await.unwrap_or_default();
                    // One whose history was just declared, or declared differently, is given what already happened.
                    for collection in now.iter().filter(|collection| !collection.history.is_empty()) {
                        let before = declared
                            .iter()
                            .find(|was| was.project == collection.project && was.name == collection.name);
                        if before.is_none_or(|was| was.history != collection.history) {
                            let _ = this.recall(collection, follow.places(), &identity).await;
                        }
                    }
                    declared = now;
                    *tables.write().expect("declared lock") = declared.clone();
                }
                // Where it has read to is kept with the lines written for
                // an event, in the write that keeps them, and the place
                // in every journal now and then besides: a project's own
                // journal keeps only its own.
                let mut lines = Vec::new();
                let (mut told, mut followed) = (None, None);
                for collection in declared.iter().filter(|collection| !collection.history.is_empty()) {
                    let line = this.line(collection, &event, &identity).await;
                    match line.as_ref() {
                        Some(noted) if event.topic == crate::records::COMMENTED => {
                            told = told.or(this.named(collection, &noted.record, &event, &identity).await);
                        }
                        Some(noted) if followed.is_none() => {
                            followed = this.followed(collection, noted, &event, &identity).await;
                        }
                        _ => {}
                    }
                    lines.extend(line);
                }
                let read = follow.heard_in().map(|heard| (follow.places(), heard));
                let whole = match (told, followed) {
                    (None, None) => this.note(&event, lines, read).await.unwrap_or((false, false)).1,
                    // Its line is written, then whoever it names or follows
                    // the record is told, and only then is where this has
                    // read to kept: stopped between, the event is read
                    // again, its line is there, and the telling is the same
                    // done twice.
                    (told, followed) => {
                        let _ = this.note(&event, lines, None).await;
                        if let Some(comment) = told {
                            let _ = channels.tell_commented(comment).await;
                        }
                        if let Some(change) = followed {
                            let _ = channels.tell_followed(change).await;
                        }
                        journal.kv_set(PLACE.0, PLACE.1, &follow.places().kept()).await.is_ok()
                    }
                };
                since += 1;
                if whole {
                    since = 0;
                } else if since >= 500 {
                    let _ = journal.kv_set(PLACE.0, PLACE.1, &follow.places().kept()).await;
                    since = 0;
                }
            }
        });
        keeper.abort_handle()
    }
}
