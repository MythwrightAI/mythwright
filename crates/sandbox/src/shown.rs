//! What a sandbox hands back: everything that came of it, read from its
//! own journal as a factory's is read.
//!
//! Its instances and runs are the shapes the daemon hands out for real
//! ones, each run with its steps in the order they began; its events
//! are in the order they were said; its tables are each with their
//! rows, and its channels each with what was said there. Beside those,
//! what no real run has: every asking of a stand-in with what it was
//! asked and what it answered, what kind each action was, and the same
//! things as one list in the order they came about ([`Inside::timeline`]),
//! for whoever prints it. All of it says that it was a sandbox.

use std::collections::{BTreeMap, HashMap, HashSet};

use blueprint::{Blueprint, Step};
use conductor::{InstanceFilter, RunInfo};
use journal::{EventQuery, MetricQuery};
use kernel::{Clock, Event, Result};
use serde_json::{Value, json};

use crate::inside::Inside;
use crate::offered::{Done, Kind};
use crate::stood::Asking;

/// The step called `id` of `flow` in `blueprint`: among its own steps,
/// or those of the handler the flow is, and inside steps done as one.
pub(crate) fn step_of<'a>(blueprint: &'a Blueprint, flow: &str, id: &str) -> Option<&'a Step> {
    fn among<'a>(steps: &'a [Step], id: &str) -> Option<&'a Step> {
        steps.iter().find_map(|step| match step.id == id {
            true => Some(step),
            false => among(&step.inner.steps, id),
        })
    }
    match flow.strip_prefix("on.") {
        Some(handler) => blueprint
            .triggers
            .iter()
            .filter(|trigger| trigger.name == handler)
            .find_map(|trigger| among(&trigger.flow.steps, id)),
        None => among(&blueprint.flow.steps, id),
    }
}

/// What is said of a step beside its key: what it says of itself, as far
/// as its first full stop, and what it does.
fn said_of(step: Option<&Step>) -> (String, String) {
    let Some(step) = step else { return (String::new(), String::new()) };
    let says = step.description.as_deref().map(blueprint::summary::says).unwrap_or_default();
    (says, format!("{} {}", step.kind.verb(), step.kind.target()))
}

impl Inside {
    /// Every event the sandbox said since it was made, in the order said:
    /// what its journal keeps, and what the runner said of its own steps
    /// and runs and passed on without keeping, which its order holds.
    async fn events(&self) -> Result<Vec<Event>> {
        let mut events =
            self.journal.read_every(EventQuery { limit: crate::MOST, ..EventQuery::default() }).await?;
        events.extend(self.order.said_passing().iter().map(|event| Event::clone(event)));
        events.retain(|event| self.order.of(event.seq).is_some_and(|at| at > self.made_at));
        events.sort_by_key(|event| self.order.of(event.seq));
        Ok(events)
    }

    /// Every run, each with its steps, in the order they began.
    async fn runs(&self, events: &[Event]) -> Result<Vec<RunInfo>> {
        let mut runs = Vec::new();
        for listed in self.engine.recent_runs(crate::MOST, None).await? {
            runs.push(self.engine.run(&listed.id).await.unwrap_or(listed));
        }
        // A clock that stands still says nothing of which began first:
        // the order their beginnings were said in does.
        let began: HashMap<&str, usize> = events
            .iter()
            .enumerate()
            .filter(|(_, event)| event.topic == "run.started")
            .filter_map(|(at, event)| Some((event.source.run.as_deref()?, at)))
            .collect();
        runs.sort_by_key(|run| began.get(run.id.as_str()).copied().unwrap_or(usize::MAX));
        Ok(runs)
    }

    async fn tables(&self) -> Result<Vec<Value>> {
        let mut tables = Vec::new();
        for table in self.studio.collections.list(None).await? {
            let records = self.studio.collections.records(&table).await?;
            let mut shown = json!(table);
            shown["records"] = json!(records);
            tables.push(shown);
        }
        Ok(tables)
    }

    async fn channels(&self) -> Result<Vec<Value>> {
        let mut channels = Vec::new();
        for channel in self.studio.channels.readable(&self.owner).await? {
            let reader = self.studio.channels.may_read(&channel.id, &self.owner).await?;
            let said = self.studio.channels.messages(&reader, &channel.id, None, 500).await?;
            let mut shown = json!(channel);
            shown["messages"] = json!(said);
            channels.push(shown);
        }
        Ok(channels)
    }

