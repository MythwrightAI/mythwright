//! Running one step, with everything a step gets for free.
//!
//! This is where the reusable lifecycle lives. Whatever a step does, it gets
//! a guard, a timeout, retries with backoff, watch rules that fire while it
//! runs, hooks when it starts, finishes or fails, a log, a measurement, and a
//! record that survives a restart. None of that is written per component; it
//! is here once.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use blueprint::{Blueprint, Step, StepKind, WatchCond, duration, template};
use kernel::{Cancel, Fields, Principal};
use regex_lite::Regex;
use runs::{StepInfo, StepStatus};
use serde_json::{Value, json};

use crate::action::{ActionCx, ActionError, CxInner, Element, FailureKind, Left, check_args};
pub(crate) use crate::hooks::Decision;
use crate::hooks::{Around, watch};
use crate::intrinsic;
use crate::labels;
use crate::run::{RunCx, record};
use crate::sink::{Sink, log_id};
use crate::task;

/// How many times `on_fail` hooks may send a step back for another go.
/// Bounded so an agent that keeps answering "retry" cannot loop forever.
pub(crate) const MAX_RECOVERIES: u32 = 2;

#[derive(Clone, Debug)]
pub(crate) struct StepResult {
    pub status: StepStatus,
    pub tolerated: bool,
    pub output: Value,
    pub error: Option<String>,
    pub attempts: u32,
    pub started: i64,
    pub finished: i64,
    /// Set by the `end` action: stop the run here.
    pub end: Option<End>,
}

#[derive(Clone, Debug)]
pub(crate) struct End {
    pub failed: bool,
    pub reason: Option<String>,
}

impl End {
    /// What is written with the step that ended its run: that it did, and
    /// how. A run taken up after a restart knows nothing of its ending but
    /// what its steps' records say; without this it went on as one whose
    /// steps had all gone well, whatever it had ended as.
    fn kept(&self) -> Value {
        json!({ "ended": { "failed": self.failed, "reason": self.reason } })
    }

    /// The ending a step's record keeps, where it ended its run.
    fn of(saved: &Value) -> Option<End> {
        let ended = saved.get("ended")?;
        Some(End {
            failed: ended["failed"].as_bool().unwrap_or(false),
            reason: ended["reason"].as_str().map(str::to_string),
        })
    }
}

impl StepResult {
    pub fn from_info(info: &StepInfo) -> Self {
        StepResult {
            status: info.status,
            tolerated: info.tolerated,
            output: info.output.clone(),
            error: info.error.clone(),
            attempts: info.attempt,
            started: info.started,
            finished: info.finished.unwrap_or(info.started),
            end: End::of(&info.saved),
        }
    }

    /// How the step appears to expressions, as `steps.<id>` and `self`.
    pub fn view(&self) -> Value {
        view(self.status, &self.output, self.error.as_deref(), self.attempts, self.finished - self.started)
    }
}

pub(crate) use texts::ran::{backoff, duration_in as resolve_duration, view, with_self};

/// What an action call produced.
pub(crate) enum Done {
    Out(Value),
    End(End),
}

/// Everything needed to invoke one action.
pub(crate) struct Call<'a> {
    pub cx: &'a RunCx,
    /// The step's id, or empty for a hook that belongs to the run.
    pub step: &'a str,
    pub attempt: u32,
    pub scope: &'a Fields,
    pub sink: Arc<Sink>,
    pub cancel: Cancel,
    pub env: &'a Fields,
    /// What `with` says, as it was written: `set` works it out again from
    /// the state as it is when its turn comes ([`crate::intrinsic`]).
    pub with: &'a Value,
    pub saved: Option<Value>,
    /// Which element of a `for_each` this is for, when it is for one.
    pub element: Option<Element>,
    pub device: Option<&'a str>,
    /// Whether the step was under way when the conductor stopped and is
    /// being done again with nothing saved ([`ActionCx::again`]).
    pub again: bool,
    /// Which entry of what is done around a step this is, where it is one.
    pub entry: Option<usize>,
    /// Which list of those done around a step it is an entry of, and
    /// which of that list: `on_fail`, the first. Entries of two lists
    /// have the same number.
    pub list: Option<(&'static str, usize)>,
    /// Which rule that watches the step this is done by, and which time
    /// in this try of the step that rule has fired, from 1.
    pub rule: Option<(usize, usize)>,
    /// It is something done around the step, or by a rule that watches
    /// it, and not the step's own action: what it sets says nothing of
    /// the step having acted.
    pub around: bool,
    /// How long the step is given, in milliseconds, where it says.
    pub limit: Option<u64>,
}

