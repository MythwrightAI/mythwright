//! Where a step's output goes.
//!
//! Each line is appended to the step's log file, broadcast (but not stored as
//! an event) for anyone watching live, and checked against the step's `log:`
//! watch rules. A rule fires at most once per attempt, so a build that prints
//! a thousand errors raises one event, not a thousand.

use std::sync::{Arc, Mutex};

use journal::{Journal, Logs};
use kernel::{Fields, Source};
use regex_lite::Regex;
use serde_json::{Value, json};
use tokio::sync::mpsc;

/// Somewhere else a step's output is also sent: across a link, to the device
/// that asked for the work.
pub type Forward = Arc<dyn Fn(char, &str) + Send + Sync>;

pub(crate) struct Sink {
    journal: Journal,
    logs: Logs,
    log_id: String,
    source: Source,
    rules: Vec<(usize, Regex)>,
    fired: Mutex<Vec<bool>>,
    hits: mpsc::UnboundedSender<usize>,
    forward: Option<Forward>,
}

impl Sink {
    /// `rules` are `(index of the watch rule, pattern)`. Returns the sink and
    /// the stream of rule indexes that matched.
    pub fn new(
        journal: Journal,
        logs: Logs,
        log_id: String,
        source: Source,
        rules: Vec<(usize, Regex)>,
    ) -> (Sink, mpsc::UnboundedReceiver<usize>) {
        let (hits, matched) = mpsc::unbounded_channel();
        let fired = Mutex::new(vec![false; rules.len()]);
        (Sink { journal, logs, log_id, source, rules, fired, hits, forward: None }, matched)
    }

    /// Where its lines are said from.
    pub fn said_from(&self) -> &Source {
        &self.source
    }

    /// Also hands every line to `forward`.
    pub fn forwarding(mut self, forward: Forward) -> Sink {
        self.forward = Some(forward);
        self
    }

    /// Writes down what the step was handed, in a factory that was started
    /// to say so: its arguments as they came to, and for a step that
    /// starts another blueprint, which one and as what. Nothing of a
    /// secret is in it: a secret is named in `env` and is never what an
    /// argument came to.
    pub fn handed(&self, to: Option<(&str, &str)>, args: &Fields) {
        if !self.logs.says_what_is_handed() {
            return;
        }
        let args = Value::Object(args.clone());
        match to {
            Some((blueprint, key)) => self.line('s', &format!("handed to {blueprint} as {key}: {args}")),
            None => self.line('s', &format!("handed: {args}")),
        }
    }

    pub fn line(&self, stream: char, text: &str) {
        let text = text.trim_end_matches(['\n', '\r']);
        self.logs.append("runs", &self.log_id, self.journal.now_ms(), stream, text);
        self.journal.emit_ephemeral(
            "log.line",
            self.source.clone(),
            json!({ "stream": stream, "text": text }),
        );
        if let Some(forward) = &self.forward {
            forward(stream, text);
        }
        if stream == 's' {
            return;
        }
        for (slot, (rule, pattern)) in self.rules.iter().enumerate() {
            if pattern.is_match(text) {
                let mut fired = self.fired.lock().expect("sink lock");
                if !std::mem::replace(&mut fired[slot], true) {
                    let _ = self.hits.send(*rule);
                }
            }
        }
    }
}

/// The log file id for a step of a run.
pub(crate) fn log_id(run: &str, step: &str) -> String {
    format!("{run}.{step}")
}
