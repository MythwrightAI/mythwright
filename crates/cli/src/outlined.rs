//! `project outline`: how a project's things are arranged in the side
//! panel, said and changed from a terminal. A folder is named by its path
//! of names from the top (`Insights/Runs`), whatever their capitals, and a
//! thing by its key (`board`, `table:test-runs`, `library:docs`, `repo`),
//! which is also how a thing is shown as something else (`--name board
//! --to Roadmap`) while everything goes on knowing it by its key.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::work::show;

/// The id of the folder at `path` (names from the top, by `/`), where there is one.
fn folder_at(outline: &Value, path: &str) -> Option<String> {
    let mut items = outline["items"].as_array()?;
    let mut found = None;
    for name in path.split('/').map(str::trim).filter(|name| !name.is_empty()) {
        let folder = items.iter().find(|node| {
            node["folder"].is_string()
                && node["name"].as_str().is_some_and(|called| called.to_lowercase() == name.to_lowercase())
        })?;
        found = folder["folder"].as_str().map(str::to_string);
        items = folder["items"].as_array()?;
    }
    found
}

/// What a folder or a thing is said as in an operation: a folder by its id, a thing by its key.
fn node_of(outline: &Value, said: &str) -> String {
    folder_at(outline, said).unwrap_or_else(|| said.to_string())
}

fn folder_needed(outline: &Value, path: &str, id: &str) -> Result<String, String> {
    folder_at(outline, path).ok_or_else(|| {
        format!(
            "`{id}` has no folder `{path}` in its outline: `{} project outline {id}` says which it has",
            brand::CLI
        )
    })
}

/// Says, or changes, how a project's things are arranged.
pub async fn outline(client: &Client, args: &Args) -> Result<(), String> {
    let id = args.need(2, "which project")?.to_string();
    let things = args.words_from(3);
    let (making, putting, top, inside) =
        (args.value("folder"), args.has("put"), args.has("top"), args.value("in"));
    let (before, renaming, to) = (args.value("before"), args.value("rename"), args.value("to"));
    let (removing, forgetting, naming) = (args.value("remove"), args.has("forget"), args.value("name"));
    let alone = [renaming.is_some(), removing.is_some(), forgetting, naming.is_some()]
        .iter()
        .filter(|said| **said)
        .count();
    if alone > 1 || (alone == 1 && (making.is_some() || putting)) {
        return Err("say one of `--rename`, `--name`, `--remove` and `--forget` by itself".into());
    }
    if putting != !things.is_empty() {
        return Err("say what is put after `--put`: the keys of things (`table:test-runs`) or folders".into());
    }
    if top && inside.is_some() {
        return Err("say `--in <folder>` or `--top`, not both".into());
    }
    if (renaming.is_some() || naming.is_some()) != to.is_some() {
        return Err(
            "say `--rename <folder> --to <name>`, or `--name <thing> --to <name>` (`--to ''` to show it as it always was)"
                .into(),
        );
    }
    let address = format!("/api/projects/{id}/outline");
    let mut outline = client.get(&address).await?["outline"].clone();
    let mut said = Vec::new();
    if let Some(path) = making {
        // Each folder of the path that is not there is made, in the one before it.
        let mut into: Option<String> = None;
        let mut walked = String::new();
        for name in path.split('/').map(str::trim).filter(|name| !name.is_empty()) {
            walked = if walked.is_empty() { name.to_string() } else { format!("{walked}/{name}") };
            if let Some(there) = folder_at(&outline, &walked) {
                into = Some(there);
                continue;
            }
            let last = walked.len() == path.trim_matches('/').len();
            let before = if last && !putting { before.map(|before| node_of(&outline, before)) } else { None };
            let made = client
                .post(&address, json!({ "op": "make", "name": name, "into": into, "before": before }))
                .await?;
            into = made["done"]["made"].as_str().map(str::to_string);
            said.push(format!("made the folder `{walked}`"));
            outline = made["outline"].clone();
        }
    }
    if putting {
        let into = match inside {
            Some(path) => Some(folder_needed(&outline, path, &id)?),
            None => None,
        };
        let nodes: Vec<String> = things.iter().map(|said| node_of(&outline, said)).collect();
        let before = before.map(|before| node_of(&outline, before));
        let put = client
            .post(&address, json!({ "op": "put", "nodes": nodes, "into": into, "before": before }))
            .await?;
        said.push(format!(
            "put {} {}",
            things.join(", "),
            inside.map_or("at the top".to_string(), |path| format!("in `{path}`"))
        ));
        outline = put["outline"].clone();
    }
    if let (Some(path), Some(to)) = (renaming, to) {
        let folder = folder_needed(&outline, path, &id)?;
        outline =
            client.post(&address, json!({ "op": "rename", "folder": folder, "name": to })).await?["outline"]
                .clone();
        said.push(format!("called `{path}` {to}"));
    }
    if let (Some(thing), Some(to)) = (naming, to) {
        let named = client.post(&address, json!({ "op": "name", "thing": thing, "name": to })).await?;
        outline = named["outline"].clone();
        said.push(match to.trim() {
            "" => format!("showed `{thing}` as it always was"),
            to => format!("showed `{thing}` as {to}"),
        });
    }
    if let Some(path) = removing {
        let folder = folder_needed(&outline, path, &id)?;
        outline =
            client.post(&address, json!({ "op": "remove", "folder": folder })).await?["outline"].clone();
        said.push(format!("removed the folder `{path}`, what was in it where it was"));
    }
    if forgetting {
        outline = client.post(&address, json!({ "op": "forget" })).await?["outline"].clone();
        said.push("forgot how it was arranged".to_string());
    }
    let answer = json!({ "project": id, "outline": outline });
    show(client, &answer, || {
        let mut lines: Vec<String> = said.iter().map(|one| format!("`{id}`: {one}.")).collect();
        lines.push(drawn(&id, &outline));
        lines.join("\n")
    });
    Ok(())
}

/// The outline as lines, a folder's things under it.
fn drawn(id: &str, outline: &Value) -> String {
    fn lines(items: &Value, depth: usize, out: &mut Vec<String>) {
        for node in items.as_array().into_iter().flatten() {
            let pad = "  ".repeat(depth + 1);
            match (node["folder"].as_str(), node["thing"].as_str()) {
                (Some(folder), _) => {
                    out.push(format!("{pad}{}/  ({folder})", node["name"].as_str().unwrap_or_default()));
                    lines(&node["items"], depth + 1, out);
                }
                (_, Some(thing)) => match node["name"].as_str() {
                    Some(name) => out.push(format!("{pad}{thing}  (shown as {name})")),
                    None => out.push(format!("{pad}{thing}")),
                },
                _ => {}
            }
        }
    }
    if outline.is_null() {
        return format!("`{id}` is shown as usual in the side panel: nobody has arranged it.");
    }
    let mut out = vec![format!("{id}, as it is arranged:")];
    lines(&outline["items"], 0, &mut out);
    out.push("and after these, in the usual order, whatever of it nobody placed.".to_string());
    out.join("\n")
}
