//! `project environment`: a project's environments and their variables.
//!
//! What an environment is, is the daemon's (`help environments`); what is
//! here is the words a person types:
//!
//! ```text
//! project environment <project>                         its environments
//! project environment <project> <env>                   one, with its variables
//! project environment <project> <env> add|remove        made, or taken away
//! project environment <project> <env> set K=V… [--agents | --not]
//! project environment <project> <env> unset K…
//! project environment <project> <env> agents K… [--not]
//! project environment <project> <env> deploy <blueprint>|--none
//! project environment <project> <env> approve <who>…|--none
//! project environment <project> <env> branches <branch>…|--any
//! project environment <project> <env> run <branch>
//! project branches <project> [--fetch]
//! ```

use reqwest::Method;
use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

pub async fn run(client: &Client, args: &Args) -> Result<(), String> {
    let project = args.need(2, "the project")?;
    let base = format!("/api/projects/{}/environments", encode(project));
    let Some(name) = args.word(3) else {
        let listed = client.get(&base).await?;
        show(client, &listed, || {
            let rows: Vec<Vec<String>> = list(&listed["environments"])
                .iter()
                .map(|environment| {
                    let vars = environment["vars"].as_object().map(|vars| vars.len()).unwrap_or(0);
                    let agents = environment["vars"]
                        .as_object()
                        .map(|vars| vars.values().filter(|var| var["agents"] == true).count())
                        .unwrap_or(0);
                    vec![
                        text(environment, "name").to_string(),
                        format!("{vars} variable{}", if vars == 1 { "" } else { "s" }),
                        if agents == 0 { String::new() } else { format!("{agents} for agents") },
                        last_run(environment),
                    ]
                })
                .collect();
            if rows.is_empty() {
                format!(
                    "`{project}` has no environments yet; `{} project environment {project} staging add` makes one",
                    brand::CLI
                )
            } else {
                table(&rows)
            }
        });
        return Ok(());
    };
    let at = format!("{base}/{}", encode(name));
    let names = |from: usize| -> Result<Vec<String>, String> {
        let names = args.words_from(from);
        if names.is_empty() { Err("say which variables, by name".to_string()) } else { Ok(names) }
    };
    let answered = match args.word(4) {
        None => client.get(&at).await?,
        Some("add") => client.post(&base, json!({ "name": name })).await?,
        Some("remove") => {
            let removed = client.call(Method::DELETE, &at, None).await?;
            println!("{}", text(&removed, "said"));
            return Ok(());
        }
        Some("set") => {
            let agents = marked(args);
            let mut last = Value::Null;
            for pair in names(5)? {
                let (var, value) = pair.split_once('=').ok_or_else(|| {
                    format!("`{pair}` is not a variable and its value: write it as NAME=value")
                })?;
                // A value that reads as JSON is that (a number, true or
                // false), as a `--param` is; anything else is text.
                let value = serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
                let body = json!({ "value": value, "agents": agents });
                last = client.call(Method::PUT, &format!("{at}/vars/{}", encode(var)), Some(body)).await?;
            }
            last["environment"].clone()
        }
        Some("unset") => {
            let mut last = Value::Null;
            for var in names(5)? {
                last = client.call(Method::DELETE, &format!("{at}/vars/{}", encode(&var)), None).await?;
            }
            last["environment"].clone()
        }
        Some("agents") => {
            let agents = !args.has("not");
            let mut last = Value::Null;
            for var in names(5)? {
                let body = json!({ "agents": agents });
                last = client.call(Method::PUT, &format!("{at}/vars/{}", encode(&var)), Some(body)).await?;
            }
            last["environment"].clone()
        }
        Some("deploy") => {
            let none = args.has("none");
            let blueprint = match (args.word(5), none) {
                (Some(blueprint), false) => json!(blueprint),
                (None, true) => Value::Null,
                _ => return Err("say the blueprint it is loaded by, or `--none` for none".to_string()),
            };
            let body = json!({ "blueprint": blueprint });
            client.call(Method::PUT, &format!("{at}/deploy"), Some(body)).await?["environment"].clone()
        }
        Some("approve") => {
            let people = args.words_from(5);
            if people.is_empty() != args.has("none") {
                return Err("say who approves what goes under it (each by their name or email), or `--none` for nobody".to_string());
            }
            let body = json!({ "approve": people });
            client.call(Method::PUT, &format!("{at}/rules"), Some(body)).await?["environment"].clone()
        }
        Some("branches") => {
            let branches = args.words_from(5);
            if branches.is_empty() != args.has("any") {
                return Err(
                    "say which branches may be run under it (`main`, `release/*`), or `--any` for any"
                        .to_string(),
                );
            }
            let body = json!({ "branches": branches });
            client.call(Method::PUT, &format!("{at}/rules"), Some(body)).await?["environment"].clone()
        }
        Some("run") => {
            let branch = args.need(5, "the branch, such as dev or origin/dev")?;
            let ran = client.post(&format!("{at}/run"), json!({ "branch": branch })).await?;
            show(client, &ran, || {
                let last = &ran["environment"]["ran"];
                let commit: String = text(last, "commit").chars().take(10).collect();
                format!(
                    "`{name}` is being loaded from `{}` ({commit}) by `{}`: its run is {} (`{} logs {}`)",
                    text(last, "branch"),
                    text(last, "blueprint"),
                    text(last, "run"),
                    brand::CLI,
                    text(last, "run"),
                )
            });
            return Ok(());
        }
        Some(other) => {
            return Err(format!(
                "`{other}` is not something done to an environment: add, remove, set, unset, agents, deploy, \
                 approve, branches or run (`{} help environments`)",
                brand::CLI
            ));
        }
    };
    show(client, &answered, || said(&answered));
    Ok(())
}

