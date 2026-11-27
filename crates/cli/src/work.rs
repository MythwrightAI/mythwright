//! Commands about the daemon itself, signing in, projects, blueprints,
//! instances and runs.

use std::io::{IsTerminal, Write};

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, encode, now_ms, table, took};

/// Prints `value` as JSON when asked for it, else the summary.
pub fn show(client: &Client, value: &Value, summary: impl FnOnce() -> String) {
    if client.json {
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
    } else {
        let text = plain(&summary());
        if !text.is_empty() {
            println!("{text}");
        }
    }
}

/// Text as it is safe to print where somebody reads it: what a terminal
/// would obey and not show (an escape, a bell, a carriage return) is shown
/// as a mark. Much of what is printed was written by an agent or a
/// program, and a terminal does what such characters say: moves the
/// cursor, retitles the window, rewrites what was printed above.
pub fn plain(text: &str) -> String {
    let obeyed = |c: char| c.is_control() && c != '\n' && c != '\t';
    if !text.chars().any(obeyed) {
        return text.to_string();
    }
    text.chars().map(|c| if obeyed(c) { '\u{fffd}' } else { c }).collect()
}

pub fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

pub fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// Asks for a line of input. A secret is not echoed when typed at a
/// terminal that can be told so, which is Unix's (`stty`); elsewhere it
/// is shown as it is typed.
fn ask(label: &str, secret: bool) -> Result<String, String> {
    eprint!("{label}: ");
    let _ = std::io::stderr().flush();
    let hide = secret && cfg!(unix) && std::io::stdin().is_terminal();
    if hide {
        let _ = std::process::Command::new("stty").arg("-echo").status();
    }
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    if hide {
        let _ = std::process::Command::new("stty").arg("echo").status();
        eprintln!();
    }
    read.map_err(|err| format!("could not read {label}: {err}"))?;
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

pub(crate) fn given_or_ask(args: &Args, flag: &str, label: &str, secret: bool) -> Result<String, String> {
    match args.value(flag) {
        Some(value) => Ok(value.to_string()),
        None => ask(label, secret),
    }
}

/// Stops the daemon, however it was started: by `up`, or by the desktop app
/// on someone's behalf.
pub async fn down(client: &Client, args: &Args) -> Result<(), String> {
    let at = json!({ "at": args.value("at"), "of": args.value("of") });
    let (drain, within) = (args.has("drain"), args.value("within"));
    if args.value("of").is_some() && args.value("at").is_none() {
        return Err("`--of` says which blueprint the step of `--at` is a step of: name the step too".into());
    }
    match (drain, within.is_some(), args.value("at").is_some()) {
        (true, _, true) => {
            return Err("`--drain` stops once what is in hand is done, and `--at` at a step: say one".into());
        }
        (false, true, _) => {
            return Err("`--within` says how long `--drain` waits at most: say `--drain` too".into());
        }
        (true, ..) => return crate::drained::down(client, within).await,
        _ => {}
    }
    match client.post("/api/system/stop", at).await {
        Ok(answer) => {
            show(client, &answer, || match answer["at"].as_str() {
                Some(step) => format!(
                    "the daemon will stop as a step called `{step}`{} begins, keeping nothing said after that; \
                     `{} up` then goes on from there",
                    answer["of"].as_str().map(|of| format!(" of {of}")).unwrap_or_default(),
                    brand::CLI
                ),
                None => format!(
                    "the daemon is stopping; what it runs is left running, and `{} up` picks it up again",
                    brand::CLI
                ),
            });
            Ok(())
        }
        // Already what was wanted.
        Err(error) if error.starts_with("nothing is answering") => {
            println!("nothing is running at {}", client.url);
            Ok(())
        }
        Err(error) => Err(error),
    }
}

/// Which program a daemon says it is, in words: a copy on its own, or one
/// run from the checkout it was built in. Nothing for a daemon that does
/// not say.
fn runs_from(meta: &Value) -> Option<&'static str> {
    match meta["runs_from"].as_str() {
        Some("installed") => Some("A copy on its own: it changes only when it is replaced."),
        Some("checkout") => Some("Run from its checkout: what is built or edited there is what this runs."),
        _ => None,
    }
}

/// What a daemon still taking up what it was doing as it started says of
/// how far it has come (`api::resuming`): nothing of one that is not.
fn taking_up(meta: &Value) -> Option<String> {
    meta["taking_up"]["says"].as_str().map(str::to_string)
}

