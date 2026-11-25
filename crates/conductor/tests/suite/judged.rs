//! What a play may differ by, by design, where the conductor was stopped.
//!
//! A play stopped at a place is held to coming to what it comes to with
//! nothing in its way ([`crate::anywhere::differs`]). A few scenarios mean
//! something else at a restart, on purpose: what keeps nothing is begun
//! again, what goes by the clock counts again from each start, a step
//! that cannot be done twice fails saying so. Each of those is said here,
//! once, as what is taken out of both sides or held in its place, with
//! everything else held as it always is; never as a looser comparison.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::anywhere::{NOT_TWICE, Place, differs};

/// What goes by the clock begins counting again when the conductor does:
/// how often it fired may be one more or one fewer, and nothing else.
/// A count of something said, in what a play came to.
pub fn count(of: &Value, topic: &str) -> u64 {
    of["said"][topic].as_u64().unwrap_or(0)
}

/// One that keeps nothing as it goes is over at a restart, saying so, and
/// whatever called it calls it again: another instance under the same
/// name. So where the conductor stopped while it was there, it is said to
/// have been stopped, and to have started and ended once more.
pub fn kept_nothing(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut came = came.clone();
    let more = count(&came, "instance.stopped").saturating_sub(count(usual, "instance.stopped"));
    for topic in ["instance.stopped", "instance.started", "instance.finished", "run.finished"] {
        if (1..=more).contains(&count(&came, topic).saturating_sub(count(usual, topic))) {
            came["said"][topic] = usual["said"][topic].clone();
        }
    }
    // Of two that were under one name, the one before was stopped at the
    // restart, having kept nothing: it is there, and is not held to what
    // it came to. Any other that was there before is.
    for (of, before) in came["earlier"].as_object_mut().into_iter().flatten() {
        let usual = usual["earlier"][of].as_array().map_or(0, Vec::len);
        if let Some(before) = before.as_array_mut().filter(|before| before.len() > usual) {
            before.retain(|one| one["instance"]["status"] != "stopped");
        }
    }
    if let Some(earlier) = came["earlier"].as_object_mut() {
        earlier.retain(|of, before| {
            before.as_array().is_some_and(|all| !all.is_empty()) || !usual["earlier"][of].is_null()
        });
    }
    // Of two that were under one name, the one that was stopped said so.
    let stopped = |said: &Value| said["status"] == "stopped";
    for (of, said) in came["ended"].as_object_mut().into_iter().flatten() {
        let usual = usual["ended"][of].as_array().cloned().unwrap_or_default();
        if let Some(said) = said.as_array_mut().filter(|_| !usual.iter().any(stopped)) {
            said.retain(|one| !stopped(one));
        }
    }
    differs(usual, &came, again)
}

/// A run of a handler that the text its instance now runs no longer has
/// cannot be carried on by a conductor that starts: nothing has its
/// steps. It is ended as a run that was cancelled, saying why, and what
/// it had still to do is not done. Where the conductor was stopped with
/// such a run under way that is what is held of it, with everything
/// else held as it always is.
pub fn left_behind(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    const WHY: &str = "its handler is not in the text its instance now runs";
    let (of, flow) = ("drops[drops]", "on.work");
    let mut came = came.clone();
    let mut found = Vec::new();
    let run = came["runs"][of][flow][0].clone();
    if run["error"].as_str().is_some_and(|error| error.contains(WHY)) {
        if run["status"] != "cancelled" {
            found.push(format!("the run its text left behind came to {}", run["status"]));
        }
        // Each of its steps is as it is with nothing in its way, having
        // ended before the conductor stopped; or was cancelled, having
        // been under way; or was never come to.
        let meant = &usual["runs"][of][flow][0]["steps"];
        for (step, usual) in meant.as_object().into_iter().flatten() {
            let came = &run["steps"][step];
            if !(came == usual || came.is_null() || came["status"] == "cancelled") {
                found.push(format!("its step `{step}` came to {came}"));
            }
        }
        let (was, is) = (&usual["asked"]["mark:test.mark"], &came["asked"]["mark:test.mark"]);
        if is.as_u64().unwrap_or(0) > was.as_u64().unwrap_or(0) {
            found.push(format!("its last step was asked {is} times"));
        }
        came["runs"][of][flow][0] = usual["runs"][of][flow][0].clone();
        came["asked"]["mark:test.mark"] = usual["asked"]["mark:test.mark"].clone();
    }
    found.extend(differs(usual, &came, again));
    found
}

