//! A part of rows, of runs and of measures as whoever's line it is reads
//! it now (`kernel::parts`), each read with their authority and nothing
//! read for them that they may not read otherwise: a table's rows as its
//! records are read (`records:read`, and what a private table is kept
//! under), runs as what runs is listed (`blueprints:read`), and
//! measurements as they are (`fleet:read`). Whoever may not is shown none
//! of it, and told why in the part's own place.
//!
//! What is chosen, which columns, what a run is called and what a
//! measurement comes to are the kernel's, with their cases; what is here
//! is the reading.

use conductor::{Engine, InstanceFilter};
use environments::last;
use journal::{Journal, MetricQuery};
use kernel::parts::{Chosen, Measures, Rows, Runs, RunsOf};
use kernel::rows::{Asked, Cell};
use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};
use studio::Studio;

/// How far back a part of measures reads: as far as a month.
const MONTH_MS: i64 = 30 * 24 * 3_600_000;

/// The rows a part of rows lists, with the columns it shows, each column
/// as its table declares it and whether it holds seconds.
pub async fn rows(studio: &Studio, who: &Principal, rows: &Rows) -> Result<Value> {
    let table = blueprint::expr::slug(&rows.table);
    // Read as its records are, which says first whether they may.
    let records = studio.records.list(&rows.project, &table, who).await?;
    let collection = studio.collections.get(&rows.project, &table).await?;
    let ids: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
    let those: Vec<String> = match &rows.saying {
        None => Vec::new(),
        Some(saying) => {
            // Chosen as `rows.read` chooses them: of what its blueprint declares.
            let cells: Vec<Cell<'_>> = collection
                .fields
                .iter()
                .filter(|field| !field.added)
                .map(|field| Cell { name: &field.name, kind: &field.kind })
                .collect();
            let pairs: Vec<(String, Map<String, Value>)> =
                records.iter().map(|record| (record.id.clone(), record.data.clone())).collect();
            let asked = Asked { saying: Some(saying), ..Asked::default() };
            let read = kernel::rows::read(&table, &cells, &pairs, &asked).map_err(Error::invalid)?;
            let those = read["those"].as_array().into_iter().flatten().filter_map(Value::as_str);
            those.map(str::to_string).collect()
        }
    };
    let those: Vec<&str> = those.iter().map(String::as_str).collect();
    let (listed, more) = kernel::parts::chosen(rows, &ids, &those);
    let declared: Vec<&str> = collection.fields.iter().map(|field| field.name.as_str()).collect();
    let (shown, unknown) = kernel::parts::columns(rows, &declared);
    let columns: Vec<Value> = shown
        .iter()
        .filter_map(|name| collection.fields.iter().find(|field| &field.name == name))
        .map(|field| {
            let mut column = json!(field);
            column["seconds"] = json!(kernel::parts::in_seconds(&field.name, &field.kind));
            column
        })
        .collect();
    let listed: Vec<Value> = listed
        .into_iter()
        .map(|one| match one {
            Chosen::At(at) => {
                let record = &records[at];
                let cells: Map<String, Value> = shown
                    .iter()
                    .map(|cell| {
                        (cell.clone(), record.data.get(cell).map(kernel::parts::brief).unwrap_or(Value::Null))
                    })
                    .collect();
                json!({ "id": record.id, "cells": cells })
            }
            Chosen::Gone(id) => json!({ "id": id, "gone": true }),
        })
        .collect();
    Ok(json!({ "columns": columns, "rows": listed, "more": more, "unknown": unknown }))
}