pub async fn status(client: &Client) -> Result<(), String> {
    let meta = client.get("/api/meta").await?;
    // Which factory this is, said first, for the ones that have a name:
    // the same commands do the same things to whichever one answers.
    let named = meta["profile"].as_str().or(meta["environment"].as_str());
    let (environment, to) = match named {
        Some(name) => (format!(" ({name})"), format!("{} --profile {name}", brand::CLI)),
        None => (String::new(), brand::CLI.to_string()),
    };
    if meta["needs_setup"] == true {
        show(client, &meta, || {
            format!(
                "{} {}{environment} is running on {} and has no owner yet.\n{}Make the first account with `{to} register`, or open {}.",
                text(&meta, "name"),
                text(&meta, "version"),
                text(&meta, "device"),
                runs_from(&meta).map(|said| format!("{said}\n")).unwrap_or_default(),
                client.url
            )
        });
        return Ok(());
    }
    if !client.signed_in() {
        show(client, &meta, || {
            format!(
                "{}{environment} is running on {}. Sign in with `{to} login`.{}",
                text(&meta, "name"),
                text(&meta, "device"),
                taking_up(&meta).map(|said| format!("\n{said}")).unwrap_or_default(),
            )
        });
        return Ok(());
    }
    let fleet = client.get("/api/fleet").await?;
    // What every page says across its top (a copy of another factory), and a daemon draining to stop, first.
    let draining = crate::drained::first(client, &meta).await;
    // A project's own file under the name of what ships that is not what runs there.
    let own = client.get("/api/blueprints/stands").await.unwrap_or_default();
    show(client, &fleet, || {
        let resources = &fleet["device"]["resources"];
        let mut lines = vec![format!(
            "{} {}{environment} on {} ({} CPUs, {} MB free of {} MB, load {})",
            text(&meta, "name"),
            text(&meta, "version"),
            text(&fleet["device"], "name"),
            resources["cpus"],
            resources["free_mem_mb"],
            resources["total_mem_mb"],
            resources["load1"].as_f64().map_or("unknown".to_string(), |load| format!("{load:.1}")),
        )];
        lines.extend(runs_from(&meta).map(str::to_string));
        // A daemon that has just started, still taking up what it was doing.
        lines.extend(taking_up(&meta));
        lines.extend(draining.clone());
        // What another installation sees this one prove, when the two link.
        if let Some(id) = fleet["device"]["id"].as_str() {
            lines.push(format!("To a device or a hub it links with, it is {id}."));
        }
        let count = |key: &str| list(&fleet[key]).len();
        lines.push(format!(
            "{} live instances, {} services, {} routers, {} agents",
            count("instances"),
            count("services"),
            count("routers"),
            list(&fleet["agents"]).iter().filter(|agent| agent["active"] == true).count()
        ));
        // Whether an agent's program is kept to its work here, and which are not.
        let walls = &fleet["device"]["walls"];
        let open: Vec<&str> = list(&fleet["agents"])
            .iter()
            // The built-in assistant has no program to keep in, and
            // one the factory does not start has none here.
            .filter(|agent| agent["active"] == true && agent["wall"] != true)
            .filter(|agent| agent["adapter"] != "builtin" && agent["adapter"] != agents::EXTERNAL)
            .map(|agent| text(agent, "name"))
            .collect();
        if walls["open"] == true {
            lines.push(
                "It was started holding no walls: every agent's program sees this device as whoever runs the daemon does."
                    .to_string(),
            );
        } else if let Some(why) = walls["why"].as_str() {
            lines.push(format!(
                "No wall can be held here: {why}. An agent that has one is not started (`{} help wall`).",
                brand::CLI
            ));
        } else if walls["held"] == true && !open.is_empty() {
            lines.push(format!(
                "Walls are held here, and {} with none: {} (`{} help wall`).",
                if open.len() == 1 { "1 agent works" } else { "some agents work" },
                open.join(", "),
                brand::CLI
            ));
        } else if walls["held"] == true {
            lines.push(
                "Walls are held here: an agent's program is shown its work and little else.".to_string(),
            );
        }
        let behind: Vec<&str> =
            list(&fleet["device"]["packages_behind"]).iter().filter_map(Value::as_str).collect();
        if !behind.is_empty() {
            lines.push(format!(
                "Installed packages that are not the ones that came with this installation: {} (`{} extension list`).",
                behind.join(", "),
                brand::CLI
            ));
        }
        for service in list(&fleet["services"]) {
            lines.push(format!(
                "  service {}  {}  port {}",
                text(service, "name"),
                text(service, "status"),
                service["port"]
            ));
        }
        for file in list(&own["own"]).iter().filter(|file| file["in_use"] == false) {
            lines.push(format!("{}: {}", text(file, "project"), text(file, "said")));
        }
        lines.join("\n")
    });
    Ok(())
}

