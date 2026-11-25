//! What stands in for an action that reaches outside the tables, and
//! the record of what such an action was asked.
//!
//! The record ([`Record`]) is of askings and knows nothing of who
//! answers: each asking is written down as it comes, with the step that
//! asked and what it handed, and again when it is answered, with what
//! it gave or why it failed and how it came by that. Whatever answers
//! for an action that reaches outside writes there ([`crate::outside`]);
//! today that is always a stand-in.
//!
//! A stand-in is registered under the name and the description of the
//! action it stands for, so that a text which checks against the real
//! one checks here. Asked, it writes down what it was asked (which step
//! of which run asked, and with what) and answers one of three ways:
//! what whoever tries said beforehand that this action answers; else,
//! where stand-ins answer at once, what its description says a stand-in
//! answers (`ActionSpec::stands`), or nothing where it says none; else
//! it waits until it is answered, or until its step is called off.
//!
//! It does nothing else. It is handed the step's arguments and the
//! names of the secrets the step would be handed, never a value: a
//! secret's value is read by what starts a program, and nothing here
//! starts one.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blueprint::ActionSpec;
use conductor::{ActionCx, ActionError, ActionResult};
use kernel::Fields;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::oneshot;

/// How a stand-in that nothing was said of beforehand answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answers {
    /// It waits until whoever runs the test answers it.
    #[default]
    Asked,
    /// It answers at once what its description says a stand-in answers.
    Default,
}

/// One asking of a stand-in: what it was asked, and what it answered.
#[derive(Clone, Debug, Serialize)]
pub struct Asking {
    /// Which asking of the sandbox this is, from 1: what it is answered by.
    pub id: u64,
    /// The action that was stood in, and what it says it does.
    pub action: String,
    pub summary: String,
    /// The step that asked: of which run of which instance.
    pub instance: String,
    pub blueprint: String,
    pub run: String,
    pub step: String,
    /// What the step handed it.
    pub asked: Value,
    /// The secrets the step would have been handed, by name. No value
    /// is here or anywhere in a sandbox.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
    /// When it was asked, by the sandbox's clock.
    pub at: i64,
    /// What a stand-in for this action answers where nobody says.
    pub usual: Value,
    /// Whether it is waiting to be answered.
    pub waiting: bool,
    /// What it answered, once it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answered: Option<Value>,
    /// Why it failed, where it was told to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// How it came by its answer: `told`, said beforehand for its
    /// action; `usual`, what a stand-in for it answers; `answered`, by
    /// whoever runs the test; `called off`, its step was stopped
    /// while it waited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<&'static str>,
}

type Answer = Result<Value, String>;

/// Every asking of a sandbox's actions that reach outside the tables,
/// in the order they were asked: what each was asked and what it
/// answered, whoever answered.
#[derive(Default)]
pub struct Record {
    asked: Mutex<Vec<Asking>>,
}

impl Record {
    /// What was asked so far, in order.
    pub fn asked(&self) -> Vec<Asking> {
        self.asked.lock().expect("record lock").clone()
    }

    /// How many askings there have been.
    pub(crate) fn count(&self) -> usize {
        self.asked.lock().expect("record lock").len()
    }

    /// Writes down that the step of `cx` asks `spec` with `args`, and
    /// gives the asking's number. `usual` is what a stand-in for the
    /// action answers where nobody says.
    pub(crate) fn note(&self, spec: &ActionSpec, cx: &ActionCx, args: Fields, usual: Value) -> u64 {
        let mut asked = self.asked.lock().expect("record lock");
        let id = asked.len() as u64 + 1;
        asked.push(Asking {
            id,
            action: spec.name.clone(),
            summary: spec.summary.clone(),
            instance: cx.instance().to_string(),
            blueprint: cx.blueprint().to_string(),
            run: cx.run().to_string(),
            step: cx.step().to_string(),
            asked: Value::Object(args),
            secrets: cx.secrets().values().cloned().collect(),
            at: cx.clock().now_ms(),
            usual,
            waiting: false,
            answered: None,
            failed: None,
            by: None,
        });
        id
    }

    /// Writes down that the asking `id` waits to be answered.
    pub(crate) fn waits(&self, id: u64) {
        if let Some(asking) =
            self.asked.lock().expect("record lock").iter_mut().find(|asking| asking.id == id)
        {
            asking.waiting = true;
        }
    }

