//! The actions built into the language.
//!
//! These act on the run itself (wait, publish, change state, end), so they
//! live here rather than in the registry. Each is safe to repeat, and `wait`
//! checkpoints what it is waiting for, so a five-hour wait survives a
//! restart and still ends when it was going to. `emit` and `stop`, done
//! again after a restart, look first for what they did the first time.

use std::time::Duration;

use blueprint::validate::is_topic;
use blueprint::{Cond, Flow, RESERVED_TOPICS, Step, duration, template, within};
use journal::EventQuery;
use kernel::Fields;
use runs::Done as Kept;
use serde_json::{Value, json};

use crate::action::ActionError;
use crate::run::RunCx;
use crate::step::{Call, Done, End};

pub(crate) async fn run(call: Call<'_>, action: &str, args: Fields) -> Result<Done, ActionError> {
    let cx = call.cx;
    let text = |key: &str| args.get(key).and_then(Value::as_str).map(str::to_string);
    match action {
        "noop" => Ok(Done::Out(Value::Null)),
        "log" => {
            call.sink.line('o', &text("message").unwrap_or_default());
            Ok(Done::Out(Value::Null))
        }
        "fail" => Err(ActionError::new(text("message").unwrap_or_else(|| "failed on purpose".into()))),
        "end" => {
            let failed = match text("status").as_deref() {
                None | Some("ok") => false,
                Some("failed") => true,
                Some(other) => {
                    return Err(ActionError::new(format!(
                        "`status` must be `ok` or `failed`, got `{other}`"
                    )));
                }
            };
            Ok(Done::End(End { failed, reason: text("reason") }))
        }
        "emit" => match said_before(&call, &args).await {
            Some(seq) => Ok(Done::Out(json!({ "seq": seq }))),
            None => Ok(Done::Out(json!({ "seq": say(&call, args).await? }))),
        },
        "set" => {
            let declared =
                |args: &Fields| match args.keys().find(|key| !cx.blueprint.state.contains_key(*key)) {
                    Some(unknown) => Err(ActionError::new(format!(
                        "there is no state key `{unknown}`; declare it under `state` with an initial value"
                    ))),
                    None => Ok(()),
                };
            // Said before its turn comes, of what was worked out as the
            // step began: a key that is not there is not waited for.
            declared(&args)?;
            // What it sets is worked out again when its turn comes, from
            // the state as it then is. What it read as the step began may
            // have been set since by a handler or a step beside it, and
            // `${ state.n + 1 }` must add to what is there.
            // With what it sets is written that this step has acted, as
            // one thing: taken up after a restart it is not acted again.
            let (run, step) = (cx.run_id.clone(), call.step.to_string());
            let (entry, at) = (call.entry, call.element.as_ref().map(|element| element.at));
            let around = call.around;
            let done = move |gave: &Fields| {
                let gave = Value::Object(gave.clone());
                Some(match (entry, at) {
                    (Some(entry), _) => crate::hooks::done_entry(cx, step, entry),
                    (None, Some(position)) => Kept::Element { run, step, position, gave },
                    // Done by a rule that watches a step, or as an
                    // instance stops: no record says how far those have
                    // come, and the step's own is not theirs to mark.
                    (None, None) if step.is_empty() || around => return None,
                    (None, None) => Kept::Step { run, step, gave },
                })
            };
            let work_out = |state: &Fields| {
                let mut scope = call.scope.clone();
                scope.insert("state".into(), Value::Object(state.clone()));
                let args = match template::render(call.with, &scope)
                    .map_err(|err| format!("`with` could not be filled in: {err}"))?
                {
                    Value::Object(args) => args,
                    Value::Null => Fields::new(),
                    other => {
                        return Err(ActionError::new(format!("`with` must produce a mapping, got {other}")));
                    }
                };
                declared(&args)?;
                Ok(args)
            };
            let set = cx.engine.set_state(&cx.live, work_out, done).await?;
            Ok(Done::Out(Value::Object(set)))
        }
        "keep" | "drop" => kept(&call, action, &args).await,
        "measure" => {
            let name = text("name").unwrap_or_default();
            if let Some(why) = blueprint::labels::measurement(&name) {
                return Err(ActionError::new(why));
            }
            let own = match args.get("labels") {
                None | Some(Value::Null) => blueprint::Labels::new(),
                Some(Value::Object(own)) => blueprint::labels::given(own).map_err(ActionError::new)?,
                Some(_) => {
                    return Err(ActionError::new("`labels` is a mapping from a label's name to its value"));
                }
            };
            // Whose it is and what it runs under come first, as they do for
            // everything else measured here: its own labels add to those.
            let project = cx.live.meta().project.unwrap_or_default();
            let mut all = vec![("blueprint", cx.blueprint.name.as_str()), ("project", project.as_str())];
            for (label, value) in cx.labels.iter().chain(&own) {
                if !all.iter().any(|(taken, _)| taken == label) {
                    all.push((label.as_str(), value.as_str()));
                }
            }
            let value = args.get("value").and_then(Value::as_f64).unwrap_or(0.0);
            let ok = args.get("ok").and_then(Value::as_bool).unwrap_or(true);
            cx.engine.own().measure(&name, &all, value, ok);
            Ok(Done::Out(json!({ "name": name, "value": value, "ok": ok })))
        }
        "stop" => {
            let id = match text("instance") {
                None => cx.live.id.clone(),
                Some(name) => {
                    let missing_ok = args.get("missing_ok").and_then(Value::as_bool).unwrap_or(false);
                    // Done again after a restart, having stopped it the
                    // first time: it is not alive to be found by its
                    // name, and that is what was asked for. Only one
                    // that ended since this step began counts: one of
                    // that name that was over before is not what this
                    // step stopped. Looked for before what is alive: one
                    // of that name started since (by whatever asked for
                    // it as it was being stopped, and waited) is not
                    // what this step was asked to stop, and done again it
                    // stopped that one too. And before what is said of
                    // nothing being there: a step that says that would
                    // be no fault answers, done again, as it did.
                    let stopped = match call.again {
                        true => stopped_since(cx, call.step, &name).await?,
                        false => None,
                    };
                    if let Some(stopped) = stopped {
                        return Ok(Done::Out(json!({ "instance": stopped, "stopped": true })));
                    }
                    let alive =
                        cx.engine.inner.store.live_instance_by_key(Some(&cx.live.id), &name, None).await?;
                    match alive {
                        Some(child) => child.id,
                        // Nothing alive under this one is called that, and
                        // whoever asked said that would be no fault.
                        None if missing_ok => {
                            return Ok(Done::Out(json!({ "instance": null, "stopped": false })));
                        }
                        None => name,
                    }
                }
            };
            if id == cx.live.id {
                // Stopping ourselves cancels this very step, so it cannot be
                // awaited from inside it: it is begun beside it, and is
                // under way from here, so that the run, ending first, leaves
                // the instance to it ([`crate::meets::Ahead`]).
                let ahead = crate::meets::ahead(&cx.engine, &id);
                let engine = cx.engine.clone();
                let target = id.clone();
                tokio::spawn(async move {
                    let _ = engine.stop(&target).await;
                    drop(ahead);
                });
            } else {
                cx.engine.stop(&id).await?;
            }
            Ok(Done::Out(json!({ "instance": id, "stopped": true })))
        }
        "wait" => wait(call, args).await,
        other => Err(ActionError::new(format!("`{other}` is not built in"))),
    }
}

