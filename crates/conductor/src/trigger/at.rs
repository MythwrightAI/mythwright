//! A handler that fires at a time of day (`at:`, `blueprint::at`). When
//! it last fired is kept beside its instance's rows before the run it
//! starts is written, so a time that came while the conductor was stopped
//! is fired once as it is back (the last of them), and none twice.

use std::time::Duration;

use serde_json::json;

use super::Handler;

impl Handler {
    pub(super) async fn at(&mut self, written: &str) {
        let journal = self.engine.own().clone();
        let clock = journal.clock().clone();
        let mut scope = self.engine.base_scope(&self.live);
        scope.retain(|name, _| matches!(name.as_str(), "params" | "instance" | "target"));
        let (text, at) = match blueprint::at::filled(written, &scope) {
            Ok(at) => at,
            Err(error) => {
                let error = format!("the handler `{}` never fires: {error}", self.trigger.name);
                let data = json!({ "handler": self.trigger.name, "error": error });
                self.engine.emit("handler.failed", kernel::Source::instance(&self.live.id), data).await;
                return;
            }
        };
        let memory = format!("{}/{}", self.live.id, self.trigger.name);
        let kept = journal.kv_get(runs::beside::AT, &memory).await.ok().flatten();
        let made =
            self.engine.inner.store.instance(&self.live.id).await.ok().flatten().map(|made| made.created);
        let (mut last, keep) = blueprint::at::begun(kept.as_ref(), &text, clock.now_ms(), made);
        if keep {
            let _ = journal.kv_set(runs::beside::AT, &memory, &blueprint::at::kept(&text, last)).await;
        }
        loop {
            let now = clock.now_ms();
            match at.due(last, now) {
                Ok(due) => {
                    last = due;
                    let _ = journal.kv_set(runs::beside::AT, &memory, &blueprint::at::kept(&text, due)).await;
                    let data = json!({ "handler": self.trigger.name, "at": due });
                    self.start(json!({ "topic": "timer", "ts": clock.now_ms(), "data": data }), &journal, 0)
                        .await;
                }
                Err(None) => return,
                Err(Some(next)) => tokio::select! {
                    () = clock.sleep(Duration::from_millis((next - now).max(0) as u64)) => {}
                    () = self.cancel.cancelled() => return,
                    () = self.heard.retire.cancelled() => return,
                },
            }
        }
    }
}
