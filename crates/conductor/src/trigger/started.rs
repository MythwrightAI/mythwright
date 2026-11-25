//! Starting an instance's handlers, telling them to take no more, and
//! what is kept for them between whiles: the doors [`super`] is asked
//! through by whatever begins, settles, carries over or takes down an
//! instance.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use blueprint::TriggerSource;
use journal::Places;
use kernel::Cancel;
use runs::RunInfo;

use super::places::{begins, prepare_from};
use super::{Heard, Listening, listen};
use crate::engine::{Engine, Live};

/// Keeps the runs of an instance's handlers that were under way when the
/// conductor stopped, for one that is taken up in the middle of its steps
/// or of taking a text: its handlers are not started until those have
/// settled, and these are handed to them then. Dropped, a run that takes
/// its turn by a key and goes on beside an update read as under way for
/// ever, taken up by nothing and ended by nothing.
///
/// They are kept through the end of a generation ([`Listening::ended`]):
/// applied over before its handlers were started, they are still theirs
/// when the steps of that apply settle. And an instance that is stopped
/// or fails before then has them ended with it ([`unclaimed`], asked by
/// whatever takes it down).
pub(crate) fn leave(live: &Live, runs: Vec<RunInfo>) {
    live.listening.lock().expect("live lock").left = runs;
}

/// The runs kept for an instance's handlers that none of them has taken
/// up, kept no longer: whoever takes the instance down ends them.
pub(crate) fn unclaimed(live: &Live) -> Vec<RunInfo> {
    std::mem::take(&mut live.listening.lock().expect("live lock").left)
}

impl Listening {
    /// A generation has ended: nothing of it is listening, and what was
    /// kept for the handlers that are started next is kept for them
    /// still. Taken away with the rest, a run under way when the
    /// conductor stopped, of something stopped by what had started it in
    /// the moment a conductor's start was coming to it, was handed to
    /// nobody and read as under way for good.
    ///
    /// The generation that begins is counted here, where handlers are
    /// started: whoever starts them for the one before ([`arm_of`]) is
    /// either before all of this or finds another generation.
    pub(crate) fn ended(&mut self, generation: &AtomicU64) -> u64 {
        let (left, late) = (std::mem::take(&mut self.left), self.late);
        *self = Listening { left, late, ..Listening::default() };
        generation.fetch_add(1, Ordering::SeqCst) + 1
    }
}

/// Starts the handlers of an instance. `unfinished` are handler runs that
/// were in progress when the conductor last stopped; each handler finishes
/// its own before taking new events.
pub(crate) fn arm(engine: &Engine, live: &Arc<Live>, unfinished: Vec<RunInfo>) {
    listen(engine, live, unfinished, None, None);
}

/// As [`arm`], by whoever settled the steps of `generation`, and only
/// while that is the generation there is: asked where the handlers are
/// started, since an apply may begin another between being looked for
/// and this.
pub(crate) fn arm_of(engine: &Engine, live: &Arc<Live>, generation: u64) {
    listen(engine, live, Vec::new(), None, Some(generation));
}

/// Has the handlers of an instance that is listening go over to the text
/// now in force. One whose handlers have not been started is left: they
/// will be, in that text; but for one whose starting was held back for a
/// text it was to be carried over to and has not taken, which is started
/// here. `from` is where a handler the text brings begins, for whoever
/// took that place before the text was said to be taken ([`begins`]);
/// with none said, it is now.
pub(crate) async fn rearm(engine: &Engine, live: &Arc<Live>, from: Option<Places>) {
    let from = match from {
        Some(from) => from,
        None => begins(engine, None).await,
    };
    // Whoever asks has the instance in hand: nothing is to carry it over.
    let owed = {
        let mut listening = live.listening.lock().expect("live lock");
        listening.retired = false;
        (!listening.armed).then(|| std::mem::take(&mut listening.owed))
    };
    match owed {
        // A starting was held back for a text that was not taken.
        Some(true) => {
            prepare_from(engine, live, &from).await;
            return listen(engine, live, Vec::new(), None, None);
        }
        Some(false) => return,
        None => {}
    }
    // A handler the new text adds starts from the place it was given, and
    // the others from where they had got to.
    let had: Vec<String> = live.listening.lock().expect("live lock").heard.keys().cloned().collect();
    prepare_from(engine, live, &from).await;
    // One the text before had not may still have a place: that of a handler
    // of its name an earlier text dropped, not yet taken away. It is written
    // over here, so that a daemon stopped before the handler has said its
    // own place starts it from this one; the handler says it again when the
    // one that was dropped has ended, which may write its last place yet.
    for trigger in &live.blueprint().triggers {
        if matches!(trigger.source, TriggerSource::Event(_)) && !had.contains(&trigger.name) {
            for journal in engine.hears_from(live, super::everywhere(trigger)) {
                let (name, place) = (journal.name(), from.of(journal.name()));
                let _ = engine.inner.store.set_place(&live.id, &trigger.name, name, place).await;
            }
        }
    }
    listen(engine, live, Vec::new(), Some(from), None);
}

/// Tells every handler of an instance to take no more events, and gives
/// back what each says when it has ended, which is when it is next between
/// runs. Nothing listens in their place until [`rearm`] or, after another
/// generation has begun, [`arm`]: what happens meanwhile waits in the
/// journal for whichever comes ([`crate::carry`]). And none is started
/// in the meantime, where none had been yet (`Listening::retired`).
pub(crate) fn retire(live: &Live) -> Vec<Cancel> {
    let mut listening = live.listening.lock().expect("live lock");
    listening.retired = true;
    let ended = |heard: &Heard| {
        heard.retire.cancel();
        heard.ended.clone()
    };
    listening.heard.values().map(ended).collect()
}