pub(crate) async fn exec_step(
    cx: Arc<RunCx>,
    step: Step,
    scope: Fields,
    prior: Option<StepInfo>,
) -> StepResult {
    let journal = cx.engine.own().clone();
    let first_started = prior.as_ref().map_or_else(|| journal.now_ms(), |info| info.started);
    let fail_early = |message: String| {
        let (cx, step_id) = (cx.clone(), step.id.clone());
        async move {
            let now = cx.engine.own().now_ms();
            let source = cx.about(Some(&step_id));
            let said = json!({ "attempt": 1, "duration_ms": now - first_started, "status": "failed" });
            let says = vec![
                cx.engine.own_say("step.failed", source.clone(), json!({ "error": message, "attempt": 1 })),
                cx.engine.own_say("step.finished", source, said),
            ];
            record(
                &cx,
                &step_id,
                StepStatus::Failed,
                1,
                false,
                Value::Null,
                Some(message.clone()),
                Value::Null,
                first_started,
                Some(now),
                says,
            )
            .await;
            StepResult {
                status: StepStatus::Failed,
                tolerated: false,
                output: Value::Null,
                error: Some(message),
                attempts: 1,
                started: first_started,
                finished: now,
                end: None,
            }
        }
    };

    // A step that was already running when the conductor stopped has passed
    // its guard; evaluating it again against newer state could un-run it.
    if prior.is_none()
        && let Some(when) = &step.when
    {
        match when.test(&scope) {
            Ok(true) => {}
            Ok(false) => {
                let now = journal.now_ms();
                let info = StepInfo {
                    run: cx.run_id.clone(),
                    step: step.id.clone(),
                    status: StepStatus::Skipped,
                    attempt: 0,
                    tolerated: false,
                    output: Value::Null,
                    error: None,
                    saved: Value::Null,
                    started: now,
                    finished: Some(now),
                };
                cx.pass_over(info, cx.about(Some(&step.id)), json!({ "when": when.source })).await;
                return StepResult {
                    status: StepStatus::Skipped,
                    tolerated: false,
                    output: Value::Null,
                    error: None,
                    attempts: 0,
                    started: now,
                    finished: now,
                    end: None,
                };
            }
            Err(err) => return fail_early(format!("`when` could not be evaluated: {err}")).await,
        }
    }

    let timeout = match &step.timeout {
        None => None,
        Some(timing) => {
            match timing.fixed.map(Ok).unwrap_or_else(|| resolve_duration(&timing.source, &scope)) {
                Ok(millis) => Some(millis),
                Err(err) => return fail_early(format!("`timeout`: {err}")).await,
            }
        }
    };
    let timed = match crate::hooks::timed(&step, &scope) {
        Ok(timed) => timed,
        Err(err) => return fail_early(err).await,
    };
    let mut rules = Vec::new();
    for (index, watch) in step.watch.iter().enumerate() {
        if let WatchCond::Log(pattern) = &watch.cond {
            match Regex::new(pattern) {
                Ok(regex) => rules.push((index, regex)),
                Err(err) => {
                    return fail_early(format!("watch rule `log: {pattern}` is not a valid pattern: {err}"))
                        .await;
                }
            }
        }
    }

    let mut resuming = prior.is_some();
    let mut attempt = prior.as_ref().map_or(0, |info| info.attempt.saturating_sub(1));
    let mut saved = prior.and_then(|info| (!info.saved.is_null()).then_some(info.saved));
    let mut tries = 0u32;
    let mut recoveries = 0u32;
    // What was kept of what is done around it, and of its having acted,
    // read before its record is written anew; nothing is read for a step
    // that begins.
    let mut around = match resuming && cx.kept() {
        true => Around::kept(&cx, &step.id).await,
        false => Around::default(),
    };
    let source = cx.about(Some(&step.id));
    let labels = |name: &str| {
        [
            ("blueprint", name.to_string()),
            ("step", step.id.clone()),
            ("action", step.kind.target().to_string()),
        ]
    };

    if !resuming {
        cx.hush(&step);
    }
    loop {
        attempt += 1;
        tries += 1;
        let checkpoint = saved.clone().unwrap_or(Value::Null);
        // A try that begins has nothing done around it yet.
        if !resuming {
            around = Around::default();
        }
        around.of_try(attempt);
        let started = json!({
            "attempt": attempt, "verb": step.kind.verb(), "target": step.kind.target(), "resumed": resuming,
        });
        let started = cx.engine.own_say("step.started", source.clone(), started);
        // Where a try began is written on the step's row, read back where
        // it is done again after a stop; not where it was taken up again.
        let says = vec![if resuming { started } else { started.begins_step() }];
        record(
            &cx,
            &step.id,
            StepStatus::Running,
            attempt,
            false,
            Value::Null,
            None,
            checkpoint,
            first_started,
            None,
            says,
        )
        .await;

        let step_cancel = cx.cancel.child();
        let (sink, hits) = Sink::new(
            journal.clone(),
            cx.engine.logs().clone(),
            log_id(&cx.run_id, &step.id),
            source.clone(),
            rules.clone(),
        );
        let sink = Arc::new(sink);
        if attempt > 1 || resuming {
            sink.line(
                's',
                &format!("attempt {attempt}{}", if resuming { " (resumed after a restart)" } else { "" }),
            );
        }
        let running_view =
            view(StepStatus::Running, &Value::Null, None, attempt, journal.now_ms() - first_started);
        // Taken up among what is done as it starts, its action never began.
        let starting = with_self(&scope, &step.id, running_view);
        let acting = around.acted().is_some()
            || around.starting(&cx, &step.id, &step.on_start, &starting, step_cancel.clone()).await;

        let child: Arc<Mutex<Option<String>>> = Arc::default();
        // One whose action had ended before the conductor stopped is not
        // acted again: it goes on with what is done at its ending.
        let acted = around.acted().is_some();
        let outcome = if let Some(kept) = around.acted() {
            outcome_of(kept)
        } else if cx.cancel.is_cancelled() {
            // Cut before its action began: what is ended begins nothing.
            Err(ActionError::cancelled())
        } else {
            let body = run_body(
                &cx,
                &step,
                &scope,
                attempt,
                sink.clone(),
                step_cancel.clone(),
                saved.take(),
                resuming && acting,
                &child,
            );
            tokio::pin!(body);
            // Timed from before its action begins, the rules and the limit:
            // a program it starts may be at work, and the clock have moved,
            // before either is first looked at.
            let (clock, begun) = (journal.clock().clone(), journal.now_ms());
            let watching = watch(&cx, &step, (&timed, begun), &scope, hits, step_cancel.clone());
            let limit = async move {
                match timeout {
                    Some(millis) => {
                        let gone = (clock.now_ms() - begun).max(0) as u64;
                        clock.sleep(Duration::from_millis(millis.saturating_sub(gone))).await
                    }
                    None => std::future::pending().await,
                }
            };
            // In this order, where more than one is there: an action
            // that has ended ended as it did, whatever came as it ended,
            // and one that is cut is cut, whatever else ran out with it.
            // Left to chance, such a step was cancelled three in four.
            let cut = tokio::select! {
                biased;
                result = &mut body => Ok(result),
                () = cx.cancel.cancelled() => Err(ActionError::cancelled()),
                reason = watching => Err(ActionError::new(reason)),
                () = limit => Err(ActionError::timed_out(&duration::format(timeout.unwrap_or(0)))),
            };
            match cut {
                Ok(result) => result,
                Err(error) => {
                    // A task cut short has steps of its own under way. They
                    // are told to stop and waited for, so that each is on
                    // record as it ended before the task says how it did.
                    if step.kind == StepKind::Task {
                        step_cancel.cancel();
                        let _ = body.await;
                    }
                    Err(error)
                }
            }
        };
        step_cancel.cancel();
        resuming = false;
        // A `call` that did not finish takes the callee it made down with
        // it; one that was there before the call is not kept here, and is
        // left as it is (`child::start`).
        let orphan = child.lock().expect("step lock").take();
        if let (Err(_), Some(orphan), StepKind::Call(_)) = (&outcome, orphan, &step.kind) {
            let _ = cx.engine.stop(&orphan).await;
        }

        let now = journal.now_ms();
        let elapsed = now - first_started;
        match outcome {
            Ok(done) => {
                let (output, end) = match done {
                    Done::Out(output) => (output, None),
                    Done::End(end) => (Value::Null, Some(end)),
                };
                // What is done when it went well, and then what is done when
                // it has ended however it went.
                let done_view =
                    with_self(&scope, &step.id, view(StepStatus::Ok, &output, None, attempt, elapsed));
                let (well, after) = (&step.on_success, &step.on_finish);
                if !(well.is_empty() && after.is_empty()) {
                    let how = json!({ "ok": true, "output": output, "ended": end.as_ref().map(End::kept) });
                    around.act(&cx, &step.id, how).await;
                }
                around
                    .ending(&cx, &step.id, 0, "on_success", well, &done_view, false, after.is_empty())
                    .await;
                around.ending(&cx, &step.id, well.len(), "on_finish", after, &done_view, false, true).await;
                let finished = journal.now_ms();
                labels::measure(&cx, &step, &labels(&cx.blueprint.name), finished - first_started, true);
                let said =
                    json!({ "attempt": attempt, "duration_ms": finished - first_started, "status": "ok" });
                let says = vec![cx.engine.own_say("step.finished", source, said)];
                record(
                    &cx,
                    &step.id,
                    StepStatus::Ok,
                    attempt,
                    false,
                    output.clone(),
                    None,
                    end.as_ref().map_or(Value::Null, End::kept),
                    first_started,
                    Some(finished),
                    says,
                )
                .await;
                return StepResult {
                    status: StepStatus::Ok,
                    tolerated: false,
                    output,
                    error: None,
                    attempts: attempt,
                    started: first_started,
                    finished,
                    end,
                };
            }
            Err(error) if error.kind == FailureKind::Cancelled && cx.cancel.is_cancelled() => {
                let said = json!({ "attempt": attempt, "duration_ms": elapsed, "status": "cancelled" });
                let says = vec![
                    cx.engine.own_say("step.cancelled", source.clone(), json!({ "attempt": attempt })),
                    cx.engine.own_say("step.finished", source, said),
                ];
                record(
                    &cx,
                    &step.id,
                    StepStatus::Cancelled,
                    attempt,
                    false,
                    Value::Null,
                    None,
                    Value::Null,
                    first_started,
                    Some(now),
                    says,
                )
                .await;
                return StepResult {
                    status: StepStatus::Cancelled,
                    tolerated: false,
                    output: Value::Null,
                    error: Some("cancelled".into()),
                    attempts: attempt,
                    started: first_started,
                    finished: now,
                    end: None,
                };
            }
            Err(error) => {
                // What its database refused as the daemon went is no
                // failure of the step's: nothing more is written of it.
                if error.gone() && (journal.is_shut() || cx.engine.is_halting()) {
                    return std::future::pending().await;
                }
                sink.line('s', &format!("failed: {}", error.message));
                if let Some(retry) = step.retry.as_ref().filter(|retry| !acted && tries <= retry.max) {
                    let delay = backoff(retry, tries);
                    cx.say(
                        "step.retrying",
                        source.clone(),
                        json!({ "attempt": attempt, "error": error.message, "in_ms": delay, "by": "retry" }),
                    )
                    .await;
                    tokio::select! {
                        () = journal.clock().sleep(Duration::from_millis(delay)) => continue,
                        () = cx.cancel.cancelled() => {}
                    }
                    // Cancelled while waiting to retry: fall through and
                    // record the failure that got us here.
                }
                let timed_out = error.kind == FailureKind::TimedOut;
                let hooks =
                    if timed_out && !step.on_timeout.is_empty() { &step.on_timeout } else { &step.on_fail };
                let failed_view =
                    view(StepStatus::Failed, &error.data, Some(&error.message), attempt, elapsed);
                let after = &step.on_finish;
                let decision = if cx.cancel.is_cancelled() {
                    None
                } else {
                    if !(hooks.is_empty() && after.is_empty()) {
                        let how = json!({
                            "ok": false, "message": error.message, "data": error.data, "timed_out": timed_out,
                        });
                        around.act(&cx, &step.id, how).await;
                    }
                    let failed = with_self(&scope, &step.id, failed_view);
                    let list =
                        if timed_out && !step.on_timeout.is_empty() { "on_timeout" } else { "on_fail" };
                    around.ending(&cx, &step.id, 0, list, hooks, &failed, true, after.is_empty()).await
                };
                if decision == Some(Decision::Retry) && recoveries < MAX_RECOVERIES {
                    recoveries += 1;
                    tries = 0;
                    cx.say(
                        "step.retrying",
                        source.clone(),
                        json!({ "attempt": attempt, "error": error.message, "in_ms": 0, "by": "hook" }),
                    )
                    .await;
                    continue;
                }
                let tolerated = step.allow_failure || decision == Some(Decision::Continue);
                // It has ended, and is not to be tried again: what is done
                // after any ending is done after what was done about this one.
                if !cx.cancel.is_cancelled() {
                    let ended = view(StepStatus::Failed, &error.data, Some(&error.message), attempt, elapsed);
                    let ended = with_self(&scope, &step.id, ended);
                    around.ending(&cx, &step.id, hooks.len(), "on_finish", after, &ended, false, true).await;
                }
                // A step that names its failure fails under that name, so
                // that what comes after can tell this failure from another.
                let mut error = error;
                if let Some(code) = &step.fail_code {
                    match &mut error.data {
                        Value::Object(data) => {
                            data.insert("code".into(), json!(code));
                        }
                        other => *other = json!({ "code": code }),
                    }
                }
                let finished = journal.now_ms();
                labels::measure(&cx, &step, &labels(&cx.blueprint.name), finished - first_started, false);
                let said = json!({ "attempt": attempt, "duration_ms": finished - first_started, "status": "failed" });
                let says = vec![
                    cx.engine.own_say(
                        "step.failed",
                        source.clone(),
                        json!({
                            "attempt": attempt,
                            "error": error.message,
                            "code": step.fail_code,
                            "timed_out": timed_out,
                            "tolerated": tolerated,
                            "duration_ms": finished - first_started,
                        }),
                    ),
                    cx.engine.own_say("step.finished", source, said),
                ];
                record(
                    &cx,
                    &step.id,
                    StepStatus::Failed,
                    attempt,
                    tolerated,
                    error.data.clone(),
                    Some(error.message.clone()),
                    Value::Null,
                    first_started,
                    Some(finished),
                    says,
                )
                .await;
                return StepResult {
                    status: StepStatus::Failed,
                    tolerated,
                    output: error.data,
                    error: Some(error.message),
                    attempts: attempt,
                    started: first_started,
                    finished,
                    end: None,
                };
            }
        }
    }
}

