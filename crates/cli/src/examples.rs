//! `example`: the examples that came with the program, and one put among
//! a project's own blueprints.
//!
//! Which there are and what each holds is the daemon's to say; where a
//! project's folder is on the device is the daemon's too, so nothing is
//! copied by hand.

use serde_json::json;

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

/// Names in a sentence: `a`, `a` and `b`, `a`, `b` and `c`.
fn named(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => quoted.join(""),
    }
}

/// `example list | example add <name> --project P`.
pub async fn example(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/examples").await?;
            show(client, &found, || {
                let mut lines = Vec::new();
                for one in list(&found) {
                    let blueprints: Vec<&str> =
                        list(&one["blueprints"]).iter().filter_map(|b| b.as_str()).collect();
                    lines.push(format!("{}  {}", text(one, "name"), text(one, "says")));
                    lines.push(format!("    its blueprints: {}", named(&blueprints)));
                }
                lines.push(format!(
                    "`{cli} example add <name> --project <project>` puts one among a project's own blueprints."
                ));
                lines.join("\n")
            });
            Ok(())
        }
        Some("add") => {
            let name = args.need(2, "the example to put there; `example list` says which there are")?;
            let project = crate::work::project_of(args).ok_or_else(|| {
                format!("say whose blueprints it is to be put among with `--project <project>`; `{cli} help example`")
            })?;
            let asked = json!({ "example": name });
            let done = client.post(&format!("/api/projects/{}/examples", encode(&project)), asked).await?;
            show(client, &done, || {
                let files =
                    |key: &str| -> Vec<&str> { list(&done[key]).iter().filter_map(|f| f.as_str()).collect() };
                let (written, there, blueprints) = (files("written"), files("there"), files("blueprints"));
                if written.is_empty() {
                    return format!(
                        "`{project}` has the example `{name}` already, as it came, in `{}`: nothing was written.",
                        text(&done, "folder")
                    );
                }
                let kept = match there.is_empty() {
                    true => String::new(),
                    false => format!(" {} was there already, as it came.", named(&there)),
                };
                format!(
                    "The example `{name}` is among `{project}`'s blueprints, in `{}`: {} written.{kept}\nIts \
                     blueprints are {}, applied by those names with `--project {project}`. They and what they \
                     link are the project's own files from here: read them and change them there, in the \
                     client under the project's files or in its folder.",
                    text(&done, "folder"),
                    named(&written),
                    named(&blueprints)
                )
            });
            Ok(())
        }
        Some(other) => {
            Err(format!("`example {other}` is not something `example` does; `{cli} help example`"))
        }
    }
}
