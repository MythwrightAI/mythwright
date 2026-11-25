//! The sign-ins the factory holds for its agents' programs.
//!
//! A value (a key, a token) is a secret. It is read from standard input or
//! from a file, never from an argument: arguments are seen by everything
//! else on the machine and kept by the shell. It is sent to the daemon
//! once and never shown again.

use std::io::{IsTerminal, Read};

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, now_ms, table};
use crate::work::{list, show, text};

/// The value of a key or a token: what the file named holds, or what
/// arrives on standard input.
fn value(args: &Args, kind: &str) -> Result<String, String> {
    let read = match args.value("file") {
        Some(file) => std::fs::read_to_string(file).map_err(|err| format!("{file}: {err}"))?,
        None => {
            if std::io::stdin().is_terminal() {
                // Typed at a terminal, it is not put on the screen either,
                // where the terminal can be told so: `stty` is Unix's.
                // Elsewhere it is shown, and that is said.
                let hides = cfg!(unix);
                match hides {
                    true => eprintln!("Paste the {kind} and press Enter (it is not shown):"),
                    false => eprintln!(
                        "Paste the {kind} and press Enter. It IS shown here as it is typed: to keep it off the \
                         screen, give it in a file with `--file`."
                    ),
                }
                let quiet = |how: &str| {
                    if hides {
                        let _ = std::process::Command::new("stty").arg(how).status();
                    }
                };
                quiet("-echo");
                let mut line = String::new();
                let read = std::io::stdin().read_line(&mut line);
                quiet("echo");
                eprintln!();
                read.map_err(|err| format!("reading the {kind}: {err}"))?;
                line
            } else {
                let mut all = String::new();
                std::io::stdin()
                    .read_to_string(&mut all)
                    .map_err(|err| format!("reading the {kind}: {err}"))?;
                all
            }
        }
    };
    let read = read.trim().to_string();
    if read.is_empty() {
        return Err(format!(
            "no {kind} was given: send it on standard input (`... < file`, or a pipe), or name a file that \
             holds it with `--file`"
        ));
    }
    Ok(read)
}

/// How one sign-in stands, in a few words.
fn standing(signin: &Value, now: i64) -> String {
    let held: Vec<&str> = list(&signin["held_by"]).iter().map(|holder| text(holder, "slot")).collect();
    match signin["aside_until"].as_i64() {
        Some(until) if until > now => {
            let left = crate::client::took(until - now);
            format!("set aside, good again in {left}")
        }
        _ if !held.is_empty() => format!("held by {}", held.join(", ")),
        _ => "free".to_string(),
    }
}

pub async fn signin(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/signins").await?;
            show(client, &found, || {
                let now = now_ms();
                let rows: Vec<Vec<String>> = list(&found["signins"])
                    .iter()
                    .map(|signin| {
                        vec![
                            text(signin, "name").into(),
                            text(signin, "for").into(),
                            text(signin, "kind").into(),
                            format!("{} at once", signin["at_once"]),
                            standing(signin, now),
                            match signin["last_used"].as_i64() {
                                Some(used) => format!("used {}", ago(now, used)),
                                None => "never used".into(),
                            },
                            signin["folder"].as_str().unwrap_or_default().into(),
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    format!(
                        "the factory holds no sign-ins: every agent's program signs in as whoever runs the \
                         daemon. `{cli} help signin` says how to add one"
                    )
                } else {
                    table(&rows)
                }
            });
        }
        Some("add") => {
            let name = args.need(2, "the sign-in's name")?;
            let package =
                args.value("for").ok_or("say which extension package's program it signs in, with `--for`")?;
            let kind = args.value("kind").ok_or("say what it is with `--kind`: key, token or folder")?;
            let at_once = match args.value("at-once") {
                Some(most) => Some(most.parse::<u32>().map_err(|_| {
                    format!("`--at-once` is how many agents may hold it at once; `{most}` is not a number")
                })?),
                None => None,
            };
            let mut body = json!({ "name": name, "for": package, "kind": kind, "at_once": at_once });
            match kind {
                "folder" => {
                    let folder = args.value("folder").ok_or(
                        "say where the provider's program keeps its own sign-in, with `--folder <directory>`",
                    )?;
                    // By its whole path: the daemon is not where this was typed.
                    let whole = std::path::absolute(folder).map_err(|err| format!("{folder}: {err}"))?;
                    body["folder"] = json!(whole);
                }
                "key" | "token" => body["value"] = json!(value(args, kind)?),
                // The daemon says what the kinds are.
                _ => {}
            }
            let added = client.post("/api/signins", body).await?;
            show(client, &added, || {
                format!(
                    "`{name}` is held: a {kind} for `{package}`, for {} at once. An agent draws it when told \
                     to: `{cli} agent add <name> --adapter {package} --signin {}`",
                    added["at_once"],
                    if kind == "key" { "key" } else { "subscription" },
                )
            });
        }
        Some("remove") => {
            let name = args.need(2, "the sign-in's name")?;
            client.call(reqwest::Method::DELETE, &format!("/api/signins/{name}"), None).await?;
            println!("`{name}` is no longer held; a turn that was handed it keeps it until it ends");
        }
        Some("restore") => {
            let name = args.need(2, "the sign-in's name")?;
            client.post(&format!("/api/signins/{name}/restore"), json!({})).await?;
            println!("`{name}` is good again");
        }
        Some(other) => {
            return Err(format!("`signin {other}` is not a command; try list, add, remove or restore"));
        }
    }
    Ok(())
}

