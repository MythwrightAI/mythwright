//! What is done around a step: its hooks and its watch rules.
//!
//! A hook is something done when a step starts, finishes or fails, or when
//! a run or an instance ends: an action like any other, or one of the words
//! by which a failure hook says what becomes of its step. A hook that fails
//! is said and changes nothing. A watch rule is a hook with a condition on
//! a step that is still running: so long after it started, every so often,
//! or when its output says something.
//!
//! Two lists are done by themselves and around no step: what a text says
//! is done as one that is running takes it (`on_update`), and as an
//! instance stops (`on_stop`). They are kept as the others are
//! ([`Around::alone`]): how far each has come is written as each entry is
//! done, in the run that carries the instance over or, for a stop, which
//! has no run, in the instance's own row.

use std::sync::Arc;
use std::time::Duration;

use blueprint::{Hook, Step, WatchCond, duration, template};
use kernel::{Cancel, Fields};
use runs::{Say, StepStatus, to_say};
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::run::RunCx;
use crate::sink::{Sink, log_id};
use crate::step::{Call, Done, dispatch, view, with_self};

/// What a failure hook decided should happen to its step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    Retry,
    Continue,
    Fail,
}

impl Decision {
    fn parse(word: &str) -> Option<Decision> {
        match word {
            "retry" => Some(Decision::Retry),
            "continue" => Some(Decision::Continue),
            "fail" => Some(Decision::Fail),
            _ => None,
        }
    }
}