async fn wait(call: Call<'_>, args: Fields) -> Result<Done, ActionError> {
    let cx = call.cx;
    let journal = cx.engine.own();
    let clock = journal.clock().clone();
    let millis = |key: &str| -> Result<Option<u64>, ActionError> {
        match args.get(key).and_then(Value::as_str) {
            Some(text) => {
                duration::parse(text).map(Some).map_err(|err| ActionError::new(format!("`{key}`: {err}")))
            }
            None => Ok(None),
        }
    };
    let (pause, timeout) = (millis("for")?, millis("timeout")?);
    let pattern = args.get("event").and_then(Value::as_str);
    let everywhere = match args.get("scope").and_then(Value::as_str) {
        None | Some("project") => false,
        Some("fleet") => true,
        Some(other) => {
            return Err(ActionError::new(format!(
                "`scope` says whose events a wait hears: `project`, its own project's and the factory's own, \
                 which is the usual; or `fleet`, every project's. `{other}` is neither"
            )));
        }
    };
    let saved = call.saved.clone().unwrap_or(Value::Null);
    // Only a step's own action has a place to keep in the step's record:
    // what is found there when the step is taken up is taken for the
    // action's. A wait done around a step, or by a rule that watches it,
    // keeps nothing, and begins again if it is cut short. Told by what
    // the call is, never by which try it is at: what is done around a
    // step is at the step's own try.
    let own = !call.step.is_empty() && !call.around && call.list.is_none();
    let checkpoint = |value: Value| async move {
        if own && cx.kept() {
            let _ = cx.engine.save_checkpoint(&cx.run_id, call.step, value).await;
        }
    };
    let now = clock.now_ms();

    if let Some(keys) = args.get("ended") {
        if pause.is_some() || pattern.is_some() {
            return Err(ActionError::new("`ended` is waited for alone, with neither `for` nor `event`"));
        }
        let Value::Array(keys) = keys else {
            return Err(ActionError::new(
                "`ended` is a list of what this instance started, each by the name it was started under (`as`)",
            ));
        };
        let keys: Vec<String> = keys.iter().map(blueprint::expr::display).collect();
        let every = &cx.engine.inner.factory;
        let cursor = journal::Places::from_kept(&saved["cursor"]).unwrap_or_else(|| every.now());
        let deadline = saved["deadline"].as_i64().or(timeout.map(|timeout| now + timeout as i64));
        checkpoint(json!({ "cursor": cursor.kept(), "deadline": deadline })).await;
        announce(&call, json!({ "ended": keys, "until": deadline })).await;
        // Its place among the journals is taken before anything is looked
        // at, so that what ends between the look and the listening is heard.
        let mut follow = every.follow_every(cursor, "instance.finished");
        let store = &cx.engine.inner.store;
        loop {
            let (mut came, mut gave) = (serde_json::Map::new(), Vec::new());
            let mut alive = false;
            for key in &keys {
                if store.live_instance_by_key(Some(&cx.live.id), key, None).await?.is_some() {
                    alive = true;
                    break;
                }
                // Nothing alive under the name has ended: how the last of it
                // ended, or nothing where none was ever started.
                let last = store.last_instance_by_key(&cx.live.id, key).await?;
                // And what its steps gave out, where they ran: what it
                // came to, which nothing else hands back of a spawn.
                let run = match &last {
                    Some(one) => store.last_main_run(&one.id).await?,
                    None => None,
                };
                gave.push(run.map_or(Value::Null, |run| run.outputs));
                came.insert(key.clone(), last.map_or(Value::Null, |one| json!(one.status)));
            }
            if !alive {
                return Ok(Done::Out(json!({ "timed_out": false, "ended": came, "gave": gave })));
            }
            let remaining =
                deadline.map(|deadline| Duration::from_millis((deadline - clock.now_ms()).max(0) as u64));
            let clock = clock.clone();
            let expired = async move {
                match remaining {
                    Some(remaining) => clock.sleep(remaining).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                event = follow.next() => { event?; }
                () = expired => return Ok(Done::Out(json!({ "timed_out": true, "ended": null }))),
                () = call.cancel.cancelled() => return Err(ActionError::cancelled()),
            }
            // Called off and handed an event together: called off, it takes in nothing more.
            if call.cancel.is_cancelled() {
                return Err(ActionError::cancelled());
            }
        }
    }

    match (pause, pattern) {
        (Some(_), Some(_)) => Err(ActionError::new("use either `for` or `event`, not both")),
        (None, None) => Err(ActionError::new(
            "`wait` needs `for: <duration>`, `event: <topic>` or `ended: [<name>, ...]`",
        )),
        (Some(pause), None) => {
            let wake_at = saved["wake_at"].as_i64().unwrap_or(now + pause as i64);
            checkpoint(json!({ "wake_at": wake_at })).await;
            announce(&call, json!({ "until": wake_at })).await;
            // Until the time it kept, and not so long from when the clock
            // was read: the clock may have moved while it said so.
            tokio::select! {
                () = clock.sleep_until(wake_at) => Ok(Done::Out(json!({ "timed_out": false, "waited_ms": pause }))),
                () = call.cancel.cancelled() => Err(ActionError::cancelled()),
            }
        }
        (None, Some(pattern)) => {
            // Events that arrive while the conductor is down still count:
            // the wait resumes from the position it started at, which is
            // a place in each journal. A number alone is what a program
            // from before kept.
            // A place in each journal, and what is said in any of them,
            // are asked of the factory's own, which keeps the others.
            let every = &cx.engine.inner.factory;
            let cursor = journal::Places::from_kept(&saved["cursor"]).unwrap_or_else(|| every.now());
            let deadline = saved["deadline"].as_i64().or(timeout.map(|timeout| now + timeout as i64));
            // What it is to say once it is listening: something asked, whose
            // answer is what is waited for. Said after the wait has its
            // place in the journal, so the answer cannot come too soon to
            // be heard.
            let asks = match args.get("say") {
                None | Some(Value::Null) => None,
                Some(Value::Object(asks)) => Some(asks.clone()),
                Some(_) => {
                    return Err(ActionError::new(
                        "`say` is an event, said as `emit` says one: a `topic`, and what it carries",
                    ));
                }
            };
            // A wait taken up again may have said it already, or been
            // stopped just before. The journal is what knows: whether this
            // step of this run said that topic after the wait took its
            // place. So it is asked once, however the wait was interrupted.
            let taken_up = !saved["cursor"].is_null();
            checkpoint(json!({ "cursor": cursor.kept(), "deadline": deadline })).await;
            announce(&call, json!({ "event": pattern, "until": deadline })).await;
            if let Some(asks) = asks {
                let topic = asks.get("topic").and_then(Value::as_str).unwrap_or_default().to_string();
                let said = taken_up && {
                    let own = journal::EventQuery {
                        topics: topic,
                        run: Some(cx.run_id.clone()),
                        limit: 10_000,
                        ..Default::default()
                    };
                    let by_this = |event: &kernel::Event| event.source.step.as_deref() == Some(call.step);
                    // In whichever journal keeps what it said.
                    every.read_within(&cursor, &every.now(), own).await?.iter().any(by_this)
                };
                if !said {
                    say(&call, asks).await?;
                }
            }
            let filter = match args.get("where").and_then(Value::as_str) {
                Some(text) => Some(
                    Cond::parse(text).map_err(|err| ActionError::new(format!("`where`: {}", err.message)))?,
                ),
                None => None,
            };
            // What every journal says, as a handler that hears the whole
            // factory would be handed it: what is not within its reach
            // is passed over below.
            let mut follow = every.follow_every(cursor, pattern);
            loop {
                let remaining =
                    deadline.map(|deadline| Duration::from_millis((deadline - clock.now_ms()).max(0) as u64));
                let clock = clock.clone();
                let expired = async move {
                    match remaining {
                        Some(remaining) => clock.sleep(remaining).await,
                        None => std::future::pending().await,
                    }
                };
                let event = tokio::select! {
                    event = follow.next() => event?,
                    () = expired => return Ok(Done::Out(json!({ "timed_out": true, "event": null }))),
                    () = call.cancel.cancelled() => return Err(ActionError::cancelled()),
                };
                // Called off and handed an event together, which of the
                // two is seen first is chance: called off, it takes in
                // nothing more ([`crate::meets`]).
                if call.cancel.is_cancelled() {
                    return Err(ActionError::cancelled());
                }
                if !event.is_durable() || !cx.engine.hears(&cx.principal, &cx.live.id, &event).await {
                    continue;
                }
                // As a handler that says no scope: its own project's and
                // what is nobody's, unless it says the whole factory.
                let heard_in = follow.heard_in().unwrap_or(journal::FACTORY).to_string();
                if !everywhere && !cx.engine.within_reach(&cx.live, &heard_in, &event).await {
                    continue;
                }
                let event = serde_json::to_value(&*event).unwrap_or(Value::Null);
                if let Some(filter) = &filter {
                    let mut scope = call.scope.clone();
                    scope.insert("event".into(), event.clone());
                    if !filter.test(&scope).map_err(|err| ActionError::new(format!("`where`: {err}")))? {
                        continue;
                    }
                }
                return Ok(Done::Out(json!({ "timed_out": false, "event": event })));
            }
        }
    }
}

/// Says an event for the step: its `topic`, and everything else given as
/// what it carries. Where in the journal it was said.
async fn say(call: &Call<'_>, mut said: Fields) -> Result<u64, ActionError> {
    let cx = call.cx;
    let topic = said.get("topic").and_then(Value::as_str).unwrap_or_default().to_string();
    // Checked again here because the topic may have come from a `${ }`.
    if !is_topic(&topic) {
        return Err(ActionError::new(format!(
            "`{topic}` is not a topic; use lowercase words joined by dots"
        )));
    }
    if RESERVED_TOPICS.contains(&topic.split('.').next().unwrap_or_default()) {
        return Err(ActionError::new(format!(
            "`{topic}` is in a namespace only the system may publish under"
        )));
    }
    // Saying an event is what `emit` allows, from a step as from outside:
    // what hears it does what it does with the authority of whoever
    // applied that, and takes the event for one somebody was let say.
    if !cx.principal.can(&format!("emit:{topic}")) {
        return Err(ActionError::new(format!(
            "{} may not say `{topic}` (needs `emit:{topic}`, or `emit`)",
            cx.principal.name
        )));
    }
    said.remove("topic");
    let source = cx.source((!call.step.is_empty()).then_some(call.step));
    let said = Value::Object(said);
    // Kept where what it names is kept ([`Engine::kept_in`]).
    let kept_in = cx.engine.kept_in(&said);
    if (kept_in.name() != cx.engine.own().name() || call.rule.is_some()) && cx.kept() {
        keep_place(call, &kept_in).await?;
    }
    Ok(kept_in.emit(&topic, source, said).await?.seq)
}

/// What a step's record keeps of an event it says in another journal:
/// the last number that journal had given before.
const ELSEWHERE: &str = "said_after";

/// Before an event is said in another journal than the run's, where
/// that journal has got to is kept with what says it: in the step's
/// record for its own action (for each of a list, with the element),
/// and for an entry of what is done around a step with how far those
/// have come. This run's places mean nothing there, and a step done
/// again after a stop has to find what it said: it is what this step
/// said of the topic in that journal after the place kept, since what
/// one step says it says one thing after another. A rule that watches a
/// step keeps it in whichever journal it says something, its own too,
/// with which time it fired: nothing else of a rule is kept to find what
/// it said by ([`rule_said`]).
async fn keep_place(call: &Call<'_>, journal: &journal::Journal) -> kernel::Result<()> {
    let (cx, after) = (call.cx, journal.last_seq());
    if let Some(rule) = call.rule {
        let place = json!({ "in": journal.name(), "after": after });
        return crate::hooks::keep_watched(cx, call.step, rule, place).await;
    }
    match (call.list, call.around) {
        (Some((list, index)), _) => {
            let place = json!({ "in": journal.name(), "after": after, "list": list, "index": index });
            crate::hooks::keep_elsewhere(cx, call.step, place).await
        }
        // Kept the first time it came here: what it said, it said after that.
        (None, false) if !call.step.is_empty() && call.saved.is_none() => {
            let place = json!({ ELSEWHERE: after });
            let saved = match &call.element {
                Some(element) => crate::action::Element::around(&element.items, Some((element.at, place))),
                None => place,
            };
            cx.engine.save_checkpoint(&cx.run_id, call.step, saved).await
        }
        _ => Ok(()),
    }
}

/// How far back what a step said is looked for.
const FAR: usize = 10_000;

/// The step of `flow` called `name` in full, looked for inside tasks too.
fn step_of<'a>(flow: &'a Flow, before: &str, name: &str) -> Option<&'a Step> {
    flow.steps.iter().find_map(|step| {
        let full = within::name(before, &step.id);
        match full == name {
            true => Some(step),
            false => step_of(&step.inner, &within::task(&full), name),
        }
    })
}