    /// What was told to whom: the lines of the made-up person's inbox,
    /// the latest first, put away or not. Theirs is the one inbox a
    /// sandbox has: a handle names an account of the sandbox's own, of
    /// which there is that one, and an agent has no inbox.
    async fn told(&self) -> Result<Value> {
        Ok(json!(self.studio.channels.inbox.list(&self.owner, true).await?))
    }

    /// How the sandbox stands: what waits, whether it was stopped, and
    /// what its clock says.
    pub fn standing(&self) -> Value {
        let asked = self.record.asked();
        let stood = stood(&asked);
        let waiting: Vec<Asking> = asked.into_iter().filter(|asking| asking.waiting).collect();
        json!({
            "sandbox": self.id,
            "said": crate::SAID,
            "blueprint": self.name,
            "project": crate::PROJECT,
            "clock": self.clock.now_ms(),
            "began": self.began,
            "applying": self.is_applying(),
            "waiting": waiting,
            "over": self.over,
            "events": self.said(),
            "steps": self.ran,
            "keeps": self.keeps,
            "logged": self.logs.written(),
            "limits": { "steps": self.limits.steps, "events": self.limits.events, "bytes": self.limits.bytes },
            "stood_in": self.record.count(),
            "stood": stood.iter().map(|(action, times)| json!({ "action": action, "times": times })).collect::<Vec<_>>(),
            "stood_said": stood_said(&stood),
            "does": crate::inside::DOES,
        })
    }

    /// Every action the sandbox has: what it says of itself, and its kind.
    pub fn offers(&self) -> Value {
        let offered: Vec<Value> = self
            .engine
            .registry()
            .all_specs()
            .into_iter()
            .map(|spec| json!({ "kind": self.kinds.get(&spec.name), "action": spec }))
            .collect();
        json!(offered)
    }

    /// Everything that came of the sandbox.
    pub async fn shown(&self) -> Result<Value> {
        let events = self.events().await?;
        let runs = self.runs(&events).await?;
        let found =
            self.engine.instances(InstanceFilter { limit: crate::MOST, ..InstanceFilter::default() }).await?;
        let texts: HashMap<String, Blueprint> = found
            .iter()
            .filter_map(|instance| {
                let read = blueprint::check(&instance.source, &blueprint::NoCatalog).blueprint?;
                Some((instance.id.clone(), read))
            })
            .collect();
        let asked = self.record.asked();
        // Where each thing said came in the order of what was said, for
        // what names no run: which action was at work then says which.
        let said_for = SaidFor { doing: self.doing.so_far(), at: |event: &Event| self.order.of(event.seq) };
        let timeline =
            timeline(&events, &runs, &texts, &asked, &said_for, |run, step| self.handed(run, step));
        let stood: BTreeMap<&String, &Kind> =
            self.kinds.iter().filter(|(_, kind)| **kind == Kind::Stood).collect();
        let mut shown = self.standing();
        shown["applied"] = json!(self.applied);
        shown["instances"] = json!(self.engine.shown(found));
        shown["runs"] = json!(runs);
        shown["happened"] = json!(events);
        shown["tables"] = json!(self.tables().await?);
        shown["channels"] = json!(self.channels().await?);
        shown["inbox"] = self.told().await?;
        shown["somebody"] = json!(self.owner.actor());
        shown["asked"] = json!(asked);
        shown["actions"] = json!(self.kinds);
        shown["stands_in_for"] = json!(stood.keys().collect::<Vec<_>>());
        shown["timeline"] = json!(timeline);
        shown["measured"] = self.measured().await?;
        Ok(shown)
    }

    /// What was measured of what ran, in all since the sandbox was
    /// made: each series by its name, the blueprint it is of and what
    /// was chosen, with its totals. What the factory measures of its own
    /// work, of no blueprint, is left out: it is the same whatever text
    /// is run. A clock that stands still makes every length of time
    /// nought, and one moved on makes what waited take that long.
    async fn measured(&self) -> Result<Value> {
        let query = MetricQuery {
            name: "*".into(),
            group_by: vec!["blueprint".into(), "decision".into()],
            ..MetricQuery::default()
        };
        let series = self.journal.query_metrics(query).await?;
        let of_a_blueprint = series
            .into_iter()
            .filter(|series| series.labels.get("blueprint").is_some_and(|name| !name.is_empty()))
            .map(|series| json!({ "name": series.name, "labels": series.labels, "total": series.total }));
        Ok(json!(of_a_blueprint.collect::<Vec<_>>()))
    }

