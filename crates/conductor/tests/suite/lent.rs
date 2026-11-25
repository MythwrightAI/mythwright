//! Actions that are not registered, here while a test says so
//! ([`crate::Shared::lent`]).

use std::sync::Arc;

use blueprint::ActionSpec;
use conductor::{ActionCx, ActionResult};
use kernel::{BoxFut, Fields};
use serde_json::json;

use crate::Shared;

/// Actions that are not registered: they are here while the test says so.
/// `lent.echo` is one; `test.echo` is offered too, and is never asked for,
/// since an action of that name is registered.
pub(crate) struct Lent(pub(crate) Arc<Shared>);

impl Lent {
    fn here(&self) -> Vec<Arc<dyn conductor::Action>> {
        if !*self.0.lent.lock().unwrap() {
            return Vec::new();
        }
        ["lent.echo", "test.echo"]
            .into_iter()
            .map(|name| {
                let spec = ActionSpec::new(name, "Say that it was lent.").capability("lent").idempotent();
                Arc::new(Echo(spec)) as Arc<dyn conductor::Action>
            })
            .collect()
    }
}

struct Echo(ActionSpec);

impl conductor::Action for Echo {
    fn spec(&self) -> &ActionSpec {
        &self.0
    }

    fn run(&self, _: ActionCx, _: Fields) -> BoxFut<'static, ActionResult> {
        Box::pin(async { Ok(json!({ "lent": true })) })
    }
}

impl conductor::Brought for Lent {
    fn action(&self, name: &str) -> Option<Arc<dyn conductor::Action>> {
        self.here().into_iter().find(|action| action.spec().name == name)
    }

    fn specs(&self) -> Vec<ActionSpec> {
        self.here().iter().map(|action| action.spec().clone()).collect()
    }

    fn absent(&self, name: &str, once_here: bool) -> Option<String> {
        let when = if once_here { "was lent and has been taken back" } else { "has not been lent" };
        name.starts_with("lent.").then(|| format!("`{name}` {when}"))
    }
}