/// Does one hook. `since` says it may have been under way when the
/// conductor stopped, and is the journal's last number before it began,
/// where that is known: an event it said after that is not said again,
/// and an action is told it is being done again. Where it is not known
/// where it began, nothing it said is looked for and an event is said. `entry` is which of those around its step it
/// is, counted through them; `list` which list it is an entry of and which
/// of that list; `attempt` which try of the step it is done around.
///
/// Gives what it decided, and, where it could not be done, what is to be
/// said of that: said by whoever keeps that the entry is done, in the
/// same write ([`Around`]).
#[allow(clippy::too_many_arguments)]
async fn run_hook(
    cx: &RunCx,
    step: &str,
    hook: &Hook,
    scope: &Fields,
    cancel: Option<Cancel>,
    may_decide: bool,
    since: Option<Option<u64>>,
    entry: Option<usize>,
    list: Option<(&'static str, usize)>,
    attempt: u32,
) -> (Option<Decision>, Option<Say>) {
    match enter(cx, step, hook, scope, cancel, since, entry, list, attempt, None).await {
        Ok(Entered::Decided(decided)) => (may_decide.then_some(decided), None),
        Ok(Entered::Gave(output)) => {
            let decided = output.get("decision").and_then(Value::as_str).and_then(Decision::parse);
            (decided.filter(|_| may_decide), None)
        }
        Ok(Entered::Passed) => (None, None),
        Err(Unentered::Written(error) | Unentered::Failed(error)) => {
            (None, Some(could_not(cx, step, hook, error)))
        }
    }
}

/// What is said of a hook that could not be done.
fn could_not(cx: &RunCx, step: &str, hook: &Hook, error: String) -> Say {
    let source = cx.about((!step.is_empty()).then_some(step));
    to_say("hook.failed", source, json!({ "action": hook.action, "error": error }))
}

/// What became of an entry that could be done.
enum Entered {
    /// Its `when` did not hold; or it was under way when the conductor
    /// stopped, and what it says was said then.
    Passed,
    /// It is one of the words by which a failure hook says what becomes
    /// of its step.
    Decided(Decision),
    /// Its action was done, and gave this.
    Gave(Value),
}

/// Why an entry could not be done.
enum Unentered {
    /// As it is written: its `when` or its `with` could not be read.
    Written(String),
    /// Its action failed.
    Failed(String),
}

/// Does one entry of a list, whichever list it is: around a step, as a
/// run ends, or one that is done by itself; or what a rule that watches
/// a step does as it fires (`rule`, [`fire`]).
#[allow(clippy::too_many_arguments)]
async fn enter(
    cx: &RunCx,
    step: &str,
    hook: &Hook,
    scope: &Fields,
    cancel: Option<Cancel>,
    since: Option<Option<u64>>,
    entry: Option<usize>,
    list: Option<(&'static str, usize)>,
    attempt: u32,
    rule: Option<(usize, usize)>,
) -> Result<Entered, Unentered> {
    if let Some(when) = &hook.when {
        match when.test(scope) {
            Ok(true) => {}
            Ok(false) => return Ok(Entered::Passed),
            Err(err) => return Err(Unentered::Written(format!("`when`: {err}"))),
        }
    }
    if let Some(decided) = Decision::parse(&hook.action) {
        return Ok(Entered::Decided(decided));
    }
    let args = match template::render(&hook.with, scope) {
        Ok(Value::Object(args)) => args,
        Ok(Value::Null) => Fields::new(),
        Ok(other) => {
            return Err(Unentered::Written(format!("`with` must produce a mapping, got {other}")));
        }
        Err(err) => return Err(Unentered::Written(format!("`with`: {err}"))),
    };
    if let (Some(after), "emit") = (since, hook.action.as_str())
        && crate::intrinsic::entry_said(cx, step, &args, after, list).await
    {
        return Ok(Entered::Passed);
    }
    if let (Some(rule), "emit") = (rule, hook.action.as_str())
        && crate::intrinsic::rule_said(cx, step, &args, rule).await
    {
        return Ok(Entered::Passed);
    }
    let journal = cx.engine.own().clone();
    let source = cx.about((!step.is_empty()).then_some(step));
    let (sink, _) =
        Sink::new(journal, cx.engine.logs().clone(), log_id(&cx.run_id, step), source, Vec::new());
    let call = Call {
        cx,
        step,
        attempt,
        scope,
        sink: Arc::new(sink),
        cancel: cancel.unwrap_or_else(|| cx.live.cancel.child()),
        env: &Fields::new(),
        with: &hook.with,
        saved: None,
        element: None,
        device: None,
        again: since.is_some(),
        entry,
        list,
        rule,
        around: true,
        limit: None,
    };
    match dispatch(call, &hook.action, args).await {
        Ok(Done::Out(output)) => Ok(Entered::Gave(output)),
        Ok(Done::End(_)) => Ok(Entered::Gave(Value::Null)),
        Err(error) => Err(Unentered::Failed(error.message)),
    }
}

/// How far what is done around a step has come, or what is done as a run
/// ends: which entries were done, and how the step's action ended.
///
/// It is kept in the step's own record (the run's, for a run), written as
/// each entry is done, so that a step or a run taken up among them after a
/// restart goes on from the next entry and does none twice; the one it was
/// in the middle of is done again, looking first for an event it had said.
/// A step whose action had ended is not acted again: how it ended is kept
/// before what is done at its ending begins. Nothing is written for a step
/// or a run with nothing done around it, nor for a run that keeps nothing
/// as it goes; and nothing is read but when one is taken up.
#[derive(Clone, Debug, Default)]
pub(crate) struct Around {
    /// Entries of `on_start` done.
    start: usize,
    /// How the step's action ended, once it has.
    acted: Option<Value>,
    /// Entries done at the ending, counted through its lists in the order
    /// they are done.
    end: usize,
    /// What those that may decide have decided so far.
    decision: Option<Decision>,
    /// The journal's last number when this was last kept; nothing, for a
    /// step with nothing kept whose beginning is no longer in the journal.
    at: Option<u64>,
    /// The entry it has come to may have been under way at the stop.
    taken_up: bool,
    /// Which try of the step this is around.
    attempt: u32,
    /// Why its instance is being stopped, for what is done as it stops:
    /// kept with the first of what is kept, for whoever finishes it.
    why: Option<String>,
    /// Why a list that ends at the first entry that cannot be done has
    /// ended, where it has: taken up after that, it has failed, and the
    /// entry is not done again to find that out.
    failed: Option<String>,
    /// Where another journal had got to as the entry it has come to was
    /// about to say something there ([`keep_elsewhere`]).
    elsewhere: Option<Value>,
}

/// Entries at an ending are numbered after any there could be at a start.
const AT_THE_END: usize = 1 << 16;

/// The record that says entry `entry` of what is done around `step` (the
/// run, with no step; the instance, with no run) is done: what something
/// it sets is written with.
pub(crate) fn done_entry(cx: &RunCx, step: String, entry: usize) -> runs::Done {
    let (start, done) = (entry < AT_THE_END, entry % AT_THE_END + 1);
    // What is done as an instance stops is done in no run: how far it
    // has come is in the instance's own row.
    if cx.run_id.is_empty() {
        return runs::Done::Stopping { instance: cx.live.id.clone(), done };
    }
    runs::Done::Entry { run: cx.run_id.clone(), step: (!step.is_empty()).then_some(step), start, done }
}

/// Where in the journal what is done as `step` starts was all done, for
/// a step that has `entries` of it and whose action was under way when
/// the conductor stopped: what the step's own action said, it said after
/// that. An entry may say what the step says, from the same step, and
/// where the step began is before both. Nothing, where that was not kept
/// (a run that keeps nothing; a step some entry of which was still to do,
/// whose action is not being done again at all).
pub(crate) async fn started(cx: &RunCx, step: &str, entries: usize) -> Option<u64> {
    let kept = cx.engine.inner.store.kept_hooks(&cx.run_id, Some(step)).await.ok()?;
    let all = kept["start"].as_u64() == Some(entries as u64) && kept["acted"].is_null();
    kept["at"].as_u64().filter(|_| all)
}

/// What is kept of what is done around `step` (of the run, with no
/// step; of the instance as it stops, with no run), as it is written.
async fn kept_around(cx: &RunCx, step: &str) -> kernel::Result<Value> {
    let store = &cx.engine.inner.store;
    match cx.run_id.is_empty() {
        // What is done as an instance stops is done in no run.
        true => store.kept_stop(&cx.live.id).await,
        false => store.kept_hooks(&cx.run_id, (!step.is_empty()).then_some(step)).await,
    }
}

/// Keeps `place` with how far what is done around `step` has come:
/// where another journal had got to as an entry is about to say
/// something there, and which entry. Kept until that entry is done.
pub(crate) async fn keep_elsewhere(cx: &RunCx, step: &str, place: Value) -> kernel::Result<()> {
    let mut kept = kept_around(cx, step).await?;
    if !kept.is_object() {
        kept = json!({});
    }
    kept["elsewhere"] = place;
    let store = &cx.engine.inner.store;
    match cx.run_id.is_empty() {
        true => store.save_stop(&cx.live.id, kept).await,
        false => store.save_hooks(&cx.run_id, (!step.is_empty()).then_some(step), kept).await,
    }
}

/// Keeps where a journal had got to as the `nth` firing of rule `index`
/// that watches `step` is about to say something there. Kept for the
/// try of the step, which is all that is kept of a rule, and taken away
/// with the rest when another try begins: done again after a stop, the
/// step's rules fire again from its new beginning, and the same firing
/// finds what it had said ([`crate::intrinsic::rule_said`]). Kept by
/// itself, since the step's own action may keep something beside it in
/// the same moment.
pub(crate) async fn keep_watched(
    cx: &RunCx,
    step: &str,
    (index, nth): (usize, usize),
    place: Value,
) -> kernel::Result<()> {
    let path = format!("$.watched.\"{index}/{nth}\"");
    cx.engine.inner.store.keep_at(&cx.run_id, step, path, place).await
}

/// What [`keep_watched`] kept of that firing, where anything was.
pub(crate) async fn kept_watched(cx: &RunCx, step: &str, (index, nth): (usize, usize)) -> Option<Value> {
    let kept = kept_around(cx, step).await.ok()?;
    kept["watched"].get(format!("{index}/{nth}")).filter(|place| !place.is_null()).cloned()
}

/// What [`keep_elsewhere`] kept, where it is still there.
pub(crate) async fn kept_elsewhere(cx: &RunCx, step: &str) -> Option<Value> {
    kept_around(cx, step).await.ok()?.get("elsewhere").filter(|place| !place.is_null()).cloned()
}

impl Around {
    /// What was kept of a step that is being taken up; of the run, with
    /// no step.
    pub(crate) async fn kept(cx: &RunCx, step: &str) -> Around {
        let of = (!step.is_empty()).then_some(step);
        let kept = kept_around(cx, step).await.unwrap_or(Value::Null);
        // A run with nothing kept had not come to its ending when the
        // conductor stopped, however long ago it was taken up: nothing of
        // what is done there had begun, and none of it is done again.
        // And so with a stop of which only its having begun is kept.
        if (kept.is_null() || kept.get("end").is_none()) && of.is_none() {
            return Around::default();
        }
        // With nothing kept yet, what a step said it said after it began.
        let at = match (kept["at"].as_u64(), of) {
            (Some(at), _) => Some(at),
            (None, Some(step)) => crate::intrinsic::began(cx, step).await,
            (None, None) => Some(0),
        };
        Around {
            start: kept["start"].as_u64().unwrap_or(0) as usize,
            acted: kept.get("acted").filter(|acted| !acted.is_null()).cloned(),
            end: kept["end"].as_u64().unwrap_or(0) as usize,
            decision: kept["decision"].as_str().and_then(Decision::parse),
            at,
            taken_up: true,
            attempt: 0,
            why: None,
            failed: kept["failed"].as_str().map(str::to_string),
            elsewhere: kept.get("elsewhere").filter(|place| !place.is_null()).cloned(),
        }
    }

    /// Says which try of its step this is done around: what is done as
    /// the second try starts is not what was done as the first did.
    pub(crate) fn of_try(&mut self, attempt: u32) {
        self.attempt = attempt;
    }

    /// How the step's action ended, where that was kept.
    pub(crate) fn acted(&self) -> Option<&Value> {
        self.acted.as_ref()
    }

    async fn write(&self, cx: &RunCx, step: &str, says: Vec<Say>) {
        let decision = self.decision.map(|decision| match decision {
            Decision::Retry => "retry",
            Decision::Continue => "continue",
            Decision::Fail => "fail",
        });
        let mut kept = json!({
            "start": self.start, "acted": self.acted, "end": self.end, "decision": decision, "at": self.at,
        });
        if let Some(why) = &self.why {
            kept["why"] = json!(why);
        }
        if let Some(failed) = &self.failed {
            kept["failed"] = json!(failed);
        }
        if let Some(place) = &self.elsewhere {
            kept["elsewhere"] = place.clone();
        }
        let store = &cx.engine.inner.store;
        let of = (!step.is_empty()).then_some(step);
        let _ = match (cx.run_id.is_empty(), says.is_empty()) {
            (true, true) => store.save_stop(&cx.live.id, kept).await,
            (true, false) => store.save_stop_saying(&cx.live.id, kept, says).await,
            (false, true) => store.save_hooks(&cx.run_id, of, kept).await,
            (false, false) => store.save_hooks_saying(&cx.run_id, of, kept, says).await,
        };
    }

    async fn keep(&mut self, cx: &RunCx, step: &str) {
        self.keep_saying(cx, step, None).await;
    }

    /// Keeps how far it has come, and says with it, in the same write,
    /// that the entry just done could not be: kept and then said, or said
    /// and then kept, a conductor stopped between the two did the entry
    /// again and it was said twice. Where nothing is kept it is said by
    /// itself.
    async fn keep_saying(&mut self, cx: &RunCx, step: &str, unsaid: Option<Say>) {
        // Whatever entry kept where another journal had got to is done.
        self.elsewhere = None;
        match cx.kept() {
            true => {
                self.at = Some(cx.engine.own().last_seq());
                self.write(cx, step, unsaid.into_iter().collect()).await;
            }
            false => {
                if let Some(could_not) = unsaid {
                    could_not.tell(cx.engine.own()).await;
                }
            }
        }
    }

    /// Keeps that what a run does as it ends has begun, before the first
    /// of it is: a run taken up after that goes on from the entry it had
    /// come to, which may have been under way; one taken up before it has
    /// nothing kept, and does all of it for the first time. A step needs no
    /// such mark: its own record says it began, and how its action ended
    /// is kept before anything is done about that.
    pub(crate) async fn begin(&mut self, cx: &RunCx) {
        if !self.taken_up {
            self.keep(cx, "").await;
        }
    }

    /// What is done as the step starts, from the entry it had come to.
    ///
    /// Gives whether the step's own action may have begun before now: it
    /// begins when the last of these is done, so a step taken up with one
    /// of them still to do never came to its action, and is not being done
    /// again. One taken up with all of them done may have, in the moment
    /// after the last was kept or long after, and nothing tells the two
    /// apart: its action is told it is being done again, and one that
    /// cannot be done twice is refused. A step that begins, or of which
    /// nothing was read, is answered as it always was.
    pub(crate) async fn starting(
        &mut self,
        cx: &RunCx,
        step: &str,
        hooks: &[Hook],
        scope: &Fields,
        cancel: Cancel,
    ) -> bool {
        let acting = !self.taken_up || self.start >= hooks.len();
        for (index, hook) in hooks.iter().enumerate().skip(self.start) {
            let since = std::mem::take(&mut self.taken_up).then_some(self.at);
            let list = Some(("on_start", index));
            let (_, unsaid) = run_hook(
                cx,
                step,
                hook,
                scope,
                Some(cancel.clone()),
                false,
                since,
                Some(index),
                list,
                self.attempt,
            )
            .await;
            self.start = index + 1;
            self.keep_saying(cx, step, unsaid).await;
        }
        // From here its action is under way, and what is done at its
        // ending is begun when that has ended.
        self.taken_up = false;
        acting
    }

    /// Keeps how the step's action ended, before anything is done about it.
    pub(crate) async fn act(&mut self, cx: &RunCx, step: &str, how: Value) {
        if self.acted.is_none() {
            (self.acted, self.end, self.decision) = (Some(how), 0, None);
            self.keep(cx, step).await;
        }
    }

    /// What is done at an ending: `hooks` are its entries from `first` on,
    /// and the whole of the list called `list`. `last` says nothing comes
    /// after them but the record of the ending itself, which says as much
    /// as keeping the last of them would.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn ending(
        &mut self,
        cx: &RunCx,
        step: &str,
        first: usize,
        list: &'static str,
        hooks: &[Hook],
        scope: &Fields,
        may_decide: bool,
        last: bool,
    ) -> Option<Decision> {
        for (index, hook) in hooks.iter().enumerate() {
            let entry = first + index;
            if entry < self.end {
                continue;
            }
            let since = std::mem::take(&mut self.taken_up).then_some(self.at);
            let (counted, of) = (Some(AT_THE_END + entry), Some((list, index)));
            let (decided, unsaid) =
                run_hook(cx, step, hook, scope, None, may_decide, since, counted, of, self.attempt).await;
            self.decision = decided.or(self.decision);
            self.end = entry + 1;
            // The last is kept too where it could not be done: that is
            // said with it, and the record of the ending says nothing of it.
            if unsaid.is_some() || !(last && index + 1 == hooks.len()) {
                self.keep_saying(cx, step, unsaid).await;
            }
        }
        self.decision.filter(|_| may_decide)
    }
}

impl Around {
    /// Says that this is what an instance does as it stops, and why it
    /// is being stopped: kept with the note that its stop has begun,
    /// where this is the beginning of it.
    pub(crate) fn stopping(&mut self, why: &str) {
        if !self.taken_up {
            self.why = Some(why.to_string());
        }
    }

    /// Does a list that is done by itself and around no step: what a text
    /// says is done as it is taken (`list` is `on_update`), in the run
    /// that carries its instance over, or as an instance stops
    /// (`on_stop`), in no run. From the entry it had come to, each kept
    /// as it is done: taken up after a restart, none is done twice, and
    /// the one it was in the middle of is done again looking first for an
    /// event it had said. That it has begun is kept before the first of
    /// it that is done, as a run keeps that its ending has
    /// ([`Around::begin`]); a list none of which is done keeps nothing.
    ///
    /// `scope` is read afresh for each entry, so that one sees what the
    /// one before set: taken up among them it could see nothing else.
    /// Where `strict`, the first entry that cannot be done ends it, and
    /// that is why; otherwise one that cannot be done is said and the
    /// rest are done.
    pub(crate) async fn alone(
        &mut self,
        cx: &RunCx,
        list: &'static str,
        hooks: &[Hook],
        scope: impl Fn() -> Fields,
        cancel: Cancel,
        strict: bool,
    ) -> Result<(), String> {
        // Taken up after it had ended at an entry that could not be done:
        // it has failed, as it said then.
        if let Some(why) = self.failed.take() {
            return Err(why);
        }
        let mut begun = self.taken_up;
        for (index, hook) in hooks.iter().enumerate().skip(self.end) {
            let scope = scope();
            // One whose `when` does not hold is not done, and nothing is
            // kept for it: looked at again when the list is taken up, it
            // still does not hold, since nothing after it was done.
            if let Some(when) = &hook.when
                && matches!(when.test(&scope), Ok(false))
            {
                self.end = index + 1;
                continue;
            }
            if !begun {
                self.keep(cx, "").await;
                begun = true;
            }
            let since = std::mem::take(&mut self.taken_up).then_some(self.at);
            let (counted, of) = (Some(AT_THE_END + index), Some((list, index)));
            let entered =
                enter(cx, "", hook, &scope, Some(cancel.clone()), since, counted, of, 0, None).await;
            let wrong = |what: &str| format!("`{list}` entry {}, `{}`: {what}", index + 1, hook.action);
            match entered {
                Ok(_) => {}
                // As it is written, it never began: that is the list's
                // failing where it is strict, and nothing was done.
                Err(Unentered::Written(error)) if strict => return Err(wrong(&error)),
                Err(Unentered::Failed(error)) if strict => {
                    // That it could not be done is said in the write that
                    // keeps why the list ended here.
                    let why = wrong(&error);
                    (self.end, self.failed) = (index + 1, Some(why.clone()));
                    self.keep_saying(cx, "", Some(could_not(cx, "", hook, error))).await;
                    return Err(why);
                }
                Err(Unentered::Written(error) | Unentered::Failed(error)) => {
                    self.end = index + 1;
                    self.keep_saying(cx, "", Some(could_not(cx, "", hook, error))).await;
                    continue;
                }
            }
            self.end = index + 1;
            self.keep(cx, "").await;
        }
        self.taken_up = false;
        Ok(())
    }
}

pub(crate) use texts::ran::timed;

/// Watches a running step, timed as [`timed`] says from when it began
/// (`started`, on the conductor's clock). Returns only if a rule with
/// `fail: true` fires, with the reason; otherwise it runs until the step
/// is done and is dropped.
pub(crate) async fn watch(
    cx: &RunCx,
    step: &Step,
    (timed, started): (&[Option<u64>], i64),
    scope: &Fields,
    mut hits: UnboundedReceiver<usize>,
    cancel: Cancel,
) -> String {
    if step.watch.is_empty() {
        return std::future::pending().await;
    }
    let clock = cx.engine.own().clock().clone();
    // How often each rule has fired in this try of the step.
    let mut fired = vec![0; step.watch.len()];
    // When each timed rule is next due, in milliseconds since the step started.
    let mut due: Vec<Option<u64>> = timed.to_vec();
    loop {
        let elapsed = (clock.now_ms() - started).max(0) as u64;
        let next = due.iter().enumerate().filter_map(|(index, at)| at.map(|at| (at, index))).min();
        let firing = match next {
            Some((at, index)) if at <= elapsed => {
                due[index] = match (&step.watch[index].cond, timed[index]) {
                    (WatchCond::Every(_), Some(millis)) => Some(at + millis),
                    _ => None,
                };
                index
            }
            Some((at, _)) => tokio::select! {
                () = clock.sleep(Duration::from_millis(at - elapsed)) => continue,
                hit = hits.recv() => match hit {
                    Some(index) => index,
                    None => continue,
                },
            },
            None => match hits.recv().await {
                Some(index) => index,
                None => return std::future::pending().await,
            },
        };
        let rule = &step.watch[firing];
        fired[firing] += 1;
        let running = view(StepStatus::Running, &Value::Null, None, 0, clock.now_ms() - started);
        let hook_scope = with_self(scope, &step.id, running);
        let entered = enter(
            cx,
            &step.id,
            &rule.hook,
            &hook_scope,
            Some(cancel.clone()),
            None,
            None,
            None,
            0,
            Some((firing, fired[firing])),
        );
        // Nothing is kept of a rule that this could be said with.
        if let Err(Unentered::Written(error) | Unentered::Failed(error)) = entered.await {
            could_not(cx, &step.id, &rule.hook, error).tell(cx.engine.own()).await;
        }
        if rule.fail {
            let when = duration::format(timed[firing].unwrap_or_default());
            let why = match &rule.cond {
                WatchCond::After(_) => format!("still running after {when}"),
                WatchCond::Every(_) => format!("its {when} check"),
                WatchCond::Log(pattern) => format!("its output matched `{pattern}`"),
            };
            return format!("stopped by a watch rule: {why}");
        }
    }
}
