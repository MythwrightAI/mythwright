//! Whoever follows a board, a table, a library, a channel or a project is
//! told of what changes in it, and whoever is told hourly or daily is told
//! once the hour or the day is over (`talk::following`).
//!
//! **What changed** is heard here, from every journal, by what is said of
//! it (`kernel::following::thing_change`): a row added, changed, taken
//! away or said something on, a message, a project renamed or moved. A
//! change of nothing anybody follows costs no reading. Each that somebody
//! follows is told in one write, which keeps where this has heard to:
//! stopped anywhere, a change is told once. A page of a wiki is told by
//! whoever keeps its history, as it always was (`talk::followed`).
//!
//! **A digest** is due once the period its first change came in is over
//! (`kernel::following::due`): the next whole hour, or the next time of
//! day the follower said in the zone they keep (`people::clocked`; UTC
//! where they keep none). What keeps the time sleeps until the first that
//! is due, on the daemon's clock, and is woken as a change comes to wait;
//! nothing is looked for on a timer. Started again, whatever is due is
//! told then, once.
//!
//! **Who may.** A follower is told of a change only where they may hear
//! what was said of it, as the event stream would tell them, and read
//! what they follow, both as they hold things then: asked in the write
//! that tells.

use std::sync::Arc;

use journal::Places;
use kernel::following::{Often, thing_change, worded};
use kernel::{Event, Principal, PrincipalKind, Source};
use serde_json::Value;
use studio::Store;
use talk::following::{Follow, May, Pended, Told};
use topics::Topics;

use crate::state::App;

/// Where what tells followers has heard to, in the factory's own journal.
const PLACE: (&str, &str) = ("following", "heard");
/// What is heard: what changes something that can be followed.
const HEARD: &str = "record.created,record.updated,record.removed,record.commented,channel.message,project.renamed,project.placed";

/// Whether whoever is an actor, holding what they hold, may be told of a
/// change said as `topic` that takes `takes`: as the event stream would.
pub(crate) fn may(app: &App) -> Arc<May> {
    let (journal, engine) = (app.journal.clone(), app.engine.clone());
    Arc::new(move |actor: &str, held: &[String], topic: &str, takes: Option<&str>| {
        let (kind, id) = actor.split_once(':').unwrap_or(("user", actor));
        let kind = if kind == "agent" { PrincipalKind::Agent } else { PrincipalKind::User };
        let who = Principal { kind, id: id.into(), name: id.into(), scopes: held.to_vec(), key: None };
        let event = Event {
            seq: 0,
            ts: 0,
            topic: topic.into(),
            source: Source::default().taking(takes),
            data: Value::Null,
        };
        journal.may_tell_as(actor, held, &event) && engine.may_hear(&who, topic)
    })
}

/// Which kind of table `name` of `project` is: its board, a library, or a table.
pub(crate) async fn table_kind(app: &App, project: &str, name: &str) -> Option<&'static str> {
    let table = app.studio.collections.get(project, name).await.ok()?;
    Some(match (table.name == studio::board::BOARD, table.store) {
        (true, _) => "board",
        (false, Store::Documents) => "library",
        _ => "table",
    })
}

/// What a project is called, or its id.
async fn project_called(app: &App, project: &str) -> String {
    app.studio.projects.get(project).await.map_or_else(|_| project.to_string(), |found| found.name)
}

/// What the thing `kind` and `thing` name is called, as a line says it.
pub(crate) async fn title_of(app: &App, kind: &str, thing: &str) -> String {
    let system = Principal::system();
    match kind {
        "page" => {
            let topics = Topics::new(app.journal.clone(), app.studio.clone(), None);
            topics
                .find(thing, &system)
                .await
                .map_or_else(|_| thing.to_string(), |page| studio::models::Topic::read(&page.data).title)
        }
        "channel" => app
            .studio
            .channels
            .found(thing, &system)
            .await
            .map_or_else(|_| thing.to_string(), |found| format!("#{}", found.name)),
        "project" => project_called(app, thing).await,
        _ => match thing.split_once('/') {
            Some((project, _)) if kind == "board" => format!("{} board", project_called(app, project).await),
            Some((project, name)) => format!("{name} of {}", project_called(app, project).await),
            None => thing.to_string(),
        },
    }
}

/// What a row or a page is called where it says: its title, or its name;
/// nothing where it is gone.
pub(crate) async fn label_of(app: &App, record: &str) -> String {
    let Ok((_, kept)) = app.studio.collections.record(record).await else { return String::new() };
    ["title", "name"]
        .iter()
        .find_map(|field| kept.data.get(*field).and_then(Value::as_str).filter(|said| !said.is_empty()))
        .unwrap_or_default()
        .to_string()
}