/// Whether what is set is handed to agents: `--agents`, `--not`, or as it was.
fn marked(args: &Args) -> Value {
    match (args.has("agents"), args.has("not")) {
        (true, _) => json!(true),
        (false, true) => json!(false),
        _ => Value::Null,
    }
}

/// An environment as a terminal prints it: its name, then each variable.
fn said(environment: &Value) -> String {
    let mut lines = vec![format!("{} (made by {})", text(environment, "name"), text(environment, "by_name"))];
    if let Some(deploy) = environment["deploy"].as_str() {
        lines.push(format!("  loaded by {deploy}"));
    }
    let approve: Vec<&str> =
        environment["rules"]["approve"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    if !approve.is_empty() {
        lines.push(format!("  approved by {}", approve.join(", ")));
    }
    let branches: Vec<&str> =
        environment["rules"]["branches"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    if !branches.is_empty() {
        lines.push(format!("  runs {} alone", branches.join(", ")));
    }
    if environment["last"].is_object() {
        lines.push(format!("  last run: {}", last_run(environment)));
    }
    match environment["vars"].as_object().filter(|vars| !vars.is_empty()) {
        None => lines.push("  no variables".to_string()),
        Some(vars) => {
            let rows: Vec<Vec<String>> = vars
                .iter()
                .map(|(name, var)| {
                    let value = match &var["value"] {
                        Value::String(text) => text.clone(),
                        other => other.to_string(),
                    };
                    let agents = if var["agents"] == true { "for agents" } else { "" };
                    vec![format!("  {name}"), value, agents.to_string()]
                })
                .collect();
            lines.push(table(&rows));
        }
    }
    lines.join("\n")
}

/// What an environment last ran, as the daemon says it (`environments::last`),
/// with when beside it where it says when: the same words the page says.
fn last_run(environment: &Value) -> String {
    let last = &environment["last"];
    match last["at"].as_i64() {
        Some(at) => format!("{}, {}", text(last, "said"), crate::client::ago(crate::client::now_ms(), at)),
        None => text(last, "said").to_string(),
    }
}

/// `project branches <project> [--fetch]`: the branches of a project's
/// repository, local and as each remote holds them; with `--fetch` its
/// remotes are asked what they have now first, which is the network.
pub async fn branches(client: &Client, args: &Args) -> Result<(), String> {
    let fetch = args.has("fetch");
    let project = args.need(2, "the project")?;
    let base = format!("/api/projects/{}/branches", encode(project));
    if fetch {
        let fetched = client.post(&format!("{base}/fetch"), json!({})).await?;
        eprintln!("{}", text(&fetched, "said"));
    }
    let listed = client.get(&base).await?;
    show(client, &listed, || {
        if listed["repository"] == false {
            return text(&listed, "said").to_string();
        }
        let rows: Vec<Vec<String>> = list(&listed["branches"])
            .iter()
            .map(|branch| {
                let commit: String = text(branch, "commit").chars().take(10).collect();
                vec![text(branch, "name").to_string(), commit, text(branch, "subject").to_string()]
            })
            .collect();
        if rows.is_empty() {
            format!("`{project}` has no branch yet: nothing is committed")
        } else {
            table(&rows)
        }
    });
    Ok(())
}