/// `secret list [--where] | set <name> [--file F] | remove <name>`: what the
/// factory keeps for the programs blueprints run. A value goes in as a
/// sign-in's does, from standard input or a file, and is never shown.
pub async fn secret(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    if args.value("project").is_some() || args.value("environment").is_some() {
        return environment_secret(client, args).await;
    }
    match args.word(1) {
        None | Some("list") => {
            // Where each is named is the owner's to ask, so it is asked
            // only when it is asked for: `list` alone is for anybody who
            // may use one.
            let named = args.has("where");
            let found = client.get(if named { "/api/secrets?named=true" } else { "/api/secrets" }).await?;
            show(client, &found, || {
                let now = now_ms();
                let rows: Vec<Vec<String>> = list(&found["secrets"])
                    .iter()
                    .map(|kept| {
                        let set = match kept["set"].as_i64() {
                            Some(set) => format!("set {}", ago(now, set)),
                            None => "not set".to_string(),
                        };
                        let mut row = vec![text(kept, "name").into(), set];
                        if named {
                            row.push(match text(kept, "set_by") {
                                "" => String::new(),
                                by => format!("by {by}"),
                            });
                            row.push(named_by(&kept["named"]).unwrap_or_else(|| "named nowhere".to_string()));
                        }
                        row
                    })
                    .collect();
                if rows.is_empty() {
                    format!("the factory keeps no secrets. `{cli} help secret` says how one is set and used")
                } else {
                    table(&rows)
                }
            });
            Ok(())
        }
        Some("set") => {
            let name = args.need(2, "the secret's name, such as `stripe`")?;
            let body = json!({ "value": value(args, "secret")? });
            let path = format!("/api/secrets/{}", crate::client::encode(name));
            client.call(reqwest::Method::PUT, &path, Some(body)).await?;
            println!(
                "{name} is set. A blueprint names it where a program reads it: `env: {{ KEY: {{ secret: {name} }} }}`"
            );
            Ok(())
        }
        Some("remove") => {
            let name = args.need(2, "the secret's name")?;
            let path = format!("/api/secrets/{}", crate::client::encode(name));
            client.call(reqwest::Method::DELETE, &path, None).await?;
            println!("{name} is gone. A service that was handed it has it until it is next started");
            // What goes on naming it is what will fail for want of it.
            let left = client.get("/api/secrets?named=true").await.unwrap_or_default();
            let still = list(&left["secrets"]).iter().find(|one| text(one, "name") == name);
            if let Some(by) = still.and_then(|one| named_by(&one["named"])) {
                println!("It is still named by {by}: each fails for want of it until it is set again");
            }
            Ok(())
        }
        Some(other) => Err(format!(
            "`secret {other}` is not a command; there are `list [--where]`, `set <name>` and `remove <name>`"
        )),
    }
}