/// The runs a part of runs lists, the newest first: each with what it is
/// a run of, how it went, when it began and when it ended.
pub async fn runs(engine: &Engine, who: &Principal, runs: &Runs) -> Result<Value> {
    who.require("blueprints:read")?;
    let of = |project: &str| InstanceFilter {
        project: Some(project.to_string()),
        limit: last::THINGS,
        ..InstanceFilter::default()
    };
    let listed = match &runs.of {
        RunsOf::Instance(id) => {
            let info = engine.instance(id).await?;
            if !runs.project.is_empty() && info.project.as_deref() != Some(runs.project.as_str()) {
                return Err(Error::not_found(format!("`{id}` is nothing that runs in `{}`", runs.project)));
            }
            shown(&info, engine.runs(id, runs.most).await?)
        }
        RunsOf::Key(key) => {
            // Every instance of the project under the key, a pipeline applied
            // again being another each time its steps are over: theirs, the newest first.
            let mut under: Vec<conductor::InstanceInfo> = engine
                .instances(of(&runs.project))
                .await?
                .into_iter()
                .filter(|one| &one.key == key)
                .collect();
            under.sort_by_key(|one| std::cmp::Reverse(one.created));
            let mut listed = Vec::new();
            for one in under.iter().take(runs.most) {
                listed.extend(shown(one, engine.runs(&one.id, runs.most).await?));
            }
            listed.sort_by_key(|run| std::cmp::Reverse(run["started"].as_i64()));
            listed.truncate(runs.most);
            listed
        }
        RunsOf::Environment(name) => {
            // What ran under it, as its page lists it (`environments::last`).
            let things = engine.instances(of(&runs.project)).await?;
            let ran = engine.recent_runs(last::RUNS, Some(runs.project.clone())).await?;
            let read = last::Read {
                runs: ran
                    .into_iter()
                    .map(|run| last::Once {
                        status: run.status.as_str().to_string(),
                        id: run.id,
                        instance: run.instance,
                        started: run.started,
                    })
                    .collect(),
                things: things
                    .into_iter()
                    .map(|one| last::Thing {
                        live: one.status.is_live(),
                        by_name: one.principal.name,
                        id: one.id,
                        parent: one.parent,
                        environment: one.environment,
                        blueprint: one.blueprint,
                        key: one.key,
                    })
                    .collect(),
                whole: true,
            };
            let mut listed = Vec::new();
            for acted in last::runs(name, &read).into_iter().take(runs.most) {
                let finished = engine.run(&acted.run).await.ok().and_then(|run| run.finished);
                listed.push(json!({
                    "id": acted.run, "called": acted.what, "status": acted.status,
                    "started": acted.at, "finished": finished,
                }));
            }
            listed
        }
    };
    Ok(json!({ "runs": listed }))
}

/// Runs of one instance, as a part of runs lists them.
fn shown(info: &conductor::InstanceInfo, ran: Vec<conductor::RunInfo>) -> Vec<Value> {
    ran.into_iter()
        .map(|run| {
            json!({
                "id": run.id, "called": kernel::parts::run_called(&info.blueprint, &info.key, &run.flow),
                "status": run.status.as_str(), "started": run.started, "finished": run.finished,
            })
        })
        .collect()
}

/// The measurements a part of measures names, each by its name and what
/// it is of where its name says (`unit`), with its latest value beside
/// the median of its last few, a minute's values being one.
pub async fn measures(journal: &Journal, who: &Principal, measures: &Measures) -> Result<Value> {
    who.require("fleet:read")?;
    let now = journal.now_ms();
    let mut listed = Vec::new();
    for name in &measures.names {
        let series = journal
            .query_metrics(MetricQuery {
                name: name.clone(),
                labels: measures.labels.clone(),
                since_ms: now - MONTH_MS,
                until_ms: 0,
                group_by: Vec::new(),
                step_minutes: 1,
            })
            .await?;
        if series.is_empty() {
            listed.push(json!({ "name": name, "unit": blueprint::labels::unit(name), "measured": null }));
        }
        for one in series {
            let values: Vec<(i64, f64)> = one.points.iter().map(|(at, stats)| (*at, stats.avg)).collect();
            let measured = kernel::parts::measured(&values, measures.last).map(
                |came| json!({ "latest": came.latest, "at": came.at, "median": came.median, "of": came.of }),
            );
            listed.push(json!({
                "name": one.name, "unit": blueprint::labels::unit(&one.name), "measured": measured,
                "count": one.total.count,
            }));
        }
    }
    Ok(
        json!({ "measures": listed, "labels": measures.labels.iter().map(|(label, value)| (label.clone(), json!(value))).collect::<Map<String, Value>>() }),
    )
}