pub async fn logout(client: &Client, args: &Args) -> Result<(), String> {
    // Everywhere: every session of whoever this is, wherever it was begun.
    // That is asked as whoever is signed in, and so it can be refused.
    if args.has("everywhere") {
        let ended = client.post("/api/auth/logout", json!({ "everywhere": true })).await?;
        client.forget_session();
        let keys = match ended["keys"].as_u64().unwrap_or(0) {
            0 => String::new(),
            many => format!(
                "\n{many} of your keys still work: a key is no session. `{0} keys` lists them, and `{0} key revoke <id>` withdraws one.",
                brand::CLI
            ),
        };
        println!("Signed out everywhere: {} sessions ended.{keys}", ended["sessions"].as_u64().unwrap_or(0));
        return Ok(());
    }
    let _ = client.post("/api/auth/logout", json!({})).await;
    client.forget_session();
    println!("Signed out.");
    Ok(())
}

pub async fn whoami(client: &Client) -> Result<(), String> {
    let me = client.get("/api/me").await?;
    show(client, &me, || {
        let scopes: Vec<&str> = list(&me["principal"]["scopes"]).iter().filter_map(Value::as_str).collect();
        format!("{} ({})\nmay: {}", text(&me["principal"], "name"), text(&me, "actor"), scopes.join(", "))
    });
    Ok(())
}

