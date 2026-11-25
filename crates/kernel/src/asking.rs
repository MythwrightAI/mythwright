//! Which step asked for what is being begun: a `call` asks for its child,
//! and the run of the child's steps that the call begins keeps which step
//! of which run asked (`runs.asked_by`), so that whatever made the run
//! that asked can find the run it came to (a press of Run on a project's
//! Pipelines page opens the run that press began, and no other).
//!
//! It is carried as the chain of events is ([`crate::chain`]): the step
//! asks [`within`] what it asks for, and whatever writes a run down in
//! that task reads [`current`] as it writes. What is asked is named by the
//! instance that asks and the child's key, so that a run of anything else
//! begun in that task (what the child attaches) keeps nothing of it.

/// A step asking for the child of `parent` called `key`: `by` is the step,
/// as its run and its name, and `asked` what the event that set its run
/// off named its asking (`asked` of what it carries: what a press of Run
/// named it), by which the run is found again (`runs::beside::ASKED`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asking {
    pub parent: String,
    pub key: String,
    pub by: String,
    pub asked: Option<String>,
}

impl Asking {
    /// What the event that set a run off named its asking, where it named one.
    pub fn named(event: &serde_json::Value) -> Option<String> {
        let asked = event.get("data")?.get("asked")?.as_str()?;
        (!asked.is_empty()).then(|| asked.to_string())
    }
}

tokio::task_local! {
    static ASKING: Asking;
}

/// What a step is asking for here and now, if one is.
pub fn current() -> Option<Asking> {
    ASKING.try_with(Clone::clone).ok()
}

/// Does `work` as asked by `asking`.
pub async fn within<T>(asking: Asking, work: impl Future<Output = T>) -> T {
    ASKING.scope(asking, work).await
}
