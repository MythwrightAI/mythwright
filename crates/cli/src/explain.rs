//! `explain`: a blueprint said in plain words.
//!
//! What it is, what it makes, what it does when it is applied, what it
//! hears and then does, what it may do, and what it does as it ends: the
//! summary the daemon makes of a text (`blueprint::summary`), printed.
//! Nothing is worked out here. A file is read where it is and its text
//! sent, said to be the file it is where it is one of the project it is
//! asked for; anything else is a blueprint's name, found as an apply
//! would find it. Nothing is run and nothing is written, and the blueprint
//! need not be applied anywhere.
//!
//! One thing is added: where the summary says something a page does by
//! a press (fork it, go back to the default), the command that does it
//! from here is said after it ([`hints`]).

use blueprint::summary::{Known, Line, Part, Summary, plain, summary};
use serde_json::json;

use crate::args::Args;
use crate::client::Client;
use crate::work::{print_diagnostics, project_of};

pub async fn explain(client: &Client, args: &Args) -> Result<(), String> {
    let Some(named) = args.word(1) else {
        return Err(format!(
            "missing the blueprint to explain: a file, or a blueprint's name such as `std/board` \
             (`{} help explain`)",
            brand::CLI
        ));
    };
    let file = std::path::Path::new(named).is_file();
    let source = match file {
        true => Some(std::fs::read_to_string(named).map_err(|err| format!("cannot read {named}: {err}"))?),
        false => None,
    };
    let body = match &source {
        // A file of the project it is asked for is said to be that file,
        // so that a fork says what it was forked from.
        Some(source) => {
            let path = within(client, named, project_of(args).as_deref()).await;
            json!({ "source": source, "project": project_of(args), "path": path })
        }
        None => json!({ "name": named, "project": project_of(args) }),
    };
    let answer = match (client.post("/api/blueprints/summary", body).await, &source) {
        (Ok(answer), _) => answer,
        // No daemon to ask: what can be said of the file alone, which is
        // said to be less.
        (Err(error), Some(source)) if Client::is_unreachable(&error) => {
            let said = alone(named, source)?;
            println!("{}", plain(&said).trim_end());
            eprintln!(
                "note: {error}; this is said from the file alone, with no action known but those that are \
                 part of the language: a step that says nothing of itself is said by its action's name, and \
                 what the others make, run and take leave for is not said"
            );
            return Ok(());
        }
        (Err(error), _) => return Err(error),
    };
    if client.json {
        println!("{}", serde_json::to_string_pretty(&answer).unwrap_or_default());
        return if answer["ok"] == true { Ok(()) } else { Err(String::new()) };
    }
    if answer["ok"] != true {
        print_diagnostics(named, &answer["diagnostics"]);
        return Err(format!(
            "`{named}` does not check, so there is no summary of it: what is wrong with it is said above"
        ));
    }
    let said: Summary = serde_json::from_value(answer["summary"].clone())
        .map_err(|err| format!("the daemon's answer was not a summary: {err}"))?;
    println!("{}", plain(&said).trim_end());
    for hint in hints(&said.parts, &said.name, project_of(args).as_deref()) {
        println!("\n{hint}");
    }
    Ok(())
}

/// Where the file `named` is within `project`, as the daemon calls a
/// file of a project's: nothing where no project was said, the daemon
/// cannot be asked, or the file is not in the project's folder.
async fn within(client: &Client, named: &str, project: Option<&str>) -> Option<String> {
    let project = crate::client::encode(project?);
    let file = kernel::whole::whole(std::path::Path::new(named)).ok()?;
    let under = |folder: &str| {
        let folder = kernel::whole::whole(std::path::Path::new(folder)).ok()?;
        let names: Vec<String> = kernel::whole::under(&file, &folder)?
            .iter()
            .map(|name| name.to_string_lossy().into_owned())
            .collect();
        (!names.is_empty()).then(|| names.join("/"))
    };
    let found = client.get(&format!("/api/projects/{project}")).await.ok()?;
    if let Some(rest) = found["path"].as_str().and_then(under) {
        return Some(rest);
    }
    // Or where the factory keeps the project's own blueprints, which is named as `@blueprints/`.
    let kept = client.get(&format!("/api/projects/{project}/blueprints")).await.ok()?;
    kept["at"].as_str().and_then(under).map(|rest| format!("{}/{rest}", api::library::PLACE))
}

/// What the parts of a summary say that a page does by a press, as the
/// commands that do it from a terminal: forking a blueprint that ships,
/// and going back from a fork to the default. `name` is what the
/// blueprint is called, and `project` the project it was asked for,
/// where one was said.
pub fn hints(parts: &[Part], name: &str, project: Option<&str>) -> Vec<String> {
    fn all<'a>(lines: &'a [Line], found: &mut Vec<&'a Line>) {
        for line in lines {
            found.push(line);
            all(&line.under, found);
        }
    }
    let cli = brand::CLI;
    let mut lines = Vec::new();
    for part in parts {
        all(&part.lines, &mut lines);
    }
    let whose = project.unwrap_or("<project>");
    let mut said = Vec::new();
    for opens in lines.iter().filter_map(|line| line.opens.as_ref()) {
        match opens.what.as_str() {
            "forking" => said.push(format!(
                "To fork it: `{cli} own {name} --project {whose}`; `{cli} help own` says what follows from it."
            )),
            "back" => said.push(format!(
                "To go back to the default: `{cli} own {name} --project {whose} --back`, and `--back --force` \
                 where the fork's text says something else, which is then gone."
            )),
            _ => {}
        }
    }
    said
}

/// The summary of a file with nobody to ask: read against the language
/// alone, as `check` checks one where there is no daemon.
fn alone(file: &str, source: &str) -> Result<Summary, String> {
    let checked = blueprint::check(source, &blueprint::NoCatalog);
    if checked.has_errors() {
        println!("{}", blueprint::render_diagnostics(file, &checked.diagnostics));
        return Err(format!(
            "`{file}` does not read as a blueprint, so there is no summary of it: what is wrong with it is said \
             above"
        ));
    }
    let read = checked.blueprint.ok_or_else(|| format!("`{file}` does not read as a blueprint"))?;
    let known = Known { actions: &[], events: &[], scopes: kernel::principal::SCOPES };
    Ok(summary(&read, &known))
}