pub fn by_the_clock(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut found = Vec::new();
    // Once more or once fewer for each time the conductor was started:
    // the timer, and the action a handler asks every so often to see
    // whether what it answers has changed (asked of no step).
    let starts = came["stops"].as_i64().unwrap_or(1).max(1);
    let by_clock = ["beat:test.mark", ":test.echo"];
    for key in by_clock {
        let (was, is) = (usual["asked"][key].as_i64().unwrap_or(0), came["asked"][key].as_i64().unwrap_or(0));
        if (was - is).abs() > starts || is == 0 {
            found.push(format!("`{key}` was asked {is} times, and {was} with nothing in its way"));
        }
    }
    // What goes by the clock is taken out of both, and the rest is held
    // as anything is: the burst that settled once, what was said, what
    // became of the instance and how every other run ended.
    let rest = |of: &Value| {
        let mut of = of.clone();
        for key in by_clock {
            if let Some(asked) = of["asked"].as_object_mut() {
                asked.remove(key);
            }
        }
        // Each firing of the timer is a run, and ends: as many of both
        // as there were, and every one of them well.
        let beats = of["runs"]["timed[timed]"]["on.beat"].as_array().cloned().unwrap_or_default();
        let ended = of["said"]["run.finished"].as_i64().unwrap_or(0);
        of["said"]["run.finished"] = json!(ended - beats.len() as i64);
        let well = beats.iter().all(|run| run["status"] == "ok");
        of["runs"]["timed[timed]"]["on.beat"] =
            json!(if well { "each ended well" } else { "not each ended well" });
        if let Some(of) = of.as_object_mut() {
            of.remove("stops");
        }
        of
    };
    found.extend(differs(&rest(usual), &rest(came), again));
    found
}

/// A step's rules count from when the step began again, and a step that
/// is done again takes its whole time again: the rule that fires every so
/// often fires as often again for each time the step was under way when
/// the conductor stopped. What a rule says it says once all the same: a
/// rule that fires after a while, or on a line the step prints, fires
/// again, finds the event it said the same time before and says nothing.
/// Everything else is held: how the step and its run ended, what became
/// of the instance, what else was said, and that the line it printed was
/// seen.
pub fn watched(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut found = Vec::new();
    let begun_again = again.get("work").copied().unwrap_or(0) as u64;
    let fired = |of: &Value| of["asked"]["work:test.mark (a rule)"].as_u64().unwrap_or(0);
    let (was, is) = (fired(usual), fired(came));
    if is < was || is > was * (1 + begun_again) + begun_again {
        found.push(format!(
            "the rule that fires every minute fired {is} times, and {was} with nothing in its way, the step having been under way at {begun_again} stops"
        ));
    }
    let seen = |of: &Value, topic: &str| of["said"][topic].as_u64().unwrap_or(0);
    for topic in ["watched.error", "watched.slow"] {
        let (was, is) = (seen(usual, topic), seen(came, topic));
        if is != was {
            found.push(format!("`{topic}` was said {is} times, and {was} with nothing in its way"));
        }
    }
    // The step's own action is safe to do twice: asked once more for each
    // time it was under way at a stop, as anything is.
    let spoke = |of: &Value| of["asked"]["work:test.speak"].as_u64().unwrap_or(0);
    if spoke(came) < spoke(usual) || spoke(came) > spoke(usual) + begun_again {
        found.push(format!(
            "the step was begun {} times, and {} with nothing in its way",
            spoke(came),
            spoke(usual)
        ));
    }
    let rest = |of: &Value| {
        let mut of = of.clone();
        for part in ["said", "asked"] {
            if let Some(counted) = of[part].as_object_mut() {
                for key in ["watched.error", "watched.slow", "work:test.mark (a rule)", "work:test.speak"] {
                    counted.remove(key);
                }
            }
        }
        if let Some(of) = of.as_object_mut() {
            of.remove("stops");
        }
        of
    };
    found.extend(differs(&rest(usual), &rest(came), again));
    found
}

