//! What a play is looked at by, from outside: where its cards are, what is
//! in its repository, what its channel was told, what is alive; and the
//! stand-ins its agents are.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use axum::http::StatusCode;
use serde_json::{Value, json};

use crate::Rig;

/// What a successful GET answers; nothing, where what is asked for is
/// not there. A play that brings something up is looked at before it is
/// there, and what is not there yet has said and been asked nothing.
async fn read_if_there(rig: &Rig, path: &str) -> Value {
    let (status, body) = rig.get(path).await;
    if status == StatusCode::NOT_FOUND {
        return Value::Null;
    }
    assert_eq!(status, StatusCode::OK, "GET {path}: {body}");
    body
}

/// Where every card of a project's board is, and how often it was sent back.
pub async fn cards(rig: &Rig, project: &str) -> Value {
    let board = rig.read(&format!("/api/board/{project}")).await;
    let cards: BTreeMap<String, Value> = board["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| {
            let rounds = card["more"]["rounds"].as_u64().unwrap_or(0);
            (card["id"].as_str().unwrap().to_string(), json!({ "in": card["status"], "sent back": rounds }))
        })
        .collect();
    json!(cards)
}

/// What is on each branch of a repository, the latest first, and which
/// cards still have a place of their own in it. Work done twice over is
/// one piece of work: a commit that says what the one before it said is
/// not counted again.
pub fn repository(dir: &Path) -> Value {
    let git = |args: &[&str]| {
        let ran =
            crate::lanes::asking(dir).args(["-c", "core.hooksPath=/dev/null"]).args(args).output().unwrap();
        String::from_utf8_lossy(&ran.stdout).trim().to_string()
    };
    let mut branches = BTreeMap::new();
    for branch in git(&["branch", "--format=%(refname:short)"]).lines() {
        let mut said: Vec<String> = Vec::new();
        // In the order they follow from each other, and not by the clock:
        // two commits of one second have no order of their own.
        for subject in git(&["log", "--topo-order", "--format=%s", branch]).lines() {
            if said.last().map(String::as_str) != Some(subject) {
                said.push(subject.to_string());
            }
        }
        branches.insert(branch.to_string(), said);
    }
    let mut places: Vec<String> = std::fs::read_dir(dir.join(".lanes"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    places.sort();
    // What lies among the project's own files that nobody committed.
    let loose: Vec<String> = git(&["status", "--porcelain"]).lines().map(str::to_string).collect();
    json!({ "branches": branches, "places": places, "uncommitted": loose })
}

/// What was said in a channel, each thing once, whoever said it how often.
pub async fn said(rig: &Rig, channel: &str) -> Value {
    let messages = read_if_there(rig, &format!("/api/channels/{channel}/messages")).await;
    let said: BTreeSet<String> = messages["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|message| message["body"].as_str().unwrap_or_default().chars().take(240).collect())
        .collect();
    json!(said)
}

/// How many instances of each blueprint are alive, in a project or anywhere.
pub async fn alive(rig: &Rig, project: Option<&str>) -> Value {
    let filter = conductor::InstanceFilter {
        live_only: true,
        project: project.map(str::to_string),
        limit: 100_000,
        ..Default::default()
    };
    let mut alive: BTreeMap<String, u64> = BTreeMap::new();
    for instance in rig.node().app.engine.instances(filter).await.unwrap() {
        *alive.entry(instance.blueprint.clone()).or_insert(0) += 1;
    }
    json!(alive)
}

/// How many runs are under way: at rest, none.
pub async fn under_way(rig: &Rig) -> Value {
    let runs = rig.node().app.engine.recent_runs(500, None).await.unwrap();
    json!(runs.iter().filter(|run| run.status == conductor::RunStatus::Running).count())
}

/// What is still being done in a factory: how many runs of the runner are
/// under way, how many turns of agents are in hand (a program at work, or
/// its answer kept and the turn not yet said to have ended), and each of
/// them in words.
///
/// A turn is counted beside the runs because a program can be at work
/// with no run of the runner waiting on it: a turn somebody asked for in
/// a channel, and one a daemon found in hand and took up before whatever
/// asked for it had been begun again. A service is not counted: it is
/// meant to stay.
pub async fn at_work(rig: &Rig) -> (usize, usize, Vec<String>) {
    let app = &rig.node().app;
    let mut said = Vec::new();
    let mut running = 0;
    for run in runs_under_way(rig).await {
        running += 1;
        let of = app.engine.instance(&run.instance).await.map(|instance| instance.blueprint);
        said.push(format!("a run of {} ({})", of.unwrap_or_else(|_| run.instance.clone()), run.flow));
    }
    // In every file that keeps turns: a project's agents' are in its own.
    let mut turns: Vec<String> = Vec::new();
    for journal in app.journal.every_keeping(journal::keeps::TURNS) {
        let in_hand = journal.db().call(|connection| {
            let mut statement =
                connection.prepare("SELECT slot FROM turns WHERE status IN ('running', 'answered')")?;
            let slots = statement.query_map([], |row| row.get(0))?;
            slots.collect::<rusqlite::Result<Vec<String>>>()
        });
        turns.extend(in_hand.await.unwrap_or_default());
    }
    said.extend(turns.iter().map(|slot| format!("a turn of {slot}")));
    (running, turns.len(), said)
}

/// The runs under way, from what the runner keeps of them in every file
/// that keeps runs, and not from the newest runs there were: read so,
/// looking cost more the more runs a factory had made, up to the five
/// hundred read, and a count of what a card's move costs (`costed.rs`)
/// grew with every row another table had that a run wrote.
async fn runs_under_way(rig: &Rig) -> Vec<conductor::RunInfo> {
    let app = &rig.node().app;
    let mut under_way: Vec<String> = Vec::new();
    for journal in app.journal.every_keeping(journal::keeps::RUNS) {
        let runs = journal.db().call(|connection| {
            let mut statement = connection.prepare("SELECT run FROM under_way")?;
            let runs = statement.query_map([], |row| row.get(0))?;
            runs.collect::<rusqlite::Result<Vec<String>>>()
        });
        under_way.extend(runs.await.unwrap_or_default());
    }
    let mut runs = Vec::new();
    for run in under_way {
        if let Ok(run) = app.engine.run(&run).await
            && run.status == conductor::RunStatus::Running
        {
            runs.push(run);
        }
    }
    runs
}

/// Where each run under way is: each step of it at work, with the last
/// line its log has, which is what an action says it waits for. What
/// the journal kept does not say it: a run that keeps nothing as it goes
/// says its steps only in passing (a merge that waited for ever on a
/// clock nobody moved, `slowing.rs`, said nothing after it began).
pub async fn waiting(rig: &Rig) -> Vec<String> {
    let app = &rig.node().app;
    let mut said = Vec::new();
    for run in runs_under_way(rig).await {
        let of = app.engine.instance(&run.instance).await.map(|instance| instance.blueprint);
        let of = format!("{} ({})", of.unwrap_or_else(|_| run.instance.clone()), run.flow);
        for step in run.steps.iter().filter(|step| step.status == conductor::StepStatus::Running) {
            let last = app.engine.logs().tail("runs", &format!("{}.{}", run.id, step.step), 1);
            let last = last.first().map_or("nothing", |line| line.text.as_str()).to_string();
            said.push(format!("{of}, step `{}`, which last wrote: {last}", step.step));
        }
    }
    said
}

/// How often each of these agents was asked.
pub async fn asked(rig: &Rig, agents: &[&str]) -> BTreeMap<String, u64> {
    let mut asked = BTreeMap::new();
    for agent in agents {
        let turns = read_if_there(rig, &format!("/api/agents/{agent}/turns")).await;
        asked.insert((*agent).to_string(), turns["turns"].as_array().map_or(0, Vec::len) as u64);
    }
    asked
}

/// What each row of one of a project's tables is called, in order of
/// their names; none, where there is no such table.
pub async fn rows(rig: &Rig, project: &str, table: &str) -> Value {
    let rows = read_if_there(rig, &format!("/api/collections/{project}/{table}/records")).await;
    let mut called: Vec<String> = rows["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|row| row["data"]["title"].as_str().unwrap_or_default().to_string())
        .collect();
    called.sort();
    json!(called)
}

/// Every agent there is by name, and whether it takes work.
pub async fn agents(rig: &Rig) -> Value {
    let all = rig.read("/api/agents").await;
    let agents: BTreeMap<String, Value> = all["agents"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|agent| (agent["name"].as_str().unwrap_or_default().to_string(), agent["active"].clone()))
        .collect();
    json!(agents)
}

/// A node whose agents are stand-ins: one that changes a file where it
/// is run and says so, and one that approves what it is shown. Each says
/// that its program is there, waits to be let go, and writes down that it
/// has ended. The node, and the folder their pipes are in.
///
/// The one that works writes a file called after what it was asked, so
/// that asked the same thing twice it has done one piece of work, and two
/// cards never change the same file: what an agent makes of being asked
/// again is the agent's, and is not what is being played.
pub async fn node_with_stand_ins(label: &str) -> (Rig, PathBuf) {
    node_whose_writer(label, &["write", "CHANGES-%:sum%.md", "changed", "say", "I changed a file."]).await
}

/// The same, where the one that works does `does` in place of changing
/// a file: asks, say.
pub async fn node_whose_writer(label: &str, does: &[&str]) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with(label, |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        let (began, hold, ended) = (file("began"), file("hold"), file("ended"));
        // It reads what it is asked, says its program is there, waits to
        // be let go, does its work, and says it has ended.
        let stands_in = |who: &str, does: &[&str]| {
            let first = ["read", "tell", &began, who, "held", &hold, who];
            crate::standin::argv(&[&first[..], does, &["append", &ended, who][..]].concat())
        };
        // A file called by a number of what was asked: the same asking
        // twice is the same file.
        let writes = stands_in("writer", does);
        let looks = stands_in("critic", &["say", "It does what the card asks.", "say", "DECISION: approve"]);
        config.adapters.push(("writer".into(), writes));
        config.adapters.push(("critic".into(), looks));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}