    /// What a step was handed, as its log's first line says it.
    fn handed(&self, run: &str, step: &str) -> Option<Value> {
        let lines = self.engine.step_log(run, step, 0, 4);
        let said = lines.iter().find_map(|line| line.text.strip_prefix("handed"))?;
        let (_, args) = said.split_once(": ")?;
        serde_json::from_str(args).ok()
    }
}

/// Each action a stand-in was asked for, with how many times.
fn stood(asked: &[Asking]) -> BTreeMap<String, usize> {
    let mut stood = BTreeMap::new();
    for asking in asked {
        *stood.entry(asking.action.clone()).or_insert(0) += 1;
    }
    stood
}

/// What was stood in, in a sentence: said here once, for whatever shows
/// a sandbox, so that none of them words it for itself.
fn stood_said(stood: &BTreeMap<String, usize>) -> String {
    if stood.is_empty() {
        return "Nothing was stood in: nothing this did reaches outside the factory's tables.".to_string();
    }
    let actions: Vec<String> = stood.keys().map(|action| format!("`{action}`")).collect();
    let times = match stood.values().sum::<usize>() {
        1 => "once".to_string(),
        many => format!("{many} times"),
    };
    format!(
        "Stood in, and so not exercised: {}. Asked {times} in all, they ran nothing, and answered what a \
         stand-in was told to answer.",
        actions.join(", ")
    )
}

/// One step of a run as a line of the timeline.
fn step_line(
    run: &RunInfo,
    step: &conductor::StepInfo,
    text: Option<&Blueprint>,
    asked: &[Asking],
    handed: &impl Fn(&str, &str) -> Option<Value>,
) -> Value {
    let (says, does) = said_of(text.and_then(|text| step_of(text, &run.flow, &step.step)));
    let stood: Vec<u64> = asked
        .iter()
        .filter(|asking| asking.run == run.id && asking.step == step.step)
        .map(|asking| asking.id)
        .collect();
    json!({
        "what": "step",
        "instance": run.instance,
        "run": run.id,
        "flow": run.flow,
        "step": step.step,
        "says": says,
        "does": does,
        "status": step.status,
        "tolerated": step.tolerated,
        "handed": handed(&run.id, &step.step),
        "gave": step.output,
        "error": step.error,
        "stood": stood,
    })
}

/// Which run something that names none was said for: the one an action
/// of the sandbox's own was at work for as it was said.
struct SaidFor<F> {
    doing: Vec<Done>,
    at: F,
}

impl<F: Fn(&Event) -> Option<u64>> SaidFor<F> {
    /// The run `event` was said for, where it names no run and no
    /// instance, was said as somebody (what follows the journal for the
    /// factory itself says things as the system, for no run), and the
    /// run is known: one run alone had an action at work as it was
    /// said; or of several, one alone was of an instance the event
    /// names; or one alone had an action of the event's own family at
    /// work (`channel.ensure` says `channel.created`).
    fn run<'a>(&self, event: &Event, by_id: &HashMap<&str, &'a RunInfo>) -> Option<&'a RunInfo> {
        let by_the_system = event.source.actor.as_deref() == Some("system");
        if event.source.run.is_some() || event.source.instance.is_some() || by_the_system {
            return None;
        }
        let at = (self.at)(event)?;
        let then: Vec<(&'a RunInfo, &str)> = self
            .doing
            .iter()
            .filter(|one| one.from < at && one.to.is_none_or(|to| at <= to))
            .filter_map(|one| Some((*by_id.get(one.run.as_str())?, one.action.as_str())))
            .collect();
        // The one run every one of `those` is of, where there is one.
        let alone = |those: Vec<&'a RunInfo>| {
            let first = *those.first()?;
            those.iter().all(|run| run.id == first.id).then_some(first)
        };
        let names = |run: &RunInfo| {
            event
                .data
                .as_object()
                .is_some_and(|said| said.values().any(|value| *value == json!(run.instance)))
        };
        let family = |name: &str| name.split('.').next().map(str::to_string);
        let akin = |action: &str| family(action) == family(&event.topic);
        alone(then.iter().map(|(run, _)| *run).collect())
            .or_else(|| alone(then.iter().filter(|(run, _)| names(run)).map(|(run, _)| *run).collect()))
            .or_else(|| alone(then.iter().filter(|(_, action)| akin(action)).map(|(run, _)| *run).collect()))
    }
}