/// How a step's action ended, as it was kept before what is done at its
/// ending began ([`Around::act`]).
fn outcome_of(kept: &Value) -> Result<Done, ActionError> {
    if kept["ok"] == true {
        return Ok(match End::of(&kept["ended"]) {
            Some(end) => Done::End(end),
            None => Done::Out(kept["output"].clone()),
        });
    }
    Err(ActionError {
        message: kept["message"].as_str().unwrap_or_default().to_string(),
        data: kept["data"].clone(),
        kind: if kept["timed_out"] == true { FailureKind::TimedOut } else { FailureKind::Failed },
    })
}

/// Which element of a `for_each` a step is being run for, and what the
/// children started for the elements before it were called.
pub(crate) struct Each<'a> {
    pub index: usize,
    /// The list the loop is over.
    pub items: &'a Arc<Vec<Value>>,
    pub named: &'a Mutex<Vec<String>>,
    /// Children an earlier fault misnamed ([`crate::child`]).
    pub misnamed: &'a crate::child::Misnamed,
}

/// The step's work: once, or once per element of `for_each`.
#[allow(clippy::too_many_arguments)]
async fn run_body(
    cx: &RunCx,
    step: &Step,
    scope: &Fields,
    attempt: u32,
    sink: Arc<Sink>,
    cancel: Cancel,
    saved: Option<Value>,
    resuming: bool,
    child: &Mutex<Option<String>>,
) -> Result<Done, ActionError> {
    let Some(each) = &step.for_each else {
        return run_once(cx, step, scope, attempt, sink, cancel, saved, resuming, child, None).await;
    };
    let store = &cx.engine.inner.store;
    // A loop that was under way when the conductor stopped goes on over the
    // list it began with, which may not be what the expression comes to
    // now, from the element it was at.
    let was = saved.as_ref().filter(|_| resuming).and_then(Left::of);
    let going_on = was.is_some();
    let (items, mut own) = match was {
        Some(Left { items, own }) => (items, own),
        None => {
            let items = match each.expr.eval(scope).map_err(|err| format!("`for_each`: {err}"))? {
                Value::Array(items) => items,
                Value::Null => Vec::new(),
                // So many times: from 1 to the number, as `range()` counts
                // them out, so that a text says how many and calls nothing.
                count @ Value::Number(_) => blueprint::expr::up_to(&count)
                    .map_err(|why| ActionError::new(format!("`for_each` over a number {why}")))?,
                other => {
                    return Err(ActionError::new(format!(
                        "`for_each` must produce a list, or a number to count up to; got {other}"
                    )));
                }
            };
            // Begun, or begun again by a retry: from its first element.
            if cx.kept() {
                store.forget_elements(&cx.run_id, &step.id).await?;
                cx.engine.save_checkpoint(&cx.run_id, &step.id, Element::around(&items, None)).await?;
            }
            (items, None)
        }
    };
    let items = Arc::new(items);
    let (mut outputs, begun) = match going_on {
        true => store.elements(&cx.run_id, &step.id).await?,
        false => (Vec::new(), false),
    };
    outputs.truncate(items.len());
    let done = outputs.len();
    let named = Mutex::new(Vec::new());
    let misnamed = crate::child::misnamed(cx, step).await;
    // Each element of a task has steps of the same names, kept apart by
    // forgetting an element's as it is written down as finished.
    let inside = (step.kind == StepKind::Task).then(|| blueprint::within::task(&step.id));
    for (index, item) in items.iter().cloned().enumerate() {
        let mut scope = scope.clone();
        scope.insert("item".into(), item);
        scope.insert("index".into(), index.into());
        if index < done {
            // Finished before the conductor stopped. What its child was
            // called is still a name no later element may come to.
            if let Some(key) = crate::child::named(step, &scope) {
                named.lock().expect("step lock").push(key);
            }
            continue;
        }
        // The element it was in the middle of is picked up as a step is:
        // with what its action saved, or as one interrupted with nothing
        // saved. One it had not come to, and every one after, begins.
        let at_it = begun && index == done;
        let saved = own.take().filter(|(at, _)| at_it && *at == index).map(|(_, saved)| saved);
        if !at_it && cx.kept() {
            store.begin_element(&cx.run_id, &step.id, index, cx.engine.own().last_seq()).await?;
        }
        let each = Each { index, items: &items, named: &named, misnamed: &misnamed };
        match run_once(
            cx,
            step,
            &scope,
            attempt,
            sink.clone(),
            cancel.clone(),
            saved,
            at_it,
            child,
            Some(each),
        )
        .await?
        {
            Done::Out(output) => {
                match (cx.kept(), &inside) {
                    (true, _) => {
                        // What was passed over inside it and is not yet
                        // written goes with the rest of the element's.
                        if let Some(inside) = &inside {
                            cx.forget_held(inside);
                        }
                        store.put_element(&cx.run_id, &step.id, index, &output, inside.clone()).await?;
                    }
                    // The next element's steps have the same names.
                    (false, Some(inside)) => cx.forget_steps(inside).await?,
                    (false, None) => {}
                }
                outputs.push(output);
            }
            end @ Done::End(_) => return Ok(end),
        }
    }
    Ok(Done::Out(Value::Array(outputs)))
}

