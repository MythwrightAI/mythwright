//! What every run says of itself, held in every play.
//!
//! A play is held to coming to what it comes to with nothing in its way,
//! and what it came to is what its instances are, how their runs ended
//! and what was said that is a blueprint's to say. What the conductor
//! says of its own runs is not counted there: a run taken up says so,
//! and that is no part of what a blueprint came to. But some of it is
//! true of every run wherever the conductor was stopped, and whatever
//! hears it is owed it once: a run that has ended said that it finished,
//! once; none of its steps reads as under way; a run that was taken up,
//! or that ended well, said that it started; and none says so twice.
//! Each that is not so is a sentence here, and a play has none.

use std::collections::HashMap;

use conductor::{InstanceInfo, RunStatus};

use crate::Rig;

/// How often each run said each of the things a run says of itself.
#[derive(Default)]
struct Said {
    started: usize,
    resumed: usize,
    finished: usize,
}

/// What is not so of the runs of `all`, each in a sentence that names
/// the instance by `path` and never by an id.
pub async fn amiss(rig: &Rig, all: &[InstanceInfo], path: &dyn Fn(&str) -> String) -> Vec<String> {
    let mut said: HashMap<String, Said> = HashMap::new();
    for event in rig.events("run.started,run.resumed,run.finished").await {
        let Some(run) = event.source.run.clone() else { continue };
        let of = said.entry(run).or_default();
        match event.topic.as_str() {
            "run.started" => of.started += 1,
            "run.resumed" => of.resumed += 1,
            _ => of.finished += 1,
        }
    }
    let mut found = Vec::new();
    for one in all.iter().rev() {
        let mut runs = rig.engine.runs(&one.id, 10_000).await.unwrap();
        runs.sort_by_key(|run| run.started);
        for run in runs {
            let whole = rig.engine.run(&run.id).await.unwrap();
            let of = said.remove(&run.id).unwrap_or_default();
            let what = format!("a run of {} ({})", path(&one.id), whole.flow);
            if of.started > 1 {
                found.push(format!("{what} said that it started {} times", of.started));
            }
            if of.started == 0 && (of.resumed > 0 || whole.status == RunStatus::Ok) {
                found.push(format!("{what} went on, and never said that it started"));
            }
            if whole.status == RunStatus::Running {
                continue;
            }
            if of.finished != 1 {
                found.push(format!(
                    "{what} ended as {}, and said that it finished {} times",
                    whole.status.as_str(),
                    of.finished
                ));
            }
            for step in whole.steps.iter().filter(|step| !step.status.is_settled()) {
                found.push(format!(
                    "{what} ended as {}, and its step `{}` still reads as {}",
                    whole.status.as_str(),
                    step.step,
                    step.status.as_str()
                ));
            }
        }
    }
    found.sort();
    found
}