pub async fn project(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let projects = client.get("/api/projects").await?;
            show(client, &projects, || {
                let rows: Vec<Vec<String>> = list(&projects["projects"])
                    .iter()
                    .map(|project| {
                        vec![
                            text(project, "id").into(),
                            text(project, "name").into(),
                            text(project, "path").into(),
                            // Said only of one that is not: a folder taken
                            // up as it stood, whose own files do not stand
                            // in for the blueprints that ship.
                            if project["trusted"] == false { "not trusted" } else { "" }.into(),
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    format!("no projects yet; `{} project create <name>`", brand::CLI)
                } else {
                    table(&rows)
                }
            });
        }
        Some("environment" | "environments") => return crate::environs::run(client, args).await,
        Some("branches") => return crate::environs::branches(client, args).await,
        Some("git") => return crate::gitseen::run(client, args).await,
        Some("create") => {
            // A setup the factory keeps, by its name, in the place of its default.
            let setup = args.value("setup");
            let name = args.rest(2);
            if name.is_empty() {
                return Err("missing the project's name".into());
            }
            // A project is added with what one starts with unless it is to
            // be bare, or to be without some of it. What there is to start
            // with is the daemon's to say, so nothing here names any of it.
            let bare = args.has("bare");
            let without = args.values("without");
            // And with what it is not given unless said.
            let with = args.values("with");
            let (path, workspace) = (args.value("path"), args.value("workspace"));
            let set_up = match (bare, without.is_empty() && with.is_empty() && setup.is_none()) {
                (true, true) => json!(false),
                (true, false) => {
                    return Err(
                        "`--bare` is the project alone, with none of what one starts with: say one or the other"
                            .into(),
                    );
                }
                (false, true) => json!(true),
                (false, false) if without.is_empty() && with.is_empty() => json!({ "setup": setup }),
                (false, false) => {
                    let read = match &setup {
                        Some(setup) => {
                            client.get(&format!("/api/setups/{}", encode(setup))).await?["rows"].clone()
                        }
                        None => client.get("/api/meta").await?["project_starts"].clone(),
                    };
                    let starts = list(&read);
                    let known: Vec<&str> = starts.iter().map(|start| text(start, "name")).collect();
                    if let Some(unknown) = without.iter().find(|name| !known.contains(name)) {
                        return Err(format!(
                            "`{unknown}` is nothing a project starts with, so there is nothing to leave out. It may be without: {}",
                            known.join(", ")
                        ));
                    }
                    if let Some(unknown) = with.iter().find(|name| !known.contains(name)) {
                        return Err(format!(
                            "`{unknown}` is nothing a project starts with. It may be with: {}",
                            known.join(", ")
                        ));
                    }
                    let kept: Vec<&str> = starts
                        .iter()
                        .filter(|start| start["on"] == true || with.contains(&text(start, "name")))
                        .map(|start| text(start, "name"))
                        .filter(|name| !without.contains(name))
                        .collect();
                    match setup {
                        Some(setup) => json!({ "setup": setup, "rows": kept }),
                        None => json!(kept),
                    }
                }
            };
            let body = json!({
                "name": name, "path": path, "set_up": set_up, "workspace": workspace,
            });
            let project = client.post("/api/projects", body).await?;
            let tree = client.get(&format!("/api/projects/{}/tree", text(&project, "id"))).await?;
            show(client, &project, || {
                let files: Vec<&str> = list(&tree["entries"])
                    .iter()
                    .filter(|e| e["dir"] == false)
                    .map(|e| text(e, "path"))
                    .collect();
                let kind = if project["brownfield"] == true { "adopted as it is" } else { "created" };
                // What was already in the folder was written by somebody
                // else, and does not get to say what `std/board` means here
                // until the owner says it may.
                let trust = if project["trusted"] == false {
                    format!(
                        "\nIts own files do not stand in for the blueprints that ship until it is trusted: `{} project trust {}`",
                        brand::CLI,
                        text(&project, "id")
                    )
                } else {
                    String::new()
                };
                format!(
                    "project `{}` {kind} at {}{}{trust}\n{}",
                    text(&project, "id"),
                    text(&project, "path"),
                    crate::created::given(&project, bare),
                    files.join("\n")
                )
            });
        }
        Some(word @ ("trust" | "distrust")) => {
            let id = args.need(2, "which project")?;
            let trusted = word == "trust";
            let project =
                client.post(&format!("/api/projects/{id}/trust"), json!({ "trusted": trusted })).await?;
            show(client, &project, || {
                if trusted {
                    format!(
                        "`{id}` is trusted: a blueprint among its own files with the name of one that ships is used in its place there"
                    )
                } else {
                    format!(
                        "`{id}` is not trusted: the blueprints that ship are used there, whatever its own files are called"
                    )
                }
            });
        }
        Some("rename") => {
            let id = args.need(2, "which project")?;
            let name = args.words_from(3).join(" ");
            if name.trim().is_empty() {
                return Err(format!(
                    "say what `{id}` is to be called: `{} project rename {id} <name>`",
                    brand::CLI
                ));
            }
            let project = client
                .call(reqwest::Method::PATCH, &format!("/api/projects/{id}"), Some(json!({ "name": name })))
                .await?;
            show(client, &project, || {
                format!("`{id}` is called {} now; its id is as it was", text(&project, "name"))
            });
        }
        Some("blueprints") => crate::blueprinted::blueprints(client, args).await?,
        Some("outline") => crate::outlined::outline(client, args).await?,
        Some("tree") => {
            let tree = client.get(&format!("/api/projects/{}/tree", args.need(2, "the project")?)).await?;
            show(client, &tree, || {
                list(&tree["entries"])
                    .iter()
                    .map(|entry| {
                        format!("{}{}", text(entry, "path"), if entry["dir"] == true { "/" } else { "" })
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            });
        }
        Some("scan") => {
            let found = client
                .post(&format!("/api/projects/{}/scan", args.need(2, "the project")?), json!({}))
                .await?;
            show(client, &found, || {
                let changes = list(&found["changes"]);
                if changes.is_empty() {
                    return "nothing changed".to_string();
                }
                changes
                    .iter()
                    .map(|change| format!("{:<9}{}", text(change, "change"), text(change, "path")))
                    .collect::<Vec<_>>()
                    .join("\n")
            });
        }
        Some("forget") => {
            let id = args.need(2, "the project")?;
            let gone = client.call(reqwest::Method::DELETE, &format!("/api/projects/{id}"), None).await?;
            show(client, &gone, || {
                let mut said = vec![match gone["trash"].as_str() {
                    Some(trash) => format!(
                        "forgot `{id}`, into the Trash as `{trash}`; its files are where they were, and `{cli} trash restore {trash}` gives it back",
                        cli = brand::CLI
                    ),
                    None => format!("forgot `{id}`; its files are where they were"),
                }];
                said.extend(crate::removed::still_running(&gone["running"]));
                said.join("\n")
            });
        }
        Some(other) => {
            return Err(format!(
                "`project {other}` is not a command; try list, create, tree, scan, blueprints or forget"
            ));
        }
    }
    Ok(())
}

pub async fn fleet(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        Some("init") => {
            let project =
                args.word(2).map(str::to_string).or_else(|| std::env::var(brand::env_var("PROJECT")).ok());
            let project = project.ok_or("missing the project")?;
            let entry = client.post(&format!("/api/projects/{project}/fleet"), json!({})).await?;
            show(client, &entry, || {
                format!(
                    "{path} is kept for `{project}`. Read it ({cli} file read {project} {path}), then start the \
                     fleet with\n  {cli} apply fleet --project {project}",
                    path = text(&entry, "path"),
                    cli = brand::CLI
                )
            });
            Ok(())
        }
        None | Some("status") => status(client).await,
        Some(other) => Err(format!("`fleet {other}` is not a command; try `fleet init <project>`")),
    }
}

pub fn project_of(args: &Args) -> Option<String> {
    args.value("project")
        .map(str::to_string)
        .or_else(|| std::env::var(brand::env_var("PROJECT")).ok().filter(|p| !p.is_empty()))
}

/// Prints diagnostics the way a compiler does. Returns how many are errors.
pub(crate) fn print_diagnostics(file: &str, diagnostics: &Value) -> usize {
    let mut errors = 0;
    for diagnostic in list(diagnostics) {
        let level = text(diagnostic, "severity");
        errors += usize::from(level == "error");
        let start = &diagnostic["span"]["start"];
        println!("{file}:{}:{}: {level}: {}", start["line"], start["col"], text(diagnostic, "message"));
    }
    errors
}

/// What an editor would mark in a file that is called as YAML
/// (`fleet.bp.yml`) and that checks all the same: the values written plain
/// that YAML does not read as they are read here. Each is a warning, with
/// what puts it right. A file called `.bp` is no editor's YAML until
/// somebody says so, and is told nothing.
fn marked_as_yaml(file: &str, source: &str) -> Vec<blueprint::Diagnostic> {
    if !brand::is_blueprint_as_yaml(file) {
        return Vec::new();
    }
    let said = format!(
        "this is written plain, and YAML does not read it as it is read here: an editor marks the line, or \
         takes the value for another. `{} fmt {file}` puts it in quotes, where it is the same to both",
        brand::CLI
    );
    let places = blueprint::format::not_yaml(source);
    places.into_iter().map(|span| blueprint::Diagnostic::warning("not-yaml", said.clone(), span)).collect()
}

pub async fn check(client: &Client, args: &Args) -> Result<(), String> {
    if args.word(1).is_none() {
        return Err("missing the blueprint file to check".into());
    }
    let mut failed = 0;
    let mut index = 1;
    while let Some(file) = args.word(index) {
        index += 1;
        let source = std::fs::read_to_string(file).map_err(|err| format!("cannot read {file}: {err}"))?;
        let marked = marked_as_yaml(file, &source);
        let body = json!({ "source": source, "project": project_of(args) });
        let mut answered = client.post("/api/blueprints/check", body).await;
        // Said among what the daemon found, each where it is in the file.
        if let Ok(checked) = &mut answered
            && let Some(found) = checked["diagnostics"].as_array_mut()
        {
            found.extend(marked.iter().map(|one| json!(one)));
            let place =
                |one: &Value| (one["span"]["start"]["line"].as_u64(), one["span"]["start"]["col"].as_u64());
            found.sort_by_key(place);
        }
        let errors = match answered {
            Ok(checked) if client.json => {
                println!("{}", serde_json::to_string_pretty(&checked).unwrap_or_default());
                usize::from(checked["ok"] == false)
            }
            Ok(checked) => print_diagnostics(file, &checked["diagnostics"]),
            // No daemon to ask: check what can be known from the file alone.
            Err(error) if Client::is_unreachable(&error) => {
                let mut checked = blueprint::check(&source, &blueprint::NoCatalog);
                checked.diagnostics.extend(marked);
                checked.diagnostics.sort_by_key(|one| (one.span.start.line, one.span.start.col));
                println!("{}", blueprint::render_diagnostics(file, &checked.diagnostics));
                eprintln!("note: {error}; actions and permissions were not checked");
                checked.errors().count()
            }
            Err(error) => return Err(error),
        };
        if errors == 0 && !client.json {
            println!("{file}: ok");
        }
        failed += usize::from(errors > 0);
    }
    if failed > 0 { Err(format!("{failed} file(s) have errors")) } else { Ok(()) }
}

/// A blueprint's graph as indented text: each step, what it runs, what it
/// waits for and under what condition.
pub fn graph_text(graph: &Value) -> String {
    let mut lines = vec![match graph["description"].as_str() {
        Some(description) => format!("{}: {description}", text(graph, "name")),
        None => text(graph, "name").to_string(),
    }];
    for group in list(&graph["groups"]) {
        let id = text(group, "id");
        lines.push(text(group, "label").to_string());
        let mut rows: Vec<Vec<String>> = Vec::new();
        for node in list(&graph["nodes"]).iter().filter(|node| node["group"] == id) {
            let after: Vec<&str> = list(&graph["edges"])
                .iter()
                .filter(|edge| edge["to"] == node["id"] && edge["kind"] != "fires")
                .filter_map(|edge| edge["from"].as_str().and_then(|from| from.rsplit('/').next()))
                .collect();
            let what = match text(node, "kind") {
                "trigger" => format!("when {}", text(node, "target")),
                "action" => text(node, "target").to_string(),
                kind => format!("{kind} {}", text(node, "target")),
            };
            let mut notes: Vec<String> = Vec::new();
            if !after.is_empty() {
                notes.push(format!("after {}", after.join(", ")));
            }
            for (key, label) in [
                ("when", "if"),
                ("for_each", "for each"),
                ("timeout", "timeout"),
                ("retry", "retry"),
                ("device", "on"),
            ] {
                if let Some(value) = node[key].as_str() {
                    notes.push(format!("{label} {value}"));
                }
            }
            let hooks: Vec<&str> =
                list(&node["hooks"]).iter().filter_map(|hook| hook["point"].as_str()).collect();
            if !hooks.is_empty() {
                notes.push(hooks.join("+"));
            }
            if !list(&node["watch"]).is_empty() {
                notes.push(format!("{} watch rule(s)", list(&node["watch"]).len()));
            }
            let name =
                if node["kind"] == "trigger" { "@".to_string() } else { text(node, "name").to_string() };
            rows.push(vec![format!("  {name}"), what, notes.join("; ")]);
        }
        lines.push(table(&rows));
    }
    lines.join("\n")
}

pub async fn graph(client: &Client, args: &Args) -> Result<(), String> {
    let file = args.need(1, "the blueprint file")?;
    let source = std::fs::read_to_string(file).map_err(|err| format!("cannot read {file}: {err}"))?;
    let checked = match client
        .post("/api/blueprints/check", json!({ "source": source, "project": project_of(args) }))
        .await
    {
        Ok(checked) => checked,
        Err(error) if Client::is_unreachable(&error) => {
            let checked = blueprint::check(&source, &blueprint::NoCatalog);
            json!({ "graph": checked.blueprint.as_ref().map(blueprint::graph), "diagnostics": checked.diagnostics })
        }
        Err(error) => return Err(error),
    };
    if checked["graph"].is_null() {
        print_diagnostics(file, &checked["diagnostics"]);
        return Err("it has no shape to draw yet".into());
    }
    show(client, &checked["graph"], || graph_text(&checked["graph"]));
    Ok(())
}

pub fn step_rows(run: &Value) -> String {
    let rows: Vec<Vec<String>> = list(&run["steps"])
        .iter()
        .map(|step| {
            let duration = match (step["started"].as_i64(), step["finished"].as_i64()) {
                (Some(started), Some(finished)) if step["status"] != "skipped" => took(finished - started),
                _ => String::new(),
            };
            vec![
                format!("  {}", text(step, "step")),
                text(step, "status").into(),
                duration,
                text(step, "error").into(),
            ]
        })
        .collect();
    table(&rows)
}

pub async fn apply(client: &Client, args: &Args) -> Result<(), String> {
    // An instance that is there, applied again as it is: the text now
    // there for its blueprint, with what it was started with.
    if let Some(instance) = args.value("instance") {
        let body = json!({ "force": args.has("force"), "take": args.has("take"), "wait": !args.has("no-wait"),
                "back": args.has("back"), "say": args.has("say") });
        let applied = client.post(&format!("/api/instances/{}/apply", encode(instance)), body).await?;
        return crate::applied::applied_as(client, &applied);
    }
    let target = args.need(1, "a blueprint file or name, or `--instance <id>`")?;
    let mut params = serde_json::Map::new();
    for pair in args.values("param") {
        let (key, value) =
            pair.split_once('=').ok_or_else(|| format!("`--param {pair}` should be `key=value`"))?;
        params.insert(
            key.to_string(),
            serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string())),
        );
    }
    let mut labels = serde_json::Map::new();
    for pair in args.values("label") {
        let (name, value) =
            pair.split_once('=').ok_or_else(|| format!("`--label {pair}` should be `name=value`"))?;
        labels.insert(name.to_string(), Value::String(value.to_string()));
    }
    let mut body = json!({
        "project": project_of(args),
        "params": params,
        "labels": labels,
        "key": args.value("key"),
        "environment": args.value("environment"),
        "force": args.has("force"),
        "take": args.has("take"),
        "back": args.has("back"),
        "say": args.has("say"),
        "wait": !args.has("no-wait"),
    });
    match std::fs::read_to_string(target) {
        Ok(source) => body["source"] = json!(source),
        // Not a file that is there. Something that is plainly meant as a
        // file is an error; anything else is a name the daemon may know,
        // which includes names with a slash in them, like `std/rolling`.
        Err(_)
            if brand::is_blueprint_path(target)
                || target.starts_with(['.', '/', '~'])
                || target.contains('\\') =>
        {
            return Err(format!("cannot read {target}"));
        }
        Err(_) => body["blueprint"] = json!(target),
    }
    let applied = client.post("/api/apply", body).await?;
    crate::applied::applied_as(client, &applied)
}

