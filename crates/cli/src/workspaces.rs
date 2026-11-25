//! Workspaces from the command line: what gathers the projects, and the
//! channels, that belong together. Grouping and nothing else. A factory
//! always has one and every project is in one, which the daemon holds:
//! what would leave a project in none is asked here as it is written and
//! refused there, in a sentence that says what to do instead.

use reqwest::Method;
use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

pub async fn workspace(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/workspaces").await?;
            // A channel is shown by what it is called, where whoever asks
            // may read channels; by its id where they may not.
            let channels = client.get("/api/channels").await.unwrap_or(Value::Null);
            let called = |id: &str| {
                let named = list(&channels["channels"]).iter().find(|channel| text(channel, "id") == id);
                named.map_or_else(|| id.to_string(), |channel| format!("#{}", text(channel, "name")))
            };
            show(client, &found, || {
                let rows: Vec<Vec<String>> = list(&found["workspaces"])
                    .iter()
                    .map(|one| {
                        let of = |key: &str| -> Vec<String> {
                            list(&one[key]).iter().filter_map(Value::as_str).map(str::to_string).collect()
                        };
                        let projects = of("projects");
                        let channels: Vec<String> = of("channels").iter().map(|id| called(id)).collect();
                        // The usual one is where a project goes when nobody says.
                        let usual = if one["usual"] == true { " (the usual one)" } else { "" };
                        vec![
                            text(one, "id").to_string(),
                            format!("{}{usual}", text(one, "name")),
                            if projects.is_empty() { "no projects".into() } else { projects.join(", ") },
                            channels.join(", "),
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    // Only while nobody owns the factory: its first is made as they register.
                    "no workspace yet: the first is made when the factory's owner registers".to_string()
                } else {
                    table(&rows)
                }
            });
        }
        Some("create") => {
            let name = args.rest(2);
            if name.is_empty() {
                return Err("missing the workspace's name".into());
            }
            let made = client.post("/api/workspaces", json!({ "name": name })).await?;
            show(client, &made, || {
                format!(
                    "made the workspace {} ({}), with nothing in it yet: `{cli} workspace put {} <project>`",
                    text(&made, "name"),
                    text(&made, "id"),
                    text(&made, "id"),
                    cli = brand::CLI
                )
            });
        }
        Some("rename") => {
            let which = args.need(2, "the workspace to call something else")?;
            let name = args.rest(3);
            if name.is_empty() {
                return Err("missing what it is to be called".into());
            }
            let path = format!("/api/workspaces/{}", encode(which));
            let now = client.call(Method::PATCH, &path, Some(json!({ "name": name }))).await?;
            show(client, &now, || {
                format!("{} is now called {}; its id is as it was", text(&now, "id"), text(&now, "name"))
            });
        }
        Some("remove") => {
            let which = args.need(2, "the workspace to take away")?;
            // Where what is in it goes, which one that holds anything is not taken away without.
            let to = args.value("to").map(|to| json!({ "to": to }));
            let gone = client.call(Method::DELETE, &format!("/api/workspaces/{}", encode(which)), to).await?;
            show(client, &gone, || {
                let (projects, channels) = (list(&gone["projects"]).len(), list(&gone["channels"]).len());
                match gone["to"].as_str() {
                    Some(to) if projects + channels > 0 => format!(
                        "took the workspace {} away. Nothing that was in it is removed: its {projects} project(s) and {channels} channel(s) are in {to} now",
                        text(&gone, "name"),
                    ),
                    _ => format!("took the workspace {} away, which had nothing in it", text(&gone, "name")),
                }
            });
        }
        Some("put") => {
            let which = args.need(2, "the workspace")?;
            let project = args.need(3, "the project to put in it")?;
            let path = format!("/api/projects/{}/workspace", encode(project));
            let placed = client.call(Method::PUT, &path, Some(json!({ "workspace": which }))).await?;
            show(client, &placed, || {
                format!(
                    "{} is in the workspace {}; nothing else of it changed",
                    text(&placed, "project"),
                    text(&placed, "workspace")
                )
            });
        }
        Some("take") => {
            // A project is always in a workspace: asked as it is written,
            // the daemon refuses, saying to put it in another.
            let project = args.need(2, "the project to take out of its workspace")?;
            let path = format!("/api/projects/{}/workspace", encode(project));
            let placed = client.call(Method::PUT, &path, Some(json!({ "workspace": null }))).await?;
            show(client, &placed, || {
                format!("{} is in {}", text(&placed, "project"), text(&placed, "workspace"))
            });
        }
        Some(other) => {
            return Err(format!(
                "`workspace {other}` is not a command; try list, create, rename, remove, put or take"
            ));
        }
    }
    Ok(())
}

/// `channel belongs <name> --project P | --workspace W`: says what a
/// channel belongs to, or changes it. `--nothing` is still taken and
/// asked as it is written: the daemon refuses it, saying that a channel
/// is always a project's or a workspace's.
pub async fn belongs(client: &Client, args: &Args) -> Result<(), String> {
    let name = args.need(2, "the channel")?.trim_start_matches('#');
    let said = [args.value("project").is_some(), args.value("workspace").is_some(), args.has("nothing")];
    if said.iter().filter(|one| **one).count() != 1 {
        return Err("say what the channel belongs to, and one thing: `--project P` or `--workspace W`".into());
    }
    let body = json!({ "project": args.value("project"), "workspace": args.value("workspace") });
    let path = format!("/api/channels/{}/belongs", encode(name));
    let now = client.call(Method::PUT, &path, Some(body)).await?;
    show(client, &now, || {
        let to = match (now["project"].as_str(), now["workspace"].as_str()) {
            (Some(project), _) => format!("belongs to the project {project}"),
            (_, Some(workspace)) => format!("belongs to the workspace {workspace}"),
            _ => "belongs to nothing".to_string(),
        };
        format!("#{} {to}", text(&now, "name"))
    });
    Ok(())
}
