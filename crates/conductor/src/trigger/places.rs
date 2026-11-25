//! Where a handler begins among the events: a place in each journal it
//! hears, given when the text that brings it is taken.

use blueprint::{Blueprint, TriggerSource};
use journal::Places;

use super::everywhere;
use crate::engine::{Engine, Live};

/// Fixes the position each event handler starts from, for handlers that do
/// not have one yet. Called before the instance is announced as active, so
/// that "active" means "listening": any event published after that
/// announcement is one the handlers will see.
///
/// A handler with no position is one the text has just brought: it starts
/// from now. Unless the instance took the text because of an event
/// (`during` is that event's place: a handler's step asked, while handling
/// it): then it starts from just before that event, and so is handed it,
/// if it is one of its own. A card moved to where a lane takes it up is
/// taken up by the handler the lane's new text adds for that, and not left
/// where it was moved to.
///
/// A handler's place is a place in each journal it hears
/// ([`Engine::hears_from`]): now in each, and just before the event in
/// the journal that keeps it.
///
/// Gives back the place a handler the text has just brought starts from.
pub(crate) async fn prepare(engine: &Engine, live: &Live, during: Option<u64>) -> Places {
    let from = begins(engine, during).await;
    prepare_from(engine, live, &from).await;
    from
}

/// [`prepare`], from a place that was taken earlier: before the text
/// that brings the handlers was said to be taken ([`begins`]).
pub(crate) async fn prepare_from(engine: &Engine, live: &Live, from: &Places) {
    for (handler, journal, seq) in places_from(engine, live, &live.blueprint(), from).await {
        let _ = engine.inner.store.set_place(&live.id, &handler, &journal, seq).await;
    }
}

/// Where a handler brought now begins, in each journal there is: now,
/// and just before the event numbered `during` in the journal that
/// keeps it. A journal made later is begun from its beginning.
pub(crate) async fn begins(engine: &Engine, during: Option<u64>) -> Places {
    let journal = engine.journal();
    let mut from = journal.now();
    if let Some(seq) = during
        && let Ok(Some(keeps)) = journal.keeping(seq).await
    {
        from.put(keeps.name(), seq.saturating_sub(1));
    }
    from
}

/// The handlers of events `blueprint` has that have no place yet in a
/// journal they hear, each with that journal's name and its place in
/// `from`.
pub(crate) async fn places_from(
    engine: &Engine,
    live: &Live,
    blueprint: &Blueprint,
    from: &Places,
) -> Vec<(String, String, u64)> {
    let store = &engine.inner.store;
    let mut places = Vec::new();
    for trigger in &blueprint.triggers {
        if !matches!(trigger.source, TriggerSource::Event(_)) {
            continue;
        }
        for journal in engine.hears_from(live, everywhere(trigger)) {
            if !matches!(store.place(&live.id, &trigger.name, journal.name()).await, Ok(Some(_))) {
                places.push((trigger.name.clone(), journal.name().to_string(), from.of(journal.name())));
            }
        }
    }
    places
}

/// Where each handler that `blueprint` brings to an instance begins:
/// those of its handlers of events that have no place yet, each with the
/// place [`prepare`] would give it and what the journal it is a place in
/// is called. For whoever writes a text as taken, to write with it.
pub(crate) async fn places(
    engine: &Engine,
    live: &Live,
    blueprint: &Blueprint,
    during: Option<u64>,
) -> Vec<(String, String, u64)> {
    places_from(engine, live, blueprint, &begins(engine, during).await).await
}