/// The text an instance runs, in a few characters: the start of its
/// revision, the version the text says where it says one, and `behind`
/// when the text now there for its blueprint is another.
fn text_of(instance: &Value) -> String {
    let mut said = text(instance, "revision").chars().take(8).collect::<String>();
    if let Some(version) = instance["version"].as_str() {
        said.push_str(&format!(" v{version}"));
    }
    if !instance["behind"].is_null() {
        said.push_str(" behind");
    }
    said
}

/// What an instance says beyond its row, each line set in under it: what
/// it says of itself beside its status, why it is behind, and what moved
/// between the text it runs and the one now there.
fn said_under(instance: &Value) -> Vec<String> {
    let behind = &instance["behind"];
    // What moved, by the version each calls itself first.
    let between = json!({
        "name": instance["blueprint"], "was": instance["version"], "now": behind["version"],
        "changed": behind["changed"],
    });
    // Said where something moved, or where the two say different versions.
    let differs =
        !list(&behind["changed"]).is_empty() || (!behind.is_null() && between["was"] != between["now"]);
    let moved = differs.then(|| crate::applied::moved(&between));
    let said = [instance["error"].as_str(), behind["why"].as_str(), moved.as_deref()];
    said.into_iter().flatten().flat_map(str::lines).map(|line| format!("    {line}")).collect()
}

