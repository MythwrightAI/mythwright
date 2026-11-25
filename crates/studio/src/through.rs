//! Which blueprint's step said an event of a record that says no
//! `through` of its own, for its line of the record's history.
//!
//! What a blueprint's step does to a record is the factory's doing, said
//! as its bot (`kernel::worded::as_called`). The actions that change a
//! record say which blueprint did (`through`); an event a step says for
//! itself (`emit`: a card waits its turn, its turn came, an agent worked
//! on it) says only where in which run it was said, and whoever applied
//! the blueprint as its actor, and was written under that person. So the
//! keeper of histories asks which blueprint that instance runs, and the
//! line says it as `through` from then on, as one an action said does.
//!
//! It never waits for anything: the instance is looked for in hand where
//! an engine was attached (`Studio::attach`), and otherwise, or where it
//! has been let go of, by its row in the journals there are, which is
//! there before anything it says is, engine or none. Where neither has
//! it (a row that is gone, a journal that keeps no instances) the line is
//! written as before, under whoever applied it, with no `through`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use conductor::{Downgraded, Engine};
use journal::Journal;
use kernel::Event;

/// How many instances' blueprints are remembered before the memory is let
/// go of and begun again: a step says many things in a row, and an
/// instance's blueprint never changes.
const REMEMBERED: usize = 4096;

/// The engine the instances run in, where one was attached, and what was
/// found of each instance asked about.
#[derive(Clone, Default)]
pub(crate) struct SaidBy {
    engine: Arc<OnceLock<Downgraded>>,
    known: Arc<Mutex<HashMap<String, String>>>,
}

impl SaidBy {
    /// Held weakly: the engine holds what holds this.
    pub(crate) fn attach(&self, engine: &Engine) {
        let _ = self.engine.set(engine.downgrade());
    }

    /// The blueprint whose step said `event`, where a step of a run did
    /// and the event does not say so itself: nothing for anybody else's
    /// doing, and nothing where the instance is found neither in hand nor
    /// by its row in `journals`.
    pub(crate) async fn of(&self, event: &Event, journals: &[Journal]) -> Option<String> {
        if kernel::worded::through(&event.data).is_some() || event.source.run.is_none() {
            return None;
        }
        let instance = event.source.instance.as_deref().filter(|instance| !instance.is_empty())?;
        if let Some(known) = self.known.lock().expect("said by lock").get(instance) {
            return Some(known.clone());
        }
        let in_hand = self
            .engine
            .get()
            .and_then(Downgraded::upgrade)
            .and_then(|engine| engine.running_in_hand(instance));
        let blueprint = match in_hand {
            Some(blueprint) => blueprint,
            None => {
                let mut found = None;
                for journal in journals {
                    // Read only where it was let go of first, which goes by
                    // how the instance's ending fell: measured as such a
                    // reading is ([`conductor::LOOKED_UP`]).
                    let began = std::time::Instant::now();
                    found = conductor::blueprint_kept(journal.db(), instance).await;
                    let took = began.elapsed().as_secs_f64() * 1000.0;
                    journal.measure(conductor::LOOKED_UP, &[("kind", "blueprint")], took, true);
                    if found.is_some() {
                        break;
                    }
                }
                found?
            }
        };
        let mut known = self.known.lock().expect("said by lock");
        if known.len() >= REMEMBERED {
            known.clear();
        }
        known.insert(instance.to_string(), blueprint.clone());
        Some(blueprint)
    }
}
