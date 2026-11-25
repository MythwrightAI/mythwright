//! How deep an event is in a chain of events, each said while the one
//! before it was being handled.
//!
//! A handler that hears an event and says another, which a second handler
//! hears and answers with the first, goes round for ever, as fast as the
//! machine will go. Nothing in either says so. What tells such a round from
//! a great many things happening at once is how deep it goes: a thousand
//! cards moved are a thousand events one deep, and two handlers answering
//! each other are one event a thousand deep.
//!
//! So whatever handles an event does it [`within`] a chain one deeper than
//! the event's, and whatever is said meanwhile, by anything, anywhere in
//! that task, is of that depth: the journal reads [`current`] as it writes
//! an event down. Work started for the one handling (a step, a child's own
//! steps) is given the same chain by whoever starts it.

/// A place in a chain: how many events deep, and when the chain began.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chain {
    /// 1 for what is said while handling an event that is in no chain.
    pub depth: u32,
    /// Milliseconds since the Unix epoch.
    pub since: i64,
}

tokio::task_local! {
    static CHAIN: Chain;
}

/// The chain what is said here and now is part of, if it is part of one.
pub fn current() -> Option<Chain> {
    CHAIN.try_with(|chain| *chain).ok()
}

/// Does `work` as part of `chain`. With none, `work` is part of whatever
/// chain this already is.
pub async fn within<T>(chain: Option<Chain>, work: impl Future<Output = T>) -> T {
    match chain {
        Some(chain) => CHAIN.scope(chain, work).await,
        None => work.await,
    }
}