/// What a step that says an event had said already, when it was under way
/// as the conductor stopped and is being done again. An event is one
/// thing said, and whatever hears it would be set off twice: so the step
/// looks for it first, among what this step of this run said after it
/// began (done for each of a list, after this element began), and gives
/// that event's place as it did then.
///
/// What is done as the step starts may say events too, the same one
/// among them, and those are said from the same step: so where there is
/// any of that, the step's own are those said after the last of it was
/// done, which is kept ([`crate::hooks::started`]). An element of a list
/// began after all of it.
///
/// An event that names another project is kept in that project's
/// journal, where none of those places mean anything: what the step
/// kept before it said anything there says where to look
/// ([`keep_place`]).
///
/// Where it cannot tell, it says the event again, as it always did: its
/// beginning is no longer in the journal, or was never kept.
async fn said_before(call: &Call<'_>, said: &Fields) -> Option<u64> {
    let cx = call.cx;
    let topic = said.get("topic").and_then(Value::as_str)?;
    let kept_in = cx.engine.kept_in(&Value::Object(said.clone()));
    if kept_in.name() != cx.engine.own().name() {
        let after = call.saved.as_ref()?.get(ELSEWHERE)?.as_u64()?;
        return said_in(cx, &kept_in, call.step, topic, after).await;
    }
    if !call.again || call.step.is_empty() {
        return None;
    }
    let step = step_of(cx.flow(), "", call.step)?;
    let began = match &call.element {
        Some(element) => {
            cx.engine.inner.store.element_began(&cx.run_id, call.step, element.at).await.ok()??
        }
        None if step.on_start.is_empty() => began(cx, call.step).await?,
        None => crate::hooks::started(cx, call.step, step.on_start.len()).await?,
    };
    said_since(cx, call.step, topic, began).await
}

