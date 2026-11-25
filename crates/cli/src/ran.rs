//! What has run, read from a terminal: the runs there are, one of them
//! step by step, and what its steps printed.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, now_ms, table};
use crate::work::{list, show, step_rows, text};

pub async fn runs(client: &Client, args: &Args) -> Result<(), String> {
    let instance = args.value("instance").map(|id| format!("instance={id}&")).unwrap_or_default();
    let found =
        client.get(&format!("/api/runs?{instance}limit={}", args.value("limit").unwrap_or("20"))).await?;
    show(client, &found, || {
        let now = now_ms();
        let rows: Vec<Vec<String>> = list(&found["runs"])
            .iter()
            .map(|run| {
                vec![
                    text(run, "id").into(),
                    text(run, "flow").into(),
                    text(run, "status").into(),
                    ago(now, run["started"].as_i64().unwrap_or(now)),
                    text(run, "error").into(),
                ]
            })
            .collect();
        if rows.is_empty() { "no runs yet".to_string() } else { table(&rows) }
    });
    Ok(())
}

pub async fn run(client: &Client, args: &Args) -> Result<(), String> {
    let found = client.get(&format!("/api/runs/{}", args.need(1, "the run's id")?)).await?;
    show(client, &found, || {
        let mut lines = vec![format!(
            "run {} {} {} (instance {})",
            text(&found, "id"),
            text(&found, "flow"),
            text(&found, "status"),
            text(&found, "instance")
        )];
        lines.push(step_rows(&found));
        if let Some(error) = found["error"].as_str() {
            lines.push(format!("error: {error}"));
        }
        lines.extend(kept_from(&found));
        lines.extend(let_go(&found));
        lines.join("\n")
    });
    Ok(())
}

/// What is said of a run whose steps' detail was let go.
fn let_go(run: &Value) -> Option<String> {
    run["let_go"].as_str().map(|said| format!("{}{}.", said[..1].to_uppercase(), &said[1..]))
}

/// What is said of a run something of which is kept from whoever asks.
fn kept_from(run: &Value) -> Option<String> {
    (run["kept_from"] == true).then(|| {
        "what set this run off is kept from you, and with it what its steps printed, what they gave and why \
         any failed"
            .to_string()
    })
}

/// What a step's log says, a line each: what it printed as it is, what it
/// printed as an error after `!`, and what the factory said of it after `#`.
fn log_lines(log: &Value) -> Vec<String> {
    list(&log["lines"])
        .iter()
        .map(|line| match text(line, "stream") {
            "e" => format!("! {}", text(line, "text")),
            "s" => format!("# {}", text(line, "text")),
            _ => text(line, "text").to_string(),
        })
        .collect()
}

/// One step's log; or, with no step named, the run step by step: each
/// step's log and what it gave. In a factory started with `--handed` a
/// step's log begins with what it was handed.
pub async fn logs(client: &Client, args: &Args) -> Result<(), String> {
    let (keep, folder) = (args.value("keep"), args.value("folder"));
    if keep.is_some() || folder.is_some() || args.word(1).is_none() {
        return kept(client, keep, folder).await;
    }
    let run = args.need(1, "the run's id")?;
    if let Some(step) = args.word(2) {
        let found = client.get(&format!("/api/runs/{run}/steps/{step}/log")).await?;
        show(client, &found, || log_lines(&found).join("\n"));
        return Ok(());
    }
    let found = client.get(&format!("/api/runs/{run}")).await?;
    let kept = kept_from(&found);
    let mut steps = Vec::new();
    for step in list(&found["steps"]) {
        let name = text(step, "step");
        // The log of a step of a run kept from whoever asks is not there for them.
        let log = match kept {
            Some(_) => json!({}),
            None => client.get(&format!("/api/runs/{run}/steps/{name}/log")).await?,
        };
        steps.push(
            json!({ "step": name, "status": step["status"], "log": log["lines"], "gave": step["output"] }),
        );
    }
    let whole = json!({ "run": run, "steps": steps });
    show(client, &whole, || {
        let mut lines = Vec::new();
        for step in &steps {
            lines.push(format!("{} ({})", text(step, "step"), text(step, "status")));
            lines.extend(log_lines(&json!({ "lines": step["log"] })).iter().map(|line| format!("  {line}")));
            if !step["gave"].is_null() {
                lines.push(format!("  gave: {}", step["gave"]));
            }
        }
        lines.extend(kept.clone());
        lines.extend(let_go(&found));
        if lines.is_empty() { "this run has no steps yet".to_string() } else { lines.join("\n") }
    });
    Ok(())
}

/// `logs --keep <days>|always --folder <folder>|usual`, either or both;
/// alone, how long runs' steps' detail is kept and where it is written.
async fn kept(client: &Client, keep: Option<&str>, folder: Option<&str>) -> Result<(), String> {
    let found = match (keep, folder) {
        (None, None) => client.get("/api/system/logs").await?,
        _ => {
            let mut asked = json!({});
            if let Some(keep) = keep {
                asked["days"] = match keep.parse::<u64>() {
                    Ok(days) => json!(days),
                    Err(_) if keep == "always" => json!("always"),
                    Err(_) => {
                        return Err(format!(
                            "`--keep` is a number of days, or `always`, and `{keep}` is neither"
                        ));
                    }
                };
            }
            if let Some(folder) = folder {
                asked["folder"] = json!(folder);
            }
            client.post("/api/system/logs", asked).await?
        }
    };
    show(client, &found, || {
        let how_long = match found["days"].as_u64() {
            Some(1) => {
                "What runs' steps printed, and what was said of each as it went, is kept for a day, and let \
                        go of after; what each run came to is kept for ever."
                    .to_string()
            }
            Some(days) => format!(
                "What runs' steps printed, and what was said of each as it went, is kept for {days} days, and let \
                 go of after; what each run came to is kept for ever."
            ),
            None => "What runs' steps printed is kept for ever.".to_string(),
        };
        let mut lines = vec![how_long];
        lines.push(match found["folder"].as_str() {
            Some(folder) => format!("It is written in {folder}."),
            None => format!("It is written in the data folder, {}.", text(&found, "usual")),
        });
        let before: Vec<&str> = list(&found["before"]).iter().filter_map(Value::as_str).collect();
        if !before.is_empty() {
            lines.push(format!("What was written before is read where it is: {}.", before.join(", ")));
        }
        if let Some(went) = found["let_go"].as_object() {
            let count = |name: &str| went.get(name).and_then(Value::as_u64).unwrap_or(0);
            lines.push(match (count("events"), count("files")) {
                (0, 0) => "Nothing was old enough to let go of now.".to_string(),
                (events, files) => {
                    format!("Let go of now: {events} events and {files} files of {} bytes.", count("bytes"))
                }
            });
        }
        if let (Some(by), Some(at)) = (found["by"].as_str(), found["at"].as_i64()) {
            lines.push(format!("said by {by}, {}", ago(now_ms(), at)));
        }
        lines.join("\n")
    });
    Ok(())
}