/// `secret … --project P --environment E`: the secrets of one of a
/// project's environments, by name, set and taken away as the factory's
/// are, and said of agents (`agents <name> VAR|--not`).
async fn environment_secret(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    let (Some(project), Some(environment)) = (args.value("project"), args.value("environment")) else {
        return Err(format!(
            "a secret of a project's environment is named by both: `--project P --environment E` \
             (`{cli} project environment <project>` lists them)"
        ));
    };
    let at = format!(
        "/api/projects/{}/environments/{}",
        crate::client::encode(project),
        crate::client::encode(environment)
    );
    let one = |name: &str| format!("{at}/secrets/{}", crate::client::encode(name));
    let of = format!("`{environment}` of `{project}`");
    match args.word(1) {
        None | Some("list") => {
            let found = client.get(&at).await?;
            show(client, &found, || {
                let now = now_ms();
                let Some(secrets) = found["secrets"].as_object() else {
                    return format!("the names of {of}'s secrets are for whoever holds `secrets:use`");
                };
                let rows: Vec<Vec<String>> = secrets
                    .iter()
                    .map(|(name, kept)| {
                        let set = match kept["kept"] == true {
                            true => format!(
                                "set {} by {}",
                                ago(now, kept["set"].as_i64().unwrap_or(0)),
                                text(kept, "by_name")
                            ),
                            false => "not set: a copy carries no value".to_string(),
                        };
                        let agents = match kept["agents"].as_str() {
                            Some(variable) => format!("handed to agents as {variable}"),
                            None => "handed to no agent".to_string(),
                        };
                        vec![name.clone(), set, agents]
                    })
                    .collect();
                if rows.is_empty() {
                    format!("{of} keeps no secrets. `{cli} help secret` says how one is set")
                } else {
                    table(&rows)
                }
            });
            Ok(())
        }
        Some("set") => {
            let name = args.need(2, "the secret's name, such as `stripe`")?;
            let body = json!({ "value": value(args, "secret")? });
            client.call(reqwest::Method::PUT, &one(name), Some(body)).await?;
            println!("{name} of {of} is set. It is never shown again; `set` again replaces it");
            Ok(())
        }
        Some("remove") => {
            let name = args.need(2, "the secret's name")?;
            client.call(reqwest::Method::DELETE, &one(name), None).await?;
            println!("{name} of {of} is gone");
            Ok(())
        }
        Some("agents") => {
            let name = args.need(2, "the secret's name")?;
            let agents = match (args.word(3), args.has("not")) {
                (Some(variable), false) => json!(variable),
                (None, true) => Value::Null,
                _ => {
                    return Err(format!(
                        "say what an agent is handed it as, such as `{cli} secret agents {name} STRIPE_KEY \
                         --project {project} --environment {environment}`, or `--not` for no agent"
                    ));
                }
            };
            let path = format!("{}/agents", one(name));
            client.call(reqwest::Method::PUT, &path, Some(json!({ "agents": agents.clone() }))).await?;
            match agents.as_str() {
                Some(variable) => println!("an agent working under {of} is handed {name} as {variable}"),
                None => println!("{name} of {of} is handed to no agent"),
            }
            Ok(())
        }
        Some(other) => Err(format!(
            "`secret {other}` is not a command for an environment; there are `list`, `set <name>`, \
             `remove <name>` and `agents <name> VAR|--not`"
        )),
    }
}

/// What names a secret, as a line: the agents handed it, the services
/// started with it and the instances whose texts name it. Nothing where
/// nothing does.
fn named_by(named: &Value) -> Option<String> {
    let of = |kind: &str, said: &dyn Fn(&Value) -> String| -> Vec<String> {
        list(&named[kind]).iter().map(said).collect()
    };
    let mut by = of("agents", &|one| format!("agent {} ({})", text(one, "name"), text(one, "variable")));
    by.extend(of("services", &|one| format!("service {} ({})", text(one, "name"), text(one, "variable"))));
    by.extend(of("instances", &|one| match text(one, "key") {
        "" => format!("instance {}", text(one, "blueprint")),
        key => format!("instance {} ({key})", text(one, "blueprint")),
    }));
    (!by.is_empty()).then(|| by.join(", "))
}