/// Stopped while the step that cannot be done twice was under way, that
/// step fails saying so and the run with it; what was done before it is
/// as it was. Stopped anywhere else, nothing differs.
pub fn not_twice(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    if !again.contains_key("door") {
        return differs(usual, came, again);
    }
    cannot_twice("once-only[once-only]", usual, came)
}

/// What is held of `instance` where its step `door`, which cannot be
/// done twice, was under way when the conductor stopped.
fn cannot_twice(instance: &str, usual: &Value, came: &Value) -> Vec<String> {
    let run = &came["runs"][instance]["main"][0];
    let mut found = Vec::new();
    let why = run["steps"]["door"]["error"].as_str().unwrap_or_default();
    if run["steps"]["door"]["status"] != "failed" || !why.contains(NOT_TWICE) {
        found.push(format!("the step that cannot be done twice came to {}", run["steps"]["door"]));
    }
    if run["status"] != "failed" {
        found.push(format!("its run came to {}", run["status"]));
    }
    if run["steps"]["before"] != usual["runs"][instance]["main"][0]["steps"]["before"] {
        found.push("what was done before it is not as it was".to_string());
    }
    if came["asked"]["after:test.mark"].as_u64().unwrap_or(0) != 0 {
        found.push("the step after it was done".to_string());
    }
    found.extend(failed_once(came, instance));
    found
}

/// The same of a step that cannot be done twice and has a `wait` done as
/// it starts, after one that waits before it is tried again. Wherever the
/// conductor stopped, the action that cannot be done twice is asked once
/// at most: what the waits kept is no place of its own for it to go on
/// from. Stopped with that step under way, it fails saying so as any such
/// step does, where its action had been asked; where it had not (the
/// conductor stopped in what is done as the step starts), nothing was
/// done that cannot be done twice, and the play comes to what it comes to
/// with nothing in its way.
pub fn waited_once(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    once_at_most("waited[waited]", usual, came, again)
}

/// The same of one with several things done as it starts, some of which
/// say something.
pub fn opened_once(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    once_at_most("opens[opens]", usual, came, again)
}

/// What is held of `instance`, whose step `door` cannot be done twice
/// and has things done as it starts: its action is asked once at most,
/// and it is refused as done again only where it had been asked.
fn once_at_most(instance: &str, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut found = Vec::new();
    let asked = came["asked"]["door:test.gate"].as_u64().unwrap_or(0);
    if asked > 1 {
        found.push(format!("the action that cannot be done twice was asked {asked} times"));
    }
    let as_usual = differs(usual, came, again);
    if !again.contains_key("door") {
        found.extend(as_usual);
    } else if !as_usual.is_empty() {
        found.extend(cannot_twice(instance, usual, came));
        if asked == 0 {
            found.push(
                "the step was refused as done again, and its action had never been asked: the conductor \
                 stopped in what is done as the step starts, and nothing was done that cannot be done twice"
                    .to_string(),
            );
        }
    }
    found
}