#[allow(clippy::too_many_arguments)]
async fn run_once(
    cx: &RunCx,
    step: &Step,
    scope: &Fields,
    attempt: u32,
    sink: Arc<Sink>,
    cancel: Cancel,
    saved: Option<Value>,
    resuming: bool,
    child: &Mutex<Option<String>>,
    each: Option<Each<'_>>,
) -> Result<Done, ActionError> {
    let args = match template::render(&step.with, scope)
        .map_err(|err| format!("`with` could not be filled in: {err}"))?
    {
        Value::Object(args) => args,
        Value::Null => Fields::new(),
        other => return Err(ActionError::new(format!("`with` must produce a mapping, got {other}"))),
    };
    let device = match &step.device {
        Some(device) => Some(
            template::render(&Value::String(device.clone()), scope)
                .map_err(|err| format!("`device`: {err}"))?
                .as_str()
                .unwrap_or_default()
                .to_string(),
        ),
        None => None,
    };
    match &step.kind {
        StepKind::Task => {
            task::run(cx, step, scope, cancel, resuming, attempt, each.map(|each| each.index)).await
        }
        StepKind::Do(action) => {
            // Interrupted by a restart with nothing saved: only repeat what
            // is safe to repeat, by its action's word or its step's (`again`).
            if resuming && saved.is_none() && !step.again {
                let idempotent = blueprint::intrinsic(action)
                    .map(|spec| spec.idempotent)
                    .or_else(|| cx.engine.registry().get(action).map(|handler| handler.spec().idempotent))
                    .unwrap_or(false);
                if !idempotent {
                    return Err(ActionError::new(
                        "was interrupted by a restart, and this action is not safe to repeat automatically",
                    ));
                }
            }
            sink.handed(None, &args);
            let element = each.map(|each| Element { items: each.items.clone(), at: each.index });
            let again = resuming && saved.is_none();
            let call = Call {
                cx,
                step: &step.id,
                attempt,
                scope,
                sink,
                cancel,
                env: &step.env,
                with: &step.with,
                saved,
                element,
                device: device.as_deref(),
                again,
                entry: None,
                list: None,
                rule: None,
                around: false,
                limit: step
                    .timeout
                    .as_ref()
                    .and_then(|timing| timing.fixed.or_else(|| resolve_duration(&timing.source, scope).ok())),
            };
            dispatch(call, action, args).await
        }
        StepKind::Call(name) | StepKind::Spawn(name) => {
            let asked = crate::child::Asked { name, args, each, sink: &sink, child, saved, again: resuming };
            crate::child::start(cx, step, scope, asked).await
        }
    }
}

