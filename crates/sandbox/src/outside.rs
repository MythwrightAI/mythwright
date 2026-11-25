//! What answers for an action that reaches outside the tables: chosen
//! for each sandbox as it is made, here and nowhere else.
//!
//! Today the choice is no choice: every such action is answered by the
//! stand-in ([`crate::stood::Stood`]), which runs nothing. It is made in
//! one place all the same, so that something else may one day answer
//! for some of them where whoever makes the sandbox says so (an agent
//! that is really asked, say) without the registry or the record
//! changing: such an answerer is another kind ([`crate::Kind`]), named
//! as itself where the kinds are listed, writes each asking in the same
//! record ([`crate::stood::Record`]), and is handed here.

use std::sync::Arc;

use blueprint::ActionSpec;
use conductor::{ActionCx, ActionResult};
use kernel::{BoxFut, Fields};

use crate::Made;
use crate::offered::Kind;
use crate::stood::{Record, Stood};

/// Something that answers for an action a sandbox does not have.
pub trait Outside: Send + Sync + 'static {
    /// The step of `cx` asks `spec` with `args`.
    fn ask(
        self: Arc<Self>,
        spec: Arc<ActionSpec>,
        cx: ActionCx,
        args: Fields,
    ) -> BoxFut<'static, ActionResult>;
}

impl Outside for Stood {
    fn ask(
        self: Arc<Self>,
        spec: Arc<ActionSpec>,
        cx: ActionCx,
        args: Fields,
    ) -> BoxFut<'static, ActionResult> {
        Box::pin(async move { Stood::ask(&self, &spec, cx, args).await })
    }
}

/// What was chosen for one sandbox.
pub(crate) struct Chosen {
    pub record: Arc<Record>,
    pub stood: Arc<Stood>,
}

impl Chosen {
    /// What `made` says answers for what reaches outside: the stand-in,
    /// answering as it says stand-ins answer.
    pub fn of(made: &Made) -> Chosen {
        let record = Arc::new(Record::default());
        let stood = Stood::new(record.clone(), made.answers, made.told.clone());
        Chosen { record, stood }
    }

    /// What answers for the action `spec` describes, and what kind of
    /// action that makes it in this sandbox.
    pub fn answers_for(&self, _spec: &ActionSpec) -> (Kind, Arc<dyn Outside>) {
        (Kind::Stood, self.stood.clone())
    }
}