/// The last instance started under this one by `name` that is over,
/// where it ended since `step` of this run began: what the step stopped
/// before the conductor was. By where its ending was said, after the
/// step began or before; and where it was written as over and never said
/// to be, by when it was written, to the millisecond. Where the step is
/// not known to have begun, nothing counts.
async fn stopped_since(cx: &RunCx, step: &str, name: &str) -> kernel::Result<Option<String>> {
    let Some((began, at)) = first_began(cx, step).await else { return Ok(None) };
    let Some(last) = cx.engine.inner.store.last_over_by_key(&cx.live.id, name).await? else {
        return Ok(None);
    };
    let over = EventQuery {
        instance: Some(last.id.clone()),
        topics: "instance.finished".into(),
        newest_first: true,
        limit: 1,
        ..Default::default()
    };
    let since = match cx.engine.own().read(over).await?.first() {
        Some(said) => said.seq > began,
        None => last.updated >= at,
    };
    Ok(since.then_some(last.id))
}

/// Where in the journal a step of this run began: where it began, and not
/// where it was taken up again, which is said too.
pub(crate) async fn began(cx: &RunCx, step: &str) -> Option<u64> {
    Some(first_began(cx, step).await?.0)
}

/// Where a step of this run began, and when: as its row says, which is
/// written where its beginning is said, kept or only passed on; for a row
/// a program from before wrote, as the journal kept its saying so.
async fn first_began(cx: &RunCx, step: &str) -> Option<(u64, i64)> {
    if let Ok(Some(began)) = cx.engine.inner.store.step_began(&cx.run_id, step).await {
        return Some(began);
    }
    // Everything the run said is after where its journal had got to as
    // the run was written, which is where it is looked for from.
    let after = cx.engine.inner.store.run_place(&cx.run_id).await.ok()?;
    let started = EventQuery {
        run: Some(cx.run_id.clone()),
        topics: "step.started".into(),
        after,
        newest_first: true,
        limit: FAR,
        ..Default::default()
    };
    let began =
        |event: &kernel::Event| event.source.step.as_deref() == Some(step) && event.data["resumed"] != true;
    let first = cx.engine.own().read(started).await.ok()?.into_iter().find(began)?;
    Some((first.seq, first.ts))
}