/// Tells whoever follows what changes, for the life of the node.
pub(crate) async fn tell(app: App) {
    let journal = app.journal.clone();
    let kept = journal.kv_get(PLACE.0, PLACE.1).await.ok().flatten();
    let from = kept.and_then(|kept| Places::from_kept(&kept)).unwrap_or_else(|| journal.now());
    let mut heard = journal.follow_every(from, HEARD);
    let may = may(&app);
    let mut since = 0;
    while let Ok(event) = heard.next().await {
        let place = heard.places().kept();
        let follows = app.studio.channels.follows();
        let change = thing_change(&event.topic, &event.data);
        let things: Vec<(&str, String)> = change
            .as_ref()
            .map(|change| {
                change.things.iter().filter(|(kind, thing)| follows.anybody(kind, thing)).cloned().collect()
            })
            .unwrap_or_default();
        let Some(change) = change.filter(|_| !things.is_empty()) else {
            // Where it has heard to is kept now and then besides.
            since += 1;
            if since >= 500 {
                let _ = journal.kv_set(PLACE.0, PLACE.1, &place).await;
                since = 0;
            }
            continue;
        };
        let kind = match &change.table {
            Some((project, name)) => table_kind(&app, project, name).await.unwrap_or("table"),
            None => "project",
        };
        let (noun, verb) = worded(kind, change.noun, change.verb);
        let mut named = Vec::new();
        for (thing_kind, thing) in things {
            let called = match thing_kind {
                "table" => title_of(&app, kind, &thing).await,
                other => title_of(&app, other, &thing).await,
            };
            named.push((thing_kind.to_string(), thing, called));
        }
        let record = ["id", "message"]
            .iter()
            .find_map(|key| event.data.get(*key).and_then(Value::as_str))
            .unwrap_or_default()
            .to_string();
        let from = event.source.actor.clone().unwrap_or_default();
        // Whose doing it was: a key's is its maker's, who is told nothing of it.
        let by = match app.studio.identity.holder(&from).await {
            Ok(Some(holder)) => holder.actor(),
            _ => from.clone(),
        };
        let pended = Pended {
            seq: event.seq,
            record,
            noun: noun.to_string(),
            verb: verb.to_string(),
            topic: event.topic.clone(),
            takes: event.source.takes.clone(),
            from: from.clone(),
            at: event.ts,
        };
        let told = Told { things: named, pended, by, ts: event.ts };
        let _ = app.studio.channels.tell_things(told, may.clone(), Some((PLACE.0, PLACE.1, place))).await;
        since = 0;
    }
}

/// The first moment after `ms` that is `at` (`HH:MM`) in `zone`, or in UTC
/// where that is no zone this machine knows.
fn daily(at: &str, zone: Option<&str>, ms: i64) -> Option<i64> {
    let read = zone
        .and_then(|zone| reader::at::read(&format!("{at} {zone}")).ok())
        .or_else(|| reader::at::read(&format!("{at} UTC")).ok())?;
    read.next_after(ms)
}

/// When `follow`'s digest of what waits since `first` is due.
async fn due(app: &App, follow: &Follow, first: i64) -> Option<i64> {
    let zone = match follow.often {
        Often::Daily => app.studio.clocked.zone_of(&follow.who).await,
        _ => None,
    };
    kernel::following::due(follow.often, first, |ms| daily(&follow.at, zone.as_deref(), ms))
}

/// Tells each digest as it is due, for the life of the node.
pub(crate) async fn digests(app: App) {
    let wakes = app.studio.channels.follows().wakes();
    let clock = app.journal.clock().clone();
    let may = may(&app);
    loop {
        let woken = wakes.notified();
        tokio::pin!(woken);
        woken.as_mut().enable();
        let (mut next, mut again) = (None::<i64>, false);
        for (follow, first) in app.studio.channels.waiting().await.unwrap_or_default() {
            let Some(at) = due(&app, &follow, first).await else { continue };
            if at <= clock.now_ms() {
                // What came after its period waits for the next, which is asked of again.
                let title = title_of(&app, &follow.kind, &follow.thing).await;
                again |= app.studio.channels.digest(follow, at, title, may.clone()).await.is_ok();
            } else {
                next = Some(next.map_or(at, |next| next.min(at)));
            }
        }
        if again {
            continue;
        }
        match next {
            Some(at) => {
                tokio::select! {
                    () = clock.sleep_until(at) => {}
                    () = &mut woken => {}
                }
            }
            None => woken.await,
        }
    }
}