/// What happened, a line for each thing in the order it came about: an
/// instance beginning and ending, a run beginning and ending, each step
/// as it ended (or as it stands, of a run that has not), and whatever
/// else was said. A step that keeps nothing as it goes says nothing of
/// itself, and is put where its run ended. What an action of the
/// sandbox's own said of a table or a channel names no run, and is put
/// with the run it was said for where that is known (`said_for`).
fn timeline(
    events: &[Event],
    runs: &[RunInfo],
    texts: &HashMap<String, Blueprint>,
    asked: &[Asking],
    said_for: &SaidFor<impl Fn(&Event) -> Option<u64>>,
    handed: impl Fn(&str, &str) -> Option<Value>,
) -> Vec<Value> {
    let by_id: HashMap<&str, &RunInfo> = runs.iter().map(|run| (run.id.as_str(), run)).collect();
    let mut shown: HashSet<(String, String)> = HashSet::new();
    let mut lines = Vec::new();
    let text_of = |run: &RunInfo| texts.get(&run.instance);
    // The steps of `run` no line has said yet, as they now stand.
    let rest = |run: &RunInfo, shown: &mut HashSet<(String, String)>, lines: &mut Vec<Value>| {
        for step in &run.steps {
            if shown.insert((run.id.clone(), step.step.clone())) {
                lines.push(step_line(run, step, text_of(run), asked, &handed));
            }
        }
    };
    // The run of each instance that began last and has not ended: what
    // an action says for its step names the instance and not always the
    // run, and is put with that one.
    let mut latest: HashMap<&str, &RunInfo> = HashMap::new();
    for event in events {
        let said = |name: &str| event.data.get(name).cloned().unwrap_or(Value::Null);
        let run = event.source.run.as_deref().and_then(|id| by_id.get(id).copied());
        match (event.topic.as_str(), run) {
            ("run.started", Some(run)) => {
                latest.insert(run.instance.as_str(), run);
            }
            ("run.finished", Some(run))
                if latest.get(run.instance.as_str()).is_some_and(|one| one.id == run.id) =>
            {
                latest.remove(run.instance.as_str());
            }
            _ => {}
        }
        let run = run
            .or_else(|| event.source.instance.as_deref().and_then(|id| latest.get(id).copied()))
            .or_else(|| said_for.run(event, &by_id));
        let (kind, did) = event.topic.split_once('.').unwrap_or((event.topic.as_str(), ""));
        match (kind, did, run) {
            ("instance", "state", _) => {}
            ("instance", did, _) => lines.push(json!({
                "what": "instance",
                "did": did,
                "instance": event.source.instance,
                "blueprint": event.source.instance.as_ref().and_then(|id| texts.get(id)).map(|text| &text.name),
                "key": said("key"),
                "parent": said("parent"),
                "status": said("status"),
                "error": said("error"),
            })),
            ("run", "started", Some(run)) => lines.push(json!({
                "what": "run",
                "did": "started",
                "instance": run.instance,
                "run": run.id,
                "flow": run.flow,
                "blueprint": said("blueprint"),
                "heard": run.event.get("topic"),
            })),
            ("run", "finished", Some(run)) => {
                rest(run, &mut shown, &mut lines);
                lines.push(json!({
                    "what": "run",
                    "did": "finished",
                    "instance": run.instance,
                    "run": run.id,
                    "flow": run.flow,
                    "blueprint": said("blueprint"),
                    "status": said("status"),
                    "error": said("error"),
                }));
            }
            ("run", _, _) => {}
            ("step", "finished" | "skipped", Some(run)) => {
                let named = event.source.step.as_deref().unwrap_or_default();
                if let Some(step) = run.steps.iter().find(|step| step.step == named)
                    && shown.insert((run.id.clone(), named.to_string()))
                {
                    lines.push(step_line(run, step, text_of(run), asked, &handed));
                }
            }
            ("step", "waiting", Some(run)) => lines.push(json!({
                "what": "waits",
                "instance": run.instance,
                "run": run.id,
                "flow": run.flow,
                "step": event.source.step,
                "event": said("event"),
                "until": said("until"),
            })),
            ("step", _, _) => {}
            _ => lines.push(json!({
                "what": "said",
                "topic": event.topic,
                "by": event.source.actor,
                "instance": event.source.instance.as_ref().or(run.map(|run| &run.instance)),
                "run": run.map(|run| &run.id),
                "flow": run.map(|run| &run.flow),
                "step": event.source.step,
                "data": event.data,
            })),
        }
    }
    // What is still under way, as it stands: a step that waits for a
    // stand-in to be answered is here.
    for run in runs.iter().filter(|run| run.finished.is_none()) {
        rest(run, &mut shown, &mut lines);
    }
    lines
}
