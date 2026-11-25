//! Extension packages: what runs agents, beyond what is built in, and the
//! actions and blueprints that come with somebody else's product.

use std::path::Path;

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, table};
use crate::work::{list, plain, show, text};

/// The manifest's name inside a package's folder.
const MANIFEST: &str = "extension.yaml";

/// The blueprints a package brings, by name.
fn brought(extension: &Value) -> Vec<&str> {
    extension["blueprints"].as_array().into_iter().flatten().filter_map(Value::as_str).collect()
}

/// The actions a package brings, each as it is described everywhere.
fn actions(extension: &Value) -> impl Iterator<Item = &Value> {
    extension["actions"].as_array().into_iter().flatten()
}

/// One action in a line: its name, what it takes (in brackets what may be
/// left out) and gives out, and what it does.
fn action_row(action: &Value) -> Vec<String> {
    let takes: Vec<String> = action["args"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|arg| {
            let name = text(arg, "name");
            if arg["required"] == true { name.to_string() } else { format!("[{name}]") }
        })
        .collect();
    let gives: Vec<&str> =
        action["outputs"].as_array().into_iter().flatten().filter_map(|output| output[0].as_str()).collect();
    let mut shape = takes.join(", ");
    if !gives.is_empty() {
        shape = format!("{shape} -> {}", gives.join(", ")).trim_start().to_string();
    }
    vec![format!("  {}", text(action, "name")), shape, text(action, "summary").to_string()]
}

