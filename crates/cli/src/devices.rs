//! The other devices of a fleet.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, now_ms, table};
use crate::work::{show, text};

pub async fn device(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/devices").await?;
            show(client, &found, || {
                let now = now_ms();
                // The short name of the key each proves is said beside it,
                // for two people to compare what each of them sees.
                let mut lines =
                    vec![format!("this device is `{}`, {}", text(&found, "this"), text(&found, "id"))];
                let hub = &found["hub"];
                if !hub.is_null() {
                    lines.push(match (hub["connected"] == true, hub["error"].as_str()) {
                        (true, _) => format!(
                            "it is a member of the fleet at {} ({}), as `{}`",
                            text(hub, "url"),
                            text(hub, "id"),
                            hub["device"].as_str().unwrap_or("?")
                        ),
                        (false, why) => format!(
                            "it has joined the fleet at {} but is not linked: {}",
                            text(hub, "url"),
                            why.unwrap_or("still trying")
                        ),
                    });
                }
                // Taken out by the hub it had joined: said until it joins again.
                if let Some(why) = found["unjoined"]["why"].as_str() {
                    lines.push(format!(
                        "it was a member of the fleet at {} until {}: {why}",
                        text(&found["unjoined"], "hub"),
                        kernel::clock::rfc3339(found["unjoined"]["at"].as_i64().unwrap_or(0))
                    ));
                }
                let rows: Vec<Vec<String>> = found["devices"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(|device| {
                        let info = &device["info"];
                        vec![
                            text(device, "name").into(),
                            device["id"].as_str().unwrap_or("").to_string(),
                            if device["connected"] == true {
                                "linked".into()
                            } else if device["invited"].is_i64() {
                                "invited".to_string()
                            } else {
                                "not linked".to_string()
                            },
                            match (device["last_seen"].as_i64(), device["invited"].as_i64()) {
                                (_, Some(until)) if until > now => "has yet to use its invitation".into(),
                                (Some(at), _) if device["connected"] != true => {
                                    format!("last heard from {}", ago(now, at))
                                }
                                (Some(_), _) => String::new(),
                                (None, _) => "never heard from".into(),
                            },
                            if info.is_null() {
                                String::new()
                            } else {
                                format!(
                                    "{} {}, {} CPU, {} services",
                                    text(info, "os"),
                                    text(info, "arch"),
                                    info["cpus"],
                                    info["services"].as_array().map_or(0, Vec::len)
                                )
                            },
                        ]
                    })
                    .collect();
                lines.push(if rows.is_empty() {
                    format!(
                        "no other devices in its fleet; `{} device invite <name>` lets one in",
                        brand::CLI
                    )
                } else {
                    table(&rows)
                });
                lines.join("\n")
            });
        }
        Some("invite") => {
            let invited = client
                .post(
                    "/api/devices",
                    json!({
                        "name": args.need(2, "a name for the device")?,
                        "expires": args.value("expires"),
                        "at": args.values("at"),
                    }),
                )
                .await?;
            show(client, &invited, || {
                let cli = brand::CLI;
                // The invitation on a line of its own, and beside it the
                // command that reads one from standard input: written
                // into the command, it was in the history of every shell
                // it was pasted into, which is what joining so avoids.
                let mut lines = vec![
                    format!("`{}` is invited. Its invitation, shown this once:", text(&invited, "name")),
                    format!("  {}", text(&invited, "invitation")),
                    format!(
                        "On that device, with its daemon running and its first account made (`{cli} up`, then `{cli} register`), run"
                    ),
                    format!("  {cli} device join"),
                    "and give it the invitation when it asks: read so, it is in no shell's history."
                        .to_string(),
                    format!(
                        "It lets one device in, once, within the next {}. Pass it on as you would a password.",
                        match (invited["expires"].as_i64().unwrap_or(0) - now_ms()) as f64 / 3_600_000.0 {
                            hours if hours < 1.5 => "hour".to_string(),
                            hours if hours < 47.5 => format!("{} hours", hours.round()),
                            hours => format!("{} days", (hours / 24.0).round()),
                        }
                    ),
                ];
                if let Some(note) = invited["note"].as_str() {
                    lines.push(format!("note: {note}"));
                }
                lines.join("\n")
            });
        }
        Some("add") => {
            return Err(format!(
                "a device is no longer added with a credential: it is invited. `{} device invite <name>` makes the invitation",
                brand::CLI
            ));
        }
        Some("remove") => {
            let name = args.need(2, "the device's name")?;
            client.call(reqwest::Method::DELETE, &format!("/api/devices/{name}"), None).await?;
            println!(
                "`{name}` is removed: it is no longer let in, and an invitation made for it no longer works"
            );
        }
        Some("join") => {
            if args.word(3).is_some() {
                return Err(format!(
                    "a device no longer joins by an address and a credential. On the hub, run `{0} device invite <name>`; here, `{0} device join <invitation>`",
                    brand::CLI
                ));
            }
            let allow: Option<Vec<&str>> =
                args.value("allow").map(|scopes| scopes.split(',').map(str::trim).collect());
            // Given as a word; or, so that it need be in no shell's history,
            // on standard input: `device join -`, or with nothing after it.
            // A device nobody has registered at has nobody to join as:
            // said before the invitation is asked for, with what to do
            // first and that the invitation is still good afterwards.
            if client.get("/api/meta").await.is_ok_and(|meta| meta["needs_setup"] == true) {
                return Err(format!(
                    "this device's factory has no account yet, and joining is asked by whoever is signed in to it. \
                     Make its first account, `{0} register`, and then `{0} device join` again: the invitation has \
                     not been used",
                    brand::CLI
                ));
            }
            let invitation = match args.word(2).filter(|given| *given != "-") {
                Some(given) => given.to_string(),
                None => {
                    // Asked for where somebody is there to be asked.
                    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                        eprint!("invitation: ");
                    }
                    let mut read = String::new();
                    std::io::stdin()
                        .read_line(&mut read)
                        .map_err(|err| format!("could not read the invitation from standard input: {err}"))?;
                    read.trim().to_string()
                }
            };
            if invitation.is_empty() {
                return Err(
                    "an invitation is needed: give it after `device join`, or on standard input".into()
                );
            }
            let joined =
                client.post("/api/gateway/join", json!({ "invitation": invitation, "allow": allow })).await?;
            show(client, &joined, || {
                let hub = &joined["hub"];
                format!(
                    "joined the fleet at {} ({}); this device now does what that hub's blueprints send it{}\n\
                     What crosses between the two is sealed, and this device holds the hub to that key from now on.",
                    text(hub, "url"),
                    text(hub, "id"),
                    match &allow {
                        Some(allow) => format!(", limited to: {}", allow.join(", ")),
                        None => String::new(),
                    },
                )
            });
        }
        Some("leave") => {
            client.post("/api/gateway/leave", Value::Null).await?;
            println!("left the fleet; what was started here keeps running");
        }
        Some(other) => {
            return Err(format!("`device {other}` is not a command; see `{} help device`", brand::CLI));
        }
    }
    Ok(())
}

