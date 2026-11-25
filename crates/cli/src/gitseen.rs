//! `project git <project> [--branch B] [--parts]`: a project's repository
//! as its Git page shows it, the branch its folder is on, what is changed,
//! the commits of a branch (`--branch`, as it is listed), its branches and
//! its remotes; and with `--parts`, each part an installed package says it
//! shows of a repository it serves, read now by the package's program,
//! which is the service it is for.

use serde_json::Value;

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

pub async fn run(client: &Client, args: &Args) -> Result<(), String> {
    let (branch, parts) = (args.value("branch"), args.has("parts"));
    let project = args.need(2, "the project")?;
    let base = format!("/api/projects/{}/git", encode(project));
    let asked = match &branch {
        Some(branch) => format!("{base}?branch={}", encode(branch)),
        None => base.clone(),
    };
    let seen = client.get(&asked).await?;
    let mut read = Vec::new();
    if parts {
        for offered in list(&seen["shows"]) {
            let package = text(offered, "package").to_string();
            for part in list(&offered["parts"]) {
                let name = text(part, "name").to_string();
                let way = format!("{base}/{}/{}", encode(&package), encode(&name));
                let came = match offered["refused"].as_str() {
                    Some(why) => Err(why.to_string()),
                    None => client.get(&way).await,
                };
                read.push((text(offered, "title").to_string(), part.clone(), came));
            }
        }
    }
    show(client, &seen, || said(&seen, &read));
    Ok(())
}

fn said(seen: &Value, read: &[(String, Value, Result<Value, String>)]) -> String {
    if seen["repository"] == false {
        return text(seen, "said").to_string();
    }
    let head = &seen["head"];
    let short = |commit: &str| commit.chars().take(10).collect::<String>();
    let mut lines = vec![match head["branch"].as_str() {
        Some(branch) => format!("on {branch}"),
        None => format!("on no branch, at {}", short(text(head, "commit"))),
    }];
    if let Some(upstream) = head["upstream"].as_str() {
        let (ahead, behind) = (head["ahead"].as_u64().unwrap_or(0), head["behind"].as_u64().unwrap_or(0));
        lines.push(format!("following {upstream}: {ahead} ahead, {behind} behind"));
    }
    let changes: Vec<Vec<String>> = list(&seen["changes"])
        .iter()
        .map(|change| {
            let what = [text(change, "staged"), text(change, "unstaged")];
            let what: Vec<&str> = what.into_iter().filter(|word| !word.is_empty()).collect();
            vec![text(change, "path").to_string(), what.join(", then ")]
        })
        .collect();
    lines.push(String::new());
    lines.push(match changes.is_empty() {
        true => "nothing is changed".to_string(),
        false => format!("changed:\n{}", table(&changes)),
    });
    if let Some(more) = seen["more"].as_u64().filter(|more| *more > 0) {
        lines.push(format!("and {more} more"));
    }
    let commits: Vec<Vec<String>> = list(&seen["commits"])
        .iter()
        .map(|one| vec![short(text(one, "commit")), text(one, "author").into(), text(one, "subject").into()])
        .collect();
    lines.push(String::new());
    lines.push(format!("commits of {}:", text(seen, "of")));
    lines.push(if commits.is_empty() { "none yet".to_string() } else { table(&commits) });
    let branches: Vec<String> =
        list(&seen["branches"]).iter().map(|one| text(one, "refname").to_string()).collect();
    lines.push(String::new());
    lines.push(format!(
        "branches: {}",
        if branches.is_empty() { "none".to_string() } else { branches.join(" ") }
    ));
    let remotes: Vec<Vec<String>> = list(&seen["remotes"])
        .iter()
        .map(|one| vec![text(one, "name").into(), text(one, "fetch").into()])
        .collect();
    lines.push(if remotes.is_empty() {
        "remotes: none".to_string()
    } else {
        format!("remotes:\n{}", table(&remotes))
    });
    for (title, part, came) in read {
        lines.push(String::new());
        lines.push(format!("{title}: {}", text(part, "title")));
        match came {
            Err(why) => lines.push(why.clone()),
            Ok(came) => {
                let columns = list(&part["columns"]);
                let mut rows =
                    vec![columns.iter().map(|column| text(column, "title").to_string()).collect::<Vec<_>>()];
                for row in list(&came["rows"]) {
                    let cell =
                        |value: &Value| value.as_str().map_or_else(|| value.to_string(), str::to_string);
                    let mut cells: Vec<String> =
                        columns.iter().map(|column| cell(&row[text(column, "field")])).collect();
                    if let Some(link) = part["link"].as_str().and_then(|link| row[link].as_str()) {
                        cells.push(link.to_string());
                    }
                    rows.push(cells);
                }
                lines.push(if rows.len() == 1 { "none".to_string() } else { table(&rows) });
            }
        }
    }
    lines.join("\n")
}