/// Refuses an action the blueprint did not declare or its principal may not use.
pub(crate) fn permit(
    blueprint: &Blueprint,
    principal: &Principal,
    capability: &str,
) -> Result<(), ActionError> {
    texts::ran::permit(blueprint, principal, capability).map_err(ActionError::new)
}

/// Invokes one action: built in, registered here, or on another device.
pub(crate) async fn dispatch(call: Call<'_>, action: &str, args: Fields) -> Result<Done, ActionError> {
    let cx = call.cx;
    if let Some(spec) = blueprint::intrinsic(action) {
        check_args(&spec, &args)?;
        return intrinsic::run(call, action, args).await;
    }
    let Some(handler) = cx.engine.registry().get(action) else {
        // An action that was here when the blueprint was checked may have
        // gone since, with the package that brought it.
        return Err(ActionError::new(
            cx.engine
                .registry()
                .absent(action, true)
                .unwrap_or_else(|| format!("there is no action called `{action}` on this device")),
        ));
    };
    if let Some(capability) = &handler.spec().capability {
        permit(&cx.blueprint, &cx.principal, capability)?;
    }
    check_args(handler.spec(), &args)?;

    let mut env = std::collections::BTreeMap::new();
    let mut secrets = std::collections::BTreeMap::new();
    for (key, value) in cx.blueprint.env.iter().chain(call.env) {
        let rendered = template::render(value, call.scope).map_err(|err| format!("env `{key}`: {err}"))?;
        // A secret is named, not written: the name goes on to whatever
        // starts a program, which reads the value then. A step's word for
        // a variable replaces the blueprint's, whichever kind each is.
        match blueprint::secrets::named(&rendered) {
            // Named where it is written, and never by what an expression
            // came to: what a param or an event carries must not choose
            // which secret a program is handed.
            Some(name) if blueprint::secrets::named(value).is_none() => {
                return Err(ActionError::new(format!(
                    "env `{key}`: an expression came to naming the secret `{name}`. A secret is named \
                     where it is written, as `{{ secret: {name} }}`"
                )));
            }
            // Named by the empty text, there is none and the variable is
            // not set: how a param that may name one says it names none.
            Some("") => {
                env.remove(key);
                secrets.remove(key);
            }
            // The environment's of that name where the instance runs under
            // one that lists it, else the factory's (`crate::environs`).
            Some(name) => {
                env.remove(key);
                let sought =
                    cx.engine.secret_for(&cx.live, name).map_err(|why| format!("env `{key}`: {why}"))?;
                secrets.insert(key.clone(), sought);
            }
            None => {
                secrets.remove(key);
                env.insert(key.clone(), blueprint::expr::display(&rendered));
            }
        }
    }
    if !secrets.is_empty() {
        permit(&cx.blueprint, &cx.principal, "secrets:use")?;
    }
    // What a program a step starts is told it runs under, as an
    // expression's `environment.name`; nothing for none.
    let under = cx.live.meta().environment;
    if !under.is_empty() {
        env.insert(environments::held::ENV_VAR.to_string(), under);
    }
    let (around, doing) = crate::action::doing(&cx.under, &call);
    let action_cx = ActionCx {
        inner: Arc::new(CxInner {
            engine: cx.engine.clone(),
            instance: cx.live.id.clone(),
            blueprint: cx.blueprint.name.clone(),
            project: cx.live.meta().project,
            run: cx.run_id.clone(),
            step: call.step.to_string(),
            attempt: call.attempt,
            principal: cx.principal.clone(),
            cancel: call.cancel,
            env,
            sink: call.sink,
            labels: cx.labels.clone(),
            element: call.element,
            secrets,
            kept: cx.kept(),
            again: call.again,
            around,
            doing,
            limit: call.limit,
        }),
    };
    let here = cx.engine.own().device();
    if let Some(device) =
        call.device.filter(|device| !device.is_empty() && *device != here && *device != "local")
    {
        // Another device is the fleet's to use: whoever applied this holds
        // what changing the fleet takes, or the step stays where it is.
        if !cx.principal.can("fleet:write") {
            return Err(ActionError::new(format!(
                "{} may not send a step to another device (needs `fleet:write`)",
                cx.principal.name
            )));
        }
        let Some(remote) = cx.engine.remote() else {
            return Err(ActionError::new(format!(
                "this step is for device `{device}`, but this device has no gateway to reach it"
            )));
        };
        // A secret goes with it as the name it has, and nothing of what
        // this device keeps under that name: the device that runs the
        // program hands over what it keeps itself.
        return remote.run_action(device, action, args, action_cx).await.map(Done::Out);
    }
    let work = match call.saved {
        Some(saved) => handler.resume(action_cx, args, saved),
        None => handler.run(action_cx, args),
    };
    work.await.map(Done::Out)
}