/// The first event of `topic` that `step` of this run said after `after`;
/// with no step, that the run itself said, as what it does when it ends;
/// and with no run, that the instance said in none, as it stops.
pub(crate) async fn said_since(cx: &RunCx, step: &str, topic: &str, after: u64) -> Option<u64> {
    said_in(cx, cx.engine.own(), step, topic, after).await
}

/// The same, of what was said in `journal`.
async fn said_in(cx: &RunCx, journal: &journal::Journal, step: &str, topic: &str, after: u64) -> Option<u64> {
    let of_run = (!cx.run_id.is_empty()).then(|| cx.run_id.clone());
    let since = EventQuery {
        instance: of_run.is_none().then(|| cx.live.id.clone()),
        run: of_run,
        topics: topic.into(),
        after,
        limit: FAR,
        ..Default::default()
    };
    let own = |event: &kernel::Event| {
        event.source.step.as_deref().unwrap_or_default() == step
            && event.source.run.as_deref().unwrap_or_default() == cx.run_id
    };
    journal.read(since).await.ok()?.into_iter().find(own).map(|event| event.seq)
}

/// Whether an entry of what is done around `step`, taken up where it
/// may have been under way, had said the event it says: in the run's
/// own journal after `after`, where how far the entries had come was
/// last kept; in another, after where that journal had got to as this
/// entry was about to say it there ([`keep_place`]).
pub(crate) async fn entry_said(
    cx: &RunCx,
    step: &str,
    said: &Fields,
    after: Option<u64>,
    entry: Option<(&'static str, usize)>,
) -> bool {
    let Some(topic) = said.get("topic").and_then(Value::as_str) else { return false };
    let kept_in = cx.engine.kept_in(&Value::Object(said.clone()));
    let after = match kept_in.name() == cx.engine.own().name() {
        true => after,
        false => crate::hooks::kept_elsewhere(cx, step)
            .await
            .filter(|place| {
                let its = entry.is_some_and(|(list, index)| place["list"] == list && place["index"] == index);
                its && place["in"] == kept_in.name()
            })
            .and_then(|place| place["after"].as_u64()),
    };
    match after {
        Some(after) => said_in(cx, &kept_in, step, topic, after).await.is_some(),
        None => false,
    }
}

/// Whether a rule that watches `step`, firing for the time it fired
/// before the conductor stopped and the step was done again, had said the
/// event it says then: after where the journal it is kept in had got to
/// as that firing was about to say it ([`keep_place`]). A firing it
/// never came to before keeps nothing, and says it. What the step's own
/// action said of the same topic after that place is taken for the
/// rule's, as what an entry looks for is: there is nothing to tell the
/// two apart by.
pub(crate) async fn rule_said(cx: &RunCx, step: &str, said: &Fields, rule: (usize, usize)) -> bool {
    let Some(topic) = said.get("topic").and_then(Value::as_str) else { return false };
    let kept_in = cx.engine.kept_in(&Value::Object(said.clone()));
    let place =
        crate::hooks::kept_watched(cx, step, rule).await.filter(|place| place["in"] == kept_in.name());
    match place.and_then(|place| place["after"].as_u64()) {
        Some(after) => said_in(cx, &kept_in, step, topic, after).await.is_some(),
        None => false,
    }
}

/// Says that a step is now waiting, and for what. Published after the wait
/// has fixed its starting position, so anything published in response to
/// this is something the wait will see.
async fn announce(call: &Call<'_>, what: Value) {
    if call.step.is_empty() {
        return;
    }
    call.cx.say("step.waiting", call.cx.about(Some(call.step)), what).await;
}

/// `keep` and `drop`: one entry put into, or every such entry taken out
/// of, a list of the instance's state, under the state's own turn.
///
/// A text that keeps a list of what it has in hand (the things under
/// way, each with what it is called) changes it one entry at a time, and
/// its handlers run side by side: a whole list set from what one of them
/// read would take away what another had just kept. So the list is read
/// and written here in one turn, and what is asked for is the entry.
///
/// Each is safe to do twice as a `set` is: what it does to the list is
/// written with the record that its step has acted, as one thing, and a
/// step taken up after a restart is answered from that record. And each
/// means the same done twice by another step: an entry is the same as
/// another by a field of it that the step names (`by`), or by all of it
/// where none is named, so kept again it is there once, with what it now
/// says, and dropped again there is nothing to drop and no failure. An
/// entry that is no mapping is looked at whole, whatever `by` says: a
/// list that held the things themselves before it held mappings goes on
/// being read.
async fn kept(call: &Call<'_>, action: &str, args: &Fields) -> Result<Done, ActionError> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let cx = call.cx;
    let keeping = action == "keep";
    let (named, saying) = if keeping { ("in", "what") } else { ("from", "is") };
    let list = match args.get(named).and_then(Value::as_str) {
        Some(list) if !list.is_empty() => list.to_string(),
        _ => {
            return Err(ActionError::new(format!(
                "`{action}` needs `{named}`: the list of the state, by its name"
            )));
        }
    };
    if !cx.blueprint.state.contains_key(&list) {
        return Err(ActionError::new(format!(
            "there is no state key `{list}`; declare it under `state` as a list, `{list}: []`"
        )));
    }
    let by = args.get("by").and_then(Value::as_str).filter(|by| !by.is_empty());
    let said = match args.get(saying) {
        Some(said) if !said.is_null() => said.clone(),
        _ => return Err(ActionError::new(format!("`{action}` needs `{saying}`: it says nothing"))),
    };
    // What an entry is the same by: for one being kept, its own field.
    let same_as = match (keeping, by, &said) {
        (true, Some(by), Value::Object(what)) => match what.get(by) {
            Some(field) if !field.is_null() => field.clone(),
            _ => {
                return Err(ActionError::new(format!(
                    "`what` has no `{by}`, which is what an entry of `{list}` is the same by"
                )));
            }
        },
        (true, Some(by), other) => {
            return Err(ActionError::new(format!(
                "`by: {by}` is said of entries that are mappings, and `what` is {other}"
            )));
        }
        _ => said.clone(),
    };
    let same = |one: &Value| match (by, one) {
        (Some(by), Value::Object(one)) => one.get(by) == Some(&same_as),
        _ => *one == same_as,
    };
    // How many were taken out, and how many the list then holds.
    let (gone, now) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let gave = || {
        let (gone, now) = (gone.load(Ordering::SeqCst), now.load(Ordering::SeqCst));
        if keeping {
            json!({ "kept": now, "was": gone > 0 })
        } else {
            json!({ "dropped": gone, "left": now })
        }
    };
    let work_out = |state: &Fields| {
        let mut entries = match state.get(&list) {
            Some(Value::Array(entries)) => entries.clone(),
            Some(Value::Null) | None => Vec::new(),
            Some(other) => {
                return Err(ActionError::new(format!(
                    "`{list}` holds {other}, which is no list: `{action}` works on a list"
                )));
            }
        };
        let before = entries.len();
        entries.retain(|one| !same(one));
        gone.store(before - entries.len(), Ordering::SeqCst);
        if keeping {
            entries.push(said.clone());
        }
        now.store(entries.len(), Ordering::SeqCst);
        let mut set = Fields::new();
        set.insert(list.clone(), Value::Array(entries));
        Ok(set)
    };
    // With what it does to the list is written that this step has acted,
    // as a `set` writes it: which record, and what the step gave.
    let (run, step) = (cx.run_id.clone(), call.step.to_string());
    let (entry, at) = (call.entry, call.element.as_ref().map(|element| element.at));
    let around = call.around;
    let done = |_: &Fields| {
        Some(match (entry, at) {
            (Some(entry), _) => crate::hooks::done_entry(cx, step, entry),
            (None, Some(position)) => Kept::Element { run, step, position, gave: gave() },
            // As a `set`: what a rule that watches a step does, or what
            // is done as an instance stops, marks no step as having acted.
            (None, None) if step.is_empty() || around => return None,
            (None, None) => Kept::Step { run, step, gave: gave() },
        })
    };
    cx.engine.set_state(&cx.live, work_out, done).await?;
    Ok(Done::Out(gave()))
}
