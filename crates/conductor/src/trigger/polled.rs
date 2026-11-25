//! A handler that asks something on a timer and goes on when the answer
//! is another (`poll:`).
//!
//! The first reading, where nothing was read before, is taken as the
//! handler begins to listen and is only a baseline: what is heard is a
//! change from then on. What it last read is kept beside its instance's
//! rows, in the journal of the conductor that runs it
//! (`runs::beside::POLL`), and a reading that
//! changed is kept in the write that makes the run it sets off
//! (`Store::insert_polled_run`), never before it: a reading kept and no
//! run begun for it (the handler told to take no more while the source
//! answered, as a text is taken where its instance is; the run not
//! written; the daemon gone between the two) was the change nobody would
//! hear of, since the next reading, its own or a successor's, was held to
//! it. Where nothing is begun, the reading kept stays the one before, and
//! the change is come to again.

use std::time::Duration;

use kernel::Fields;
use serde_json::{Value, json};

use super::Handler;
use crate::labels;
use crate::sink::{Sink, log_id};
use crate::step::{self, Call, Done};

impl Handler {
    /// Where what this handler last read is kept.
    pub(super) fn memory(&self) -> String {
        format!("{}/{}", self.live.id, self.trigger.name)
    }

    pub(super) async fn poll(&mut self, every_ms: u64, action: &str, with: &Value) {
        let journal = self.engine.own().clone();
        let clock = journal.clock().clone();
        let memory = self.memory();
        let mut previous = journal.kv_get(runs::beside::POLL, &memory).await.ok().flatten();
        let mut failures = 0u32;
        // With nothing read before, the first reading is taken as it begins
        // to listen, as a handler of events hears from then: taken a whole
        // interval later, what changed meanwhile was in the first reading,
        // which is only a baseline, and set nothing off.
        let mut at_once = previous.is_none();
        loop {
            // Back off while the source is failing, and never line up with
            // every other poller on the device.
            let delay = every_ms * (1 << failures.min(4)) + kernel::id::random_below(every_ms / 10 + 1);
            if !std::mem::take(&mut at_once) {
                tokio::select! {
                    () = clock.sleep(Duration::from_millis(delay)) => {}
                    () = self.cancel.cancelled() => return,
                    () = self.heard.retire.cancelled() => return,
                }
            }
            let started = clock.now_ms();
            let result = self.poll_once(action, with).await;
            let labels = self.labels();
            let labels: Vec<(&str, &str)> =
                labels.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
            // A poll is the instance asking something, so it is measured
            // as the rest of what the instance does is.
            let carried = self.live.meta().labels;
            let labels = labels::beside(&labels, &carried);
            journal.measure("poll", &labels, (clock.now_ms() - started) as f64, result.is_ok());
            match result {
                Ok(current) => {
                    failures = 0;
                    match &previous {
                        // The first reading is only a baseline.
                        None => {
                            let _ = journal.kv_set(runs::beside::POLL, &memory, &current).await;
                            previous = Some(current);
                        }
                        Some(before) if before == &current => {}
                        Some(before) => {
                            let event = json!({
                                "topic": "poll.changed",
                                "ts": clock.now_ms(),
                                "data": current,
                                "previous": before,
                            });
                            if self.begin(event, &journal, 0, Some(&current)).await {
                                previous = Some(current);
                            }
                        }
                    }
                }
                Err(error) => {
                    if failures == 0 {
                        let source = kernel::Source::instance(&self.live.id);
                        self.engine
                            .emit(
                                "poll.failed",
                                source,
                                json!({ "handler": self.trigger.name, "error": error }),
                            )
                            .await;
                    }
                    failures += 1;
                }
            }
        }
    }

    async fn poll_once(&self, action: &str, with: &Value) -> Result<Value, String> {
        let cx = self.run_cx(String::new(), Value::Null, None);
        let scope = self.engine.base_scope(&self.live);
        let args = match blueprint::template::render(with, &scope)? {
            Value::Object(args) => args,
            _ => Fields::new(),
        };
        let journal = self.engine.own().clone();
        let source = kernel::Source::instance(&self.live.id);
        let (sink, _) = Sink::new(
            journal,
            self.engine.logs().clone(),
            log_id(&self.live.id, &self.trigger.name),
            source,
            Vec::new(),
        );
        let call = Call {
            cx: &cx,
            step: "",
            attempt: 0,
            scope: &scope,
            sink: std::sync::Arc::new(sink),
            cancel: self.cancel.child(),
            env: &Fields::new(),
            with,
            saved: None,
            element: None,
            device: None,
            again: false,
            entry: None,
            list: None,
            rule: None,
            around: false,
            limit: None,
        };
        match step::dispatch(call, action, args).await {
            Ok(Done::Out(value)) => Ok(value),
            Ok(Done::End(_)) => Ok(Value::Null),
            Err(error) => Err(error.message),
        }
    }
}
