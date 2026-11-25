//! Comparisons: one blueprint started several ways, side by side.
//!
//! What a comparison is and how one is done is in [`compared`]; what a
//! way reads in place of a linked file in [`instead`]; what is counted of
//! what came of each thing in [`counted`]. The shapes, and what is a
//! function of them alone, are `kernel::compared`, offered here under the
//! names they always had.
//!
//! This stands on the runner and is no part of it. The runner knows that
//! something may answer for comparisons and asks it four things as it
//! runs (`conductor::Compared`): what a step starts in place of what it
//! names, what an instance reads in place of a file it links, which kept
//! texts are still wanted as it starts, and that it is starting.
//! [`Comparisons::attach`] installs the answers. Everything else is asked
//! here by whoever has a comparison to start, show or end, and done with
//! what the runner offers anybody: its journal, its library, the texts
//! it keeps and checks. Nothing here knows what the work is, any more
//! than the runner does.

mod asked;
pub mod beside;
mod compared;
pub mod copy;
mod counted;
pub mod events;
mod instead;
mod rolled;

use std::sync::Arc;

use conductor::{Compared, Engine, Registry};

pub use compared::{COMPARED, Compare, Comparison, Ran, TEXT, WAY, Way};
pub use counted::{ACCEPTED, CAME, Counted, GIVEN};
pub use instead::Reads;
pub use rolled::{Rule, Shared};

/// The comparisons of a factory: what is asked to start one, show one
/// and end one, and what the runner is answered with.
#[derive(Clone)]
pub struct Comparisons {
    engine: Engine,
    kept: Arc<compared::Kept>,
}

/// Says what this crate emits, and adds what a blueprint asks of
/// comparisons (`compare.start`, `compare.end`).
pub fn register(registry: &mut Registry) {
    registry.emits(events::events());
    asked::register(registry);
}

impl Comparisons {
    /// Has `engine` ask comparisons what it asks of them from now on, and
    /// gives what everybody else asks. Once for an engine: what is
    /// installed first is what the engine goes on asking.
    ///
    /// What is installed holds what is kept in memory and no engine: it
    /// is handed the one that asks, so an engine is not held by what it
    /// holds.
    pub fn attach(engine: &Engine) -> Comparisons {
        let kept = Arc::new(compared::Kept::default());
        let with = |kept: &Arc<compared::Kept>, engine: &Engine| Comparisons {
            engine: engine.clone(),
            kept: kept.clone(),
        };
        let (starts, reads, keeps, resumed) = (kept.clone(), kept.clone(), kept.clone(), kept.clone());
        engine.set_compared(Compared {
            starts: Box::new(move |engine, start, labels| {
                let this = with(&starts, engine);
                Box::pin(async move { this.way_for(start, labels).await })
            }),
            reads: Box::new(move |engine, instance, name| {
                let this = with(&reads, engine);
                Box::pin(async move { this.read_instead(instance, name).await })
            }),
            keeps: Box::new(move |engine, alive| {
                let this = with(&keeps, engine);
                Box::pin(async move { this.kept_texts(alive).await })
            }),
            resumed: Box::new(move |engine| {
                let this = with(&resumed, engine);
                Box::pin(async move { this.count_from_here().await })
            }),
            kept: kept.clone(),
        });
        with(&kept, engine)
    }

    /// What answers for `engine`'s comparisons, as an action of this
    /// crate's is handed an engine and nothing else: the same memory as
    /// what was attached, so that what it starts is what a start is
    /// answered from.
    pub(crate) fn of(engine: &Engine) -> kernel::Result<Comparisons> {
        let kept = engine.compared_kept().and_then(|kept| kept.downcast::<compared::Kept>().ok());
        let kept = kept.ok_or_else(|| {
            kernel::Error::unavailable(
                "comparisons are not looked after where this runs, so none can be started here",
            )
        })?;
        Ok(Comparisons { engine: engine.clone(), kept })
    }

    /// The revisions of the kept texts comparisons still want as a
    /// conductor starts with `alive` alive: those their ways start and
    /// read, and those the things started under a way by an instance that
    /// is alive read. What was kept for instances that are gone is put
    /// away.
    async fn kept_texts(&self, alive: &std::collections::HashSet<&str>) -> Vec<String> {
        let mut wanted: Vec<String> = self.compared_texts().await.into_iter().collect();
        wanted.extend(self.reads_resumed(alive).await);
        wanted
    }
}