/// The names this daemon answers to besides the addresses it listens on.
pub async fn names(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/system/names").await?;
            show(client, &found, || {
                let told: Vec<&str> = found["names"]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .filter_map(Value::as_str)
                    .collect();
                let own = format!(
                    "this daemon listens at {} and answers there, and as `localhost`",
                    text(&found, "address")
                );
                if told.is_empty() {
                    format!(
                        "{own}; it has been told no other name\n`{} names add <name>` tells it one it is reached under (a tunnel's, or this device's on a network of your own)",
                        brand::CLI
                    )
                } else {
                    format!("{own}; and as:\n  {}", told.join("\n  "))
                }
            });
        }
        Some("add") => {
            let name = args.need(2, "the name the daemon is reached under, such as box.example.net")?;
            let told = client.post("/api/system/names", json!({ "name": name })).await?;
            show(client, &told, || format!("this daemon now answers as `{}`", text(&told, "name")));
        }
        Some("remove") => {
            let name = args.need(2, "the name to take back")?;
            let gone =
                client.call(reqwest::Method::DELETE, &format!("/api/system/names/{name}"), None).await?;
            show(client, &gone, || format!("this daemon no longer answers as `{}`", text(&gone, "name")));
        }
        Some(other) => {
            return Err(format!(
                "`names {other}` is not a command; it is `names`, `names add <name>` or `names remove <name>`"
            ));
        }
    }
    Ok(())
}