    /// Writes down how the asking `id` came by its answer: what it gave
    /// or why it failed, and `by` what. It waits no longer.
    pub(crate) fn settle(&self, id: u64, gives: Option<Value>, fails: Option<String>, by: &'static str) {
        if let Some(asking) =
            self.asked.lock().expect("record lock").iter_mut().find(|asking| asking.id == id)
        {
            (asking.waiting, asking.answered, asking.failed, asking.by) = (false, gives, fails, Some(by));
        }
    }

    /// The askings that wait, each with what a stand-in for it answers.
    fn waiting(&self) -> Vec<(u64, Value)> {
        let asked = self.asked.lock().expect("record lock");
        asked.iter().filter(|asking| asking.waiting).map(|asking| (asking.id, asking.usual.clone())).collect()
    }
}

#[derive(Default)]
struct Kept {
    answers: Answers,
    told: Fields,
    waits: HashMap<u64, oneshot::Sender<Answer>>,
}

/// The stand-in for every action of a sandbox that reaches outside the
/// tables: what each is answered with, and who waits to be.
pub struct Stood {
    record: Arc<Record>,
    kept: Mutex<Kept>,
}

impl Stood {
    pub(crate) fn new(record: Arc<Record>, answers: Answers, told: Fields) -> Arc<Stood> {
        Arc::new(Stood { record, kept: Mutex::new(Kept { answers, told, waits: HashMap::new() }) })
    }

    /// A stand-in for `spec` is asked by the step of `cx`, with `args`.
    pub(crate) async fn ask(&self, spec: &ActionSpec, cx: ActionCx, args: Fields) -> ActionResult {
        let usual = spec.stands.clone().unwrap_or_else(|| json!({}));
        let id = self.record.note(spec, &cx, args, usual.clone());
        let wait = {
            let mut kept = self.kept.lock().expect("stand-ins lock");
            let at_once = match (kept.told.get(&spec.name), kept.answers) {
                (Some(told), _) => Some((told.clone(), "told")),
                (None, Answers::Default) => Some((usual, "usual")),
                (None, Answers::Asked) => None,
            };
            match at_once {
                Some((answer, by)) => {
                    self.record.settle(id, Some(answer.clone()), None, by);
                    return Ok(answer);
                }
                None => {
                    let (answer, wait) = oneshot::channel();
                    kept.waits.insert(id, answer);
                    self.record.waits(id);
                    wait
                }
            }
        };
        tokio::select! {
            answered = wait => match answered {
                Ok(Ok(gives)) => Ok(gives),
                Ok(Err(why)) => Err(ActionError::new(why)),
                // The sandbox is ending: nobody is left to answer.
                Err(_) => Err(ActionError::cancelled()),
            },
            () = cx.cancel().cancelled() => {
                self.kept.lock().expect("stand-ins lock").waits.remove(&id);
                self.record.settle(id, None, None, "called off");
                Err(ActionError::cancelled())
            }
        }
    }

    /// Answers the asking `id`, which waits: with what it gives, or with
    /// why it fails.
    pub(crate) fn answer(&self, id: u64, gives: Option<Value>, fails: Option<String>) -> kernel::Result<()> {
        let waits = self.kept.lock().expect("stand-ins lock").waits.remove(&id);
        let Some(waits) = waits else {
            return Err(kernel::Error::not_found(format!(
                "no stand-in numbered {id} is waiting to be answered: what waits is listed under `waiting`"
            )));
        };
        let gives = fails.is_none().then(|| gives.unwrap_or_else(|| json!({})));
        let answer = match (&gives, &fails) {
            (Some(gives), _) => Ok(gives.clone()),
            (None, why) => Err(why.clone().unwrap_or_default()),
        };
        self.record.settle(id, gives, fails, "answered");
        // Its step may have been called off in this moment: it is then
        // written as answered, and nobody was there to take the answer.
        let _ = waits.send(answer);
        Ok(())
    }

    /// Says how stand-ins answer from now on. Where they are to answer at
    /// once, each that waits is answered what a stand-in for it answers.
    pub(crate) fn answers(&self, answers: Answers) {
        self.kept.lock().expect("stand-ins lock").answers = answers;
        if answers == Answers::Asked {
            return;
        }
        for (id, usual) in self.record.waiting() {
            let waits = self.kept.lock().expect("stand-ins lock").waits.remove(&id);
            if let Some(waits) = waits {
                self.record.settle(id, Some(usual.clone()), None, "usual");
                let _ = waits.send(Ok(usual));
            }
        }
    }
}