pub async fn instances(client: &Client, args: &Args) -> Result<(), String> {
    let live = if args.has("all") { "" } else { "live=true&" };
    let project = project_of(args).map(|p| format!("project={}&", encode(&p))).unwrap_or_default();
    let limit = args.value("limit").unwrap_or("200");
    let found = client.get(&format!("/api/instances?{live}{project}limit={}", encode(limit))).await?;
    // A list cut at so many says so, under it, and not in what is printed as JSON.
    if found["more"] == true {
        let listed = list(&found["instances"]).len();
        eprintln!("There are more than these {listed}, the newest; `--limit` says how many are listed.");
    }
    show(client, &found, || {
        let now = now_ms();
        let rows: Vec<Vec<String>> = list(&found["instances"])
            .iter()
            .map(|i| {
                let parent = i["parent"].as_str().map_or(String::new(), |parent| format!("in {parent}"));
                // Under which of its project's environments, where it is under one.
                let under = i["environment"].as_str().filter(|under| !under.is_empty());
                let parent = match under {
                    Some(under) => format!("under {under} {parent}").trim_end().to_string(),
                    None => parent,
                };
                vec![
                    text(i, "id").into(),
                    text(i, "blueprint").into(),
                    text(i, "key").into(),
                    text(i, "status").into(),
                    text_of(i),
                    ago(now, i["updated"].as_i64().unwrap_or(now)),
                    parent,
                ]
            })
            .collect();
        if rows.is_empty() {
            return "nothing is running".to_string();
        }
        let lines = table(&rows);
        let under = list(&found["instances"]).iter().map(said_under);
        let lines =
            lines.lines().zip(under).flat_map(|(row, under)| std::iter::once(row.to_string()).chain(under));
        let hearing = crate::applied::hearing_every_project(list(&found["instances"]));
        lines.chain(hearing).collect::<Vec<_>>().join("\n")
    });
    Ok(())
}

