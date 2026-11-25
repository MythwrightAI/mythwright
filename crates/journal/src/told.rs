//! Who an event may be handed to, by what the event says hearing it
//! takes ([`kernel::Source::takes`]).
//!
//! An event of something that is not everybody's to read says so. What
//! it says is a scope, held by whoever may read the thing (a record of a
//! table that is kept private), or `member:<what>`, for something kept
//! to those who are of it, which no scope answers for: holding
//! everything is not being one of them
//! ([`kernel::principal::covers`]). Who is of a thing is not the
//! journal's to know. It asks what was installed
//! ([`Journal::members_with`]) by whoever keeps that, at the moment the
//! event is handed over, and with nothing installed it answers no. One
//! thing it answers itself: what is one actor's alone
//! (`member:own:<actor>`, [`kernel::principal::own`]: somebody's own
//! themes) is handed to that actor and to nobody else.
//!
//! This is the one place `takes` is read. Whatever hands an event to
//! somebody (a stream, a handler, a count, what a run kept of the event
//! that set it off) asks [`Journal::may_tell`], and beside it whatever
//! the event's topic takes, which is its own to know; a test of the API
//! suite reads the source for anything that reads `takes` for itself.

use std::any::Any;
use std::sync::{Arc, OnceLock};

use kernel::principal::{OWN, covers, kept_to};
use kernel::{Event, Principal};

use crate::Journal;

/// What answers whether somebody is one of those a thing is kept to. It
/// is handed what follows `member:` in what the event takes, who asks as
/// an actor (a key is its maker's), and what they hold; it is asked each
/// time an event is handed over, so it answers from what it has in hand
/// and waits for nothing.
pub type Members = Arc<dyn Fn(&str, &str, &[String]) -> bool + Send + Sync>;

/// What an answer for `member:` answers from, where whoever said the
/// answer keeps that for a later saying to be handed back.
type From = Arc<dyn Any + Send + Sync>;

/// What every journal of a factory asks, held once for all of them.
#[derive(Default)]
pub(crate) struct Told {
    members: OnceLock<(Members, Option<From>)>,
}

impl Journal {
    /// Says what answers for `member:`. Said once, by whoever keeps who
    /// is of what, as the daemon starts; until then nobody is told an
    /// event kept to its members, and what is said a second time is not
    /// taken.
    pub fn members_with(&self, members: Members) {
        let _ = self.inner.told.members.set((members, None));
    }

    /// The same, by whoever answers from something it keeps in memory
    /// (`from`: who is in what), which it changes as that changes. It is
    /// still said once: `answer` is asked for only where nothing was
    /// said, and whoever says it is handed what the answer that stands
    /// answers from. So what keeps who is of what, opened a second time
    /// over the same journal (a test's daemon "started again" on a
    /// journal in memory), changes the one memory that is asked, and
    /// not one of its own that nothing asks: a member who left through
    /// the second would go on being told. Nothing, where what stands
    /// answers from something else, or from nothing.
    pub fn members_from<T: Any + Send + Sync>(
        &self,
        from: Arc<T>,
        answer: impl FnOnce(Arc<T>) -> Members,
    ) -> Option<Arc<T>> {
        let said = self.inner.told.members.get_or_init(|| (answer(from.clone()), Some(from)));
        said.1.clone().and_then(|from| from.downcast::<T>().ok())
    }

    /// Whether `who` may be handed `event`, as far as what the event
    /// itself takes goes: one that takes nothing is not kept from
    /// anybody here. What its topic takes is asked beside this.
    pub fn may_tell(&self, who: &Principal, event: &Event) -> bool {
        self.tells(event, &who.scopes, || who.actor())
    }

    /// The same, of whoever is known only by the actor they were and what
    /// they held: somebody who asked for something that goes on without
    /// them.
    pub fn may_tell_as(&self, actor: &str, held: &[String], event: &Event) -> bool {
        self.tells(event, held, || actor)
    }

    fn tells<A: AsRef<str>>(&self, event: &Event, held: &[String], actor: impl FnOnce() -> A) -> bool {
        let Some(takes) = event.source.takes.as_deref() else { return true };
        match kept_to(takes) {
            // One actor's alone: theirs to hear and nobody else's, which
            // asks nothing of whoever keeps who is of what.
            Some(of) if of.starts_with(OWN) => of[OWN.len()..] == *actor().as_ref(),
            Some(of) => {
                self.inner.told.members.get().is_some_and(|(members, _)| members(of, actor().as_ref(), held))
            }
            None => covers(held, takes),
        }
    }
}
