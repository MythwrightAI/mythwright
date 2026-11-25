//! `project blueprints`: where a project keeps blueprints of its own.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::work::show;

/// Says, or changes, where a project keeps its own blueprints: in the
/// factory's place for them (`--usual`), there and in a folder of its
/// repository, or `--none`; or moves them in from its repository
/// (`--move-in`, `--from <folder>`).
pub async fn blueprints(client: &Client, args: &Args) -> Result<(), String> {
    let id = args.need(2, "which project")?;
    let from = args.value("from").map(str::to_string);
    let asked = match (args.word(3), args.has("none"), args.has("usual"), args.has("move-in")) {
        (None, false, false, false) if from.is_none() => None,
        (Some(folder), false, false, false) if from.is_none() => Some(json!({ "folder": folder })),
        (None, true, false, false) if from.is_none() => Some(json!({ "none": true })),
        (None, false, true, false) if from.is_none() => Some(json!({ "usual": true })),
        (None, false, false, true) => Some(json!({ "move_in": true, "from": from })),
        _ => {
            return Err(
                "say a folder of the project's repository, `--none`, `--usual`, or `--move-in` (with \
                        `--from <folder>` where they are not in the folder it says), and only one of them"
                    .into(),
            );
        }
    };
    let address = format!("/api/projects/{id}/blueprints");
    let answer = match asked {
        Some(body) => client.post(&address, body).await?,
        None => client.get(&address).await?,
    };
    show(client, &answer, || said(id, &answer));
    Ok(())
}

/// What a project keeps its own blueprints as, in a sentence.
fn said(id: &str, answer: &Value) -> String {
    let kept = &answer["blueprints"];
    let unchanged = if answer["changed"] == false { " already" } else { "" };
    if let Some(moved) = answer["moved"].as_array() {
        let files = if moved.len() == 1 { "1 file".to_string() } else { format!("{} files", moved.len()) };
        let mut lines = vec![format!(
            "`{id}` keeps its own blueprints where the factory keeps them now, and {files} moved there:"
        )];
        lines.extend(moved.iter().map(|one| {
            format!(
                "  {} -> {}",
                one["from"].as_str().unwrap_or_default(),
                one["to"].as_str().unwrap_or_default()
            )
        }));
        lines.push(format!("And {}.", answer["left"].as_str().unwrap_or_default()));
        return lines.join("\n");
    }
    match (kept["said"].as_str(), kept["folder"].as_str()) {
        (Some("none"), _) => format!(
            "`{id}`{unchanged} keeps no blueprints of its own: none stands in for what ships, and it has no \
             forks. Its fleet's is read as always"
        ),
        (Some("folder"), Some(folder)) => format!(
            "`{id}`{unchanged} keeps its own blueprints under `{folder}/` of its repository, read first, and where \
             the factory keeps them for it (`{}/`)",
            kept["written"].as_str().unwrap_or_default()
        ),
        (Some("usual"), _) => format!(
            "`{id}`{unchanged} keeps its own blueprints where the factory keeps them for it (`{}/`), as a \
             project does where it says nothing: no file of its repository is read as one",
            kept["written"].as_str().unwrap_or_default()
        ),
        _ => format!("`{id}`: {kept}"),
    }
}