pub async fn instance(client: &Client, args: &Args) -> Result<(), String> {
    let found = client.get(&format!("/api/instances/{}", args.need(1, "the instance's id")?)).await?;
    show(client, &found, || {
        let i = &found["instance"];
        let mut lines = vec![format!(
            "{} {} ({}) {}",
            text(i, "id"),
            text(i, "blueprint"),
            text(i, "key"),
            text(i, "status")
        )];
        lines.push(format!("  text {}", text_of(i)));
        lines.extend(said_under(i));
        if crate::applied::hears_every_project(i) {
            lines.push(format!("  hears {}", crate::applied::HEARS_EVERY_PROJECT));
        }
        lines.push(format!("  params {}  state {}", i["params"], i["state"]));
        if !list(&i["holds"]).is_empty() {
            lines.push(format!("  holds {}", i["holds"]));
        }
        for child in list(&found["children"]) {
            lines.push(format!(
                "  child {} {} {}",
                text(child, "id"),
                text(child, "blueprint"),
                text(child, "status")
            ));
        }
        for run in list(&found["runs"]).iter().take(8) {
            lines.push(format!("  run {} {} {}", text(run, "id"), text(run, "flow"), text(run, "status")));
        }
        lines.join("\n")
    });
    Ok(())
}

pub async fn stop(client: &Client, args: &Args) -> Result<(), String> {
    let id = args.need(1, "the instance's id")?;
    client.post(&format!("/api/instances/{id}/stop"), json!({})).await?;
    println!("stopped {id}");
    Ok(())
}