/// Where an instance's steps failed after the conductor was started: it
/// is failed, says so once and says it is over once, as failed; its run
/// says it failed and that it finished, once each; and nothing was there
/// under its name before it.
pub fn failed_once(came: &Value, instance: &str) -> Vec<String> {
    let mut found = Vec::new();
    if came["instances"][instance]["status"] != "failed" {
        found.push(format!(
            "the instance came to {}, and its steps failed",
            came["instances"][instance]["status"]
        ));
    }
    for topic in ["instance.failed", "instance.finished", "run.failed", "run.finished"] {
        if count(came, topic) != 1 {
            found.push(format!("`{topic}` was said {} times of a run that failed once", count(came, topic)));
        }
    }
    let ended = came["ended"][instance].as_array().cloned().unwrap_or_default();
    if ended.len() != 1 || ended[0]["status"] != "failed" {
        found.push(format!("it said it was over as {}", json!(ended)));
    }
    if !came["earlier"].as_object().is_none_or(|earlier| earlier.is_empty()) {
        found.push(format!("something was there under a name before what is there now: {}", came["earlier"]));
    }
    found
}

/// An action that is taken up where it was says so in what it gives:
/// that, and nothing else, is other than with nothing in its way.
pub fn taken_up(place: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let build = |of: &Value| of["runs"]["picked-up[picked-up]"]["main"][0]["steps"]["build"].clone();
    // Stopped as the step began, before its action had kept anything, it
    // is a step that cannot be done twice like any other. Where what its
    // action kept was written all the same, it is taken up there.
    if place.topic == "step.started" && place.step == "build" {
        let why = build(came)["error"].as_str().unwrap_or_default().to_string();
        if why.contains(NOT_TWICE) {
            // Then it is that and nothing else: the step failed and its
            // run with it, the step after it was never done, and the
            // instance failed, saying so once.
            let run = &came["runs"]["picked-up[picked-up]"]["main"][0];
            let mut found = failed_once(came, "picked-up[picked-up]");
            if build(came)["status"] != "failed" || run["status"] != "failed" {
                found.push(format!(
                    "the step came to {} and its run to {}",
                    build(came)["status"],
                    run["status"]
                ));
            }
            if came["asked"]["after:test.echo"].as_u64().unwrap_or(0) != 0 {
                found.push("the step after it was done".to_string());
            }
            if run["steps"]["after"]["status"] == "ok" {
                found.push(format!("the step after it came to {}", run["steps"]["after"]));
            }
            return found;
        }
    }
    let mut same = came.clone();
    same["runs"]["picked-up[picked-up]"]["main"][0]["steps"]["build"] = build(usual);
    let mut found = differs(usual, &same, again);
    if build(came)["status"] != "ok" {
        found.push(format!("the step that was taken up came to {}", build(came)));
    }
    found
}

/// Two handlers side by side set in whichever order they come to it.
/// Which of them was given which number is not held; that every number
/// was given, and given once, is, with what the instance keeps.
pub fn in_any_order(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let given = |of: &Value| {
        let (mut of, mut all) = (of.clone(), Vec::new());
        for flow in ["on.one", "on.two"] {
            for run in of["runs"]["adds[adds]"][flow].as_array_mut().into_iter().flatten() {
                all.push(run["steps"]["add"]["gave"]["n"].take().as_u64());
            }
        }
        all.sort();
        (of, all)
    };
    let ((usual, was), (came, is)) = (given(usual), given(came));
    let mut found = differs(&usual, &came, again);
    if was != is {
        found.push(format!("the numbers given were {is:?}, and {was:?} with nothing in its way"));
    }
    found
}

/// One that has no handlers and holds nothing is active only for what it
/// started, and done when the last of that has ended ([`conductor`]'s
/// `Engine::rests`). With nothing in its way, what it started may still be
/// running as its steps end, and it is said to be active and then done;
/// stopped and started, what it started may have ended first, and it is
/// done with nothing said of being active. So it may say it is active
/// once fewer, and nothing else: how it ended is held as it always is.
pub fn rested(_: &Place, usual: &Value, came: &Value, again: &BTreeMap<String, usize>) -> Vec<String> {
    let mut came = came.clone();
    if count(usual, "instance.active").saturating_sub(count(&came, "instance.active")) == 1 {
        came["said"]["instance.active"] = usual["said"]["instance.active"].clone();
    }
    differs(usual, &came, again)
}