/// The installed packages a listing says are not the ones of their names
/// that came with this installation.
fn behind(found: &Value) -> Vec<&str> {
    found["extensions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|extension| extension["behind"] == true)
        .map(|extension| text(extension, "name"))
        .collect()
}

/// What taking the package that came with this installation came to, as
/// the daemon answered: whether anything was replaced, and where the one
/// that came differed from the copy.
fn taken_in_words(taken: &Value) -> String {
    let name = text(taken, "name");
    if taken["updated"] != true {
        return format!("`{name}` is already the one that came with this installation");
    }
    let mut said = format!(
        "`{name}` is now the one that came with this installation. It replaced the copy that was installed, \
         whatever was changed in that; the copy is kept beside it (`extension.was.yaml`, in the package's \
         folder under the data folder's `extensions`)."
    );
    let differs = text(&taken["differs"], "words");
    if !differs.is_empty() {
        said.push_str(&format!("\nAgainst that copy, the one that came {differs}."));
    }
    if taken["available"] != true && !text(taken, "program").is_empty() {
        said.push_str(&format!(
            "\nnote: its program, `{}`, is not installed on the daemon's device",
            text(taken, "program")
        ));
    }
    said
}

pub async fn extension(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/extensions").await?;
            show(client, &found, || {
                let rows: Vec<Vec<String>> = found["extensions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|extension| {
                        let here = if extension["available"] == true {
                            "ready".to_string()
                        } else {
                            format!("`{}` is not installed here", text(extension, "program"))
                        };
                        // What it is for: agents run on it, or it brings
                        // blueprints. Its actions are many, and follow.
                        let mut brings = brought(extension).join(", ");
                        if extension["agent"] == true {
                            brings =
                                if brings.is_empty() { "agents".into() } else { format!("agents, {brings}") };
                        }
                        // Installed as a copy, it says what it said then.
                        let behind =
                            if extension["behind"] == true { "not the one that came with this" } else { "" };
                        vec![
                            text(extension, "name").to_string(),
                            text(extension, "title").to_string(),
                            here,
                            brings,
                            behind.to_string(),
                        ]
                    })
                    .collect();
                let offered: Vec<&str> = found["offered"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|extension| text(extension, "name"))
                    .collect();
                let mut said =
                    if rows.is_empty() { "no extensions are installed".to_string() } else { table(&rows) };
                let done: Vec<Vec<String>> = found["extensions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(actions)
                    .map(action_row)
                    .collect();
                if !done.is_empty() {
                    said.push_str(&format!(
                        "\nactions they bring, for a blueprint to `do` (`{} actions <name>` says more):\n{}",
                        brand::CLI,
                        table(&done)
                    ));
                }
                let behind = behind(&found);
                if !behind.is_empty() {
                    said.push_str(&format!(
                        "\nnot the ones that came with this installation: {}. Each is a copy made when it was installed, and \
                         says what its package said then; `{cli} extension update <name>` (or `--all`) takes the one that \
                         came with this in its place, whatever was changed in the copy, and keeps the copy beside it.",
                        behind.join(", "),
                        cli = brand::CLI
                    ));
                    // Where each differs, which is all that is known of it.
                    for extension in list(&found["extensions"]) {
                        let differs = text(&extension["differs"], "words");
                        if !differs.is_empty() {
                            said.push_str(&format!(
                                "\n  {}: the one that came {differs}",
                                text(extension, "name")
                            ));
                        }
                    }
                }
                if !offered.is_empty() {
                    said.push_str(&format!(
                        "\ncame with this installation, not installed: {}  (`{} extension add <name>`)",
                        offered.join(", "),
                        brand::CLI
                    ));
                } else if rows.is_empty() {
                    said.push_str(&format!(
                        "; `{} extension add <folder>` adds one (`{} help extensions`)",
                        brand::CLI,
                        brand::CLI
                    ));
                }
                said
            });
        }
        Some("add") => {
            let wanted =
                args.need(2, "the package's folder, or the name of one that came with the product")?;
            let given = Path::new(wanted);
            let manifest = if given.is_dir() { given.join(MANIFEST) } else { given.to_path_buf() };
            let body = match std::fs::read_to_string(&manifest) {
                Ok(source) => json!({ "source": source }),
                // Not a file here: one of those the daemon has beside it.
                Err(_) if !wanted.contains(['/', '\\', '.']) => json!({ "name": wanted }),
                Err(err) => {
                    return Err(format!(
                        "cannot read {}: {err}. A package is a folder with an `{MANIFEST}` in it.",
                        manifest.display()
                    ));
                }
            };
            let added = client.post("/api/extensions", body).await?;
            show(client, &added, || {
                let name = text(&added, "name");
                let mut said = format!("`{name}` is installed");
                if added["agent"] == true {
                    said.push_str(&format!(
                        "; agents can run on it: `{} agent add <name> --adapter {name}`",
                        brand::CLI
                    ));
                }
                let done: Vec<&str> = actions(&added).map(|action| text(action, "name")).collect();
                if let Some(first) = done.first() {
                    said.push_str(&format!(
                        "\nit brings actions for a blueprint to `do`: {} (`{} actions {first}`)",
                        done.join(", "),
                        brand::CLI
                    ));
                }
                for blueprint in brought(&added) {
                    said.push_str(&format!("\nit brings `{blueprint}`: `{} apply {blueprint}`", brand::CLI));
                }
                if added["available"] != true {
                    said.push_str(&format!(
                        "\nnote: its program, `{}`, is not installed on the daemon's device yet",
                        text(&added, "program")
                    ));
                    if !text(&added, "install").is_empty() {
                        said.push_str(&format!(". {}", text(&added, "install")));
                    }
                }
                said
            });
        }
        Some("update") => {
            let names: Vec<String> = if args.has("all") {
                let found = client.get("/api/extensions").await?;
                behind(&found).into_iter().map(str::to_string).collect()
            } else {
                let wanted = args.need(
                    2,
                    "the package's name, or `--all` for every installed package that is not the one that came \
                     with this installation",
                )?;
                vec![wanted.to_string()]
            };
            // Each is said as it is taken: one that is refused stops there,
            // with those before it taken and said.
            let mut all = Vec::new();
            for name in names {
                let taken = client.post(&format!("/api/extensions/{name}/update"), json!({})).await?;
                if !args.has("all") {
                    show(client, &taken, || taken_in_words(&taken));
                } else if !client.json {
                    println!("{}", plain(&taken_in_words(&taken)));
                }
                all.push(taken);
            }
            if args.has("all") {
                // Asked for as JSON, all of them are one list, which may hold nothing.
                show(client, &Value::Array(all.clone()), || match all.is_empty() {
                    true => {
                        "every installed package is already the one that came with this installation".into()
                    }
                    false => String::new(),
                });
            }
        }
        Some("remove") => {
            let name = args.need(2, "the extension's name")?;
            client.call(reqwest::Method::DELETE, &format!("/api/extensions/{name}"), None).await?;
            println!(
                "`{name}` is removed; agents that ran on it will say so when next asked to work, and what was applied from it runs on as it was"
            );
        }
        Some("models") => {
            let name = args.need(2, "what runs the agent, e.g. an extension's name")?;
            // Asked again, the daemon runs the program: that is no read.
            let path = format!("/api/providers/{name}/models");
            let found = match args.has("fresh") {
                true => client.post(&path, serde_json::json!({})).await?,
                false => client.get(&path).await?,
            };
            show(client, &found, || {
                let rows: Vec<Vec<String>> = found["models"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|model: &Value| {
                        let usual = if model["default"] == true { "  (its default)" } else { "" };
                        vec![
                            text(model, "id").to_string(),
                            format!("{}{usual}", text(model, "label")),
                            text(model, "note").to_string(),
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    format!("`{name}` does not say which models it offers; name one with --model")
                } else {
                    table(&rows)
                }
            });
        }
        Some(other) => {
            return Err(format!(
                "`extension {other}` is not a command; there are list, add, update, remove and models"
            ));
        }
    }
    Ok(())
}