/// What an argument says is written inside it, as lines: where each
/// thing is written, what kind of thing it is, and its sentence. In the
/// order the program itself writes them (`blueprint::Within::rows`).
fn inside(within: &Value, under: &str, rows: &mut Vec<Vec<String>>) {
    let mut row =
        |path: String, kind: &str, says: &str| rows.push(vec![format!("  {path}"), kind.into(), says.into()]);
    let under = match within["each"].as_str() {
        Some(each) => {
            row(format!("{under}*"), "each", each);
            format!("{under}*.")
        }
        None => under.to_string(),
    };
    for word in list(&within["words"]) {
        row(format!("{under}{}", text(word, "name")), "word", text(word, "help"));
    }
    for key in list(&within["keys"]) {
        let path = format!("{under}{}", text(key, "name"));
        row(path.clone(), text(key, "type"), text(key, "help"));
        for word in list(&key["words"]) {
            row(format!("{path}.{}", text(word, "name")), "word", text(word, "help"));
        }
    }
    for key in list(&within["keys"]) {
        inside(&key["within"], &format!("{under}{}.", text(key, "name")), rows);
    }
}

pub async fn actions(client: &Client, args: &Args) -> Result<(), String> {
    let catalog = client.get("/api/actions").await?;
    let all = list(&catalog["actions"]);
    match args.word(1) {
        None => show(client, &catalog, || {
            let rows: Vec<Vec<String>> = all
                .iter()
                .map(|action| vec![text(action, "name").into(), text(action, "summary").into()])
                .collect();
            table(&rows)
        }),
        Some(name) => {
            let action = all
                .iter()
                .find(|action| action["name"] == name)
                .ok_or_else(|| format!("there is no action called `{name}`"))?;
            show(client, action, || {
                let mut lines = vec![format!("{}: {}", name, text(action, "summary"))];
                if let Some(capability) = action["capability"].as_str() {
                    lines.push(format!("needs: {capability}"));
                }
                let rows: Vec<Vec<String>> = list(&action["args"])
                    .iter()
                    .map(|arg| {
                        let required = if arg["required"] == true { "required" } else { "" };
                        vec![
                            format!("  {}", text(arg, "name")),
                            text(arg, "type").into(),
                            required.into(),
                            text(arg, "help").into(),
                        ]
                    })
                    .collect();
                if !rows.is_empty() {
                    lines.push("with:".into());
                    lines.push(table(&rows));
                }
                // What an argument says is written inside it, a line for
                // each key and each word, by where it is written.
                for arg in list(&action["args"]) {
                    let mut rows = Vec::new();
                    inside(&arg["within"], "", &mut rows);
                    if !rows.is_empty() {
                        let name = text(arg, "name");
                        lines.push(format!("inside `{name}` (`*` is a name of your own choosing):"));
                        lines.push(table(&rows));
                    }
                }
                let outputs: Vec<Vec<String>> = list(&action["outputs"])
                    .iter()
                    .map(|output| {
                        vec![
                            format!("  {}", output[0].as_str().unwrap_or_default()),
                            output[1].as_str().unwrap_or_default().into(),
                        ]
                    })
                    .collect();
                if !outputs.is_empty() {
                    lines.push("returns:".into());
                    lines.push(table(&outputs));
                }
                lines.join("\n")
            });
        }
    }
    Ok(())
}
