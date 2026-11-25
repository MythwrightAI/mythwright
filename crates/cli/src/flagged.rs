//! `flag`: a project's feature flags, and what each says in each of its
//! environments (`help flag`). What a flag is, and when a change of one
//! waits for a go-ahead, is the daemon's; what is here is the words a
//! person types:
//!
//! ```text
//! flag list <project>                                      each flag and what it says in each environment
//! flag set <project> <flag> <environment> <value>           on, off, a percentage or a word
//! flag unset <project> <flag> <environment>                 its variable taken away there
//! flag ways <project> <flag> --environment E --blueprint B --per P --param F
//! ```

use reqwest::Method;
use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

pub async fn run(client: &Client, args: &Args) -> Result<(), String> {
    let what = args.need(1, "what to do: list, set, unset or ways")?;
    let project = args.need(2, "the project")?;
    let base = format!("/api/projects/{}/flags", encode(project));
    match what {
        "list" => {
            let listed = client.get(&base).await?;
            show(client, &listed, || said(project, &listed));
        }
        "set" => {
            let (flag, environment) = (args.need(3, "the flag")?, args.need(4, "the environment")?);
            let value = args.need(5, "what it is to say: on, off, a percentage or a word")?;
            let path = format!("{base}/{}/{}", encode(flag), encode(environment));
            let came = client.call(Method::PUT, &path, Some(json!({ "value": value }))).await?;
            show(client, &came, || match text(&came, "came") {
                "same" => format!("{flag} says that in {environment} already."),
                "asks" => format!("{}.", text(&came, "said")),
                _ => format!("{flag} is {} in {environment}.", text(&came, "shown")),
            });
        }
        "unset" => {
            let (flag, environment) = (args.need(3, "the flag")?, args.need(4, "the environment")?);
            let path = format!("{base}/{}/{}", encode(flag), encode(environment));
            let gone = client.call(Method::DELETE, &path, None).await?;
            show(client, &gone, || match gone["taken_away"] == true {
                true => format!("{flag} says nothing in {environment} now, which is off."),
                false => format!("{flag} said nothing in {environment} already."),
            });
        }
        "ways" => {
            let flag = args.need(3, "the flag")?;
            let asked = json!({
                "environment": args.value("--environment").unwrap_or_default(),
                "blueprint": args.value("--blueprint").unwrap_or_default(),
                "per": args.value("--per").unwrap_or_default(),
                "param": args.value("--param").unwrap_or_default(),
            });
            let started = client.post(&format!("{base}/{}/compare", encode(flag)), asked).await?;
            show(client, &started, || {
                let ways: Vec<String> = list(&started["ways"])
                    .iter()
                    .map(|way| format!("{} {}", text(way, "name"), way["share"]))
                    .collect();
                format!(
                    "Started `{}`, {} given {}: `{} compare show {}` says what came of each.",
                    text(&started, "name"),
                    text(&started, "of"),
                    ways.join(" and "),
                    brand::CLI,
                    text(&started, "name")
                )
            });
        }
        other => {
            return Err(format!("`flag {other}` is not a command; `{} help flag` says what is", brand::CLI));
        }
    }
    Ok(())
}

/// Each flag with what it says in each environment, as a table.
fn said(project: &str, listed: &Value) -> String {
    let environments: Vec<String> =
        list(&listed["environments"]).iter().filter_map(Value::as_str).map(str::to_string).collect();
    let flags = list(&listed["flags"]);
    if flags.is_empty() {
        return format!(
            "`{project}` has no flags yet; `{} record add {project} flags name=<name>` adds one",
            brand::CLI
        );
    }
    let mut rows = vec![
        ["flag", "kind"]
            .iter()
            .map(|head| head.to_string())
            .chain(environments.iter().cloned())
            .collect::<Vec<_>>(),
    ];
    for flag in flags {
        let mut row = vec![text(flag, "name").to_string(), text(flag, "kind").to_string()];
        row.extend(
            environments.iter().map(|environment| text(&flag["says"][environment], "shown").to_string()),
        );
        rows.push(row);
    }
    table(&rows)
}
