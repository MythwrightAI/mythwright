//! An agent a factory does not start (`agents::external`): `agent
//! invite`, by whoever may add agents; `agent join` and `agent leave`,
//! by whoever is to be that agent, wherever its program runs.
//!
//! The invitation is shown once and the credential never: `join` keeps
//! it where a command asked as that agent finds it (`crate::joined`),
//! and prints where, not what.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, now_ms};
use crate::work::{list, show, text};

/// How long from `now` until `until`, in the words an invitation is said in.
fn within(until: i64, now: i64) -> String {
    match (until - now) as f64 / 3_600_000.0 {
        hours if hours < 1.5 => "hour".to_string(),
        hours if hours < 47.5 => format!("{} hours", hours.round()),
        hours => format!("{} days", (hours / 24.0).round()),
    }
}

/// How an external agent stands and what it holds, in a line of the
/// list of agents: nothing of a program is said of it there.
pub fn standing(agent: &Value, now: i64) -> String {
    let holds: Vec<&str> = list(&agent["scopes"]).iter().filter_map(Value::as_str).collect();
    // What one is invited with where nothing is said: what an agent
    // usually holds, and its own inbox.
    let usual = agents::external::usual();
    let holds = match holds.len() == usual.len() && usual.iter().all(|scope| holds.contains(&scope.as_str()))
    {
        true => "what an agent usually does, and its own inbox".to_string(),
        false => holds.join(", "),
    };
    format!("{}; holds {holds}", stands(agent, now))
}

fn stands(agent: &Value, now: i64) -> String {
    let external = &agent["external"];
    match (external["joined"] == true, external["invited_until"].as_i64()) {
        (true, Some(until)) => format!("joined; invited again, good for the next {}", within(until, now)),
        (true, None) => "joined".to_string(),
        (false, Some(until)) => format!("invited, has yet to join: good for the next {}", within(until, now)),
        (false, None) => {
            format!("not joined: `{} agent invite {}` lets it in", brand::CLI, text(agent, "name"))
        }
    }
}

/// What is said when something is said again of an external agent.
pub fn said_again(agent: &Value) -> String {
    let holds: Vec<&str> = list(&agent["scopes"]).iter().filter_map(Value::as_str).collect();
    format!(
        "{name} is an external agent, which this factory does not start. It holds: {holds}\n\
         What it holds is so from the next thing it asks; `{cli} agent list` says whether it has joined",
        name = text(agent, "name"),
        holds = holds.join(", "),
        cli = brand::CLI
    )
}

pub async fn invite(client: &Client, args: &Args) -> Result<(), String> {
    let scopes = args.values("scope");
    let body = json!({
        "name": args.need(2, "a name for the agent")?,
        "project": args.value("project"),
        "scopes": if scopes.is_empty() { Value::Null } else { json!(scopes) },
        "expires": args.value("expires"),
    });
    let invited = client.post("/api/agent/invite", body).await?;
    show(client, &invited, || {
        let (cli, agent) = (brand::CLI, &invited["agent"]);
        let name = text(agent, "name");
        let holds: Vec<&str> = list(&agent["scopes"]).iter().filter_map(Value::as_str).collect();
        // The invitation on a line of its own, and beside it the command
        // that reads one from standard input: written into the command,
        // it would be in the history of every shell it was pasted into.
        [
            format!("`{name}` is invited, as an agent this factory does not start. Its invitation, shown this once:"),
            format!("  {}", text(&invited, "invitation")),
            "Whoever is to be that agent runs, where its program runs,".to_string(),
            format!("  {cli} agent join --url {}", client.url),
            "and gives it the invitation when it asks: read so, it is in no shell's history.".to_string(),
            format!(
                "It lets that agent in once, within the next {}. Pass it on as you would a password.",
                within(invited["expires"].as_i64().unwrap_or(0), now_ms())
            ),
            format!("Joined, its commands are asked as it: `{cli} --as {name} --url {} ...`", client.url),
            format!("It holds: {}", holds.join(", ")),
        ]
        .join("\n")
    });
    Ok(())
}

pub async fn join(client: &Client, args: &Args) -> Result<(), String> {
    // Given as a word; or, so that it need be in no shell's history, on
    // standard input: `agent join -`, or with nothing after it.
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
        return Err("an invitation is needed: give it after `agent join`, or on standard input".into());
    }
    let joined = client.post("/api/agent/join", json!({ "invitation": invitation })).await?;
    let (name, token) = (text(&joined, "agent"), text(&joined, "token"));
    if name.is_empty() || token.is_empty() {
        return Err(format!("what answers at {} did not answer as a factory lets an agent in", client.url));
    }
    // Kept before anything is said: the credential is answered once.
    let kept = crate::joined::keep(name, &client.url, token)?;
    // Said without the credential, as JSON too.
    let said = json!({
        "agent": name,
        "url": client.url,
        "scopes": joined["scopes"],
        "project": joined["project"],
        "kept": kept,
    });
    show(client, &said, || {
        let cli = brand::CLI;
        let holds: Vec<&str> = list(&joined["scopes"]).iter().filter_map(Value::as_str).collect();
        [
            format!("joined {} as the agent `{name}`", client.url),
            format!(
                "Its credential is kept in {}, which only you can read; it is not shown.",
                kept.display()
            ),
            format!("A command is asked as it with `--as`: `{cli} --as {name} --url {} whoami`", client.url),
            format!(
                "(or with `{}={name}` in the environment). It holds: {}",
                brand::env_var("AGENT"),
                holds.join(", ")
            ),
        ]
        .join("\n")
    });
    Ok(())
}

pub async fn leave(client: &Client, _args: &Args) -> Result<(), String> {
    let Some(name) = &client.agent else {
        return Err(format!(
            "which agent leaves is said by its name: `{} --as <agent> --url <address> agent leave`",
            brand::CLI
        ));
    };
    // The factory is told first: the credential is nobody's from then,
    // whatever is kept of it anywhere. One the factory no longer takes
    // (it was stopped, or invited again and joined elsewhere) is
    // forgotten here all the same.
    match client.post("/api/agent/leave", Value::Null).await {
        Ok(_) => {
            crate::joined::forget(name, &client.url);
            println!(
                "`{name}` has left {}: its credential is nobody's, and is forgotten here. It is let in again by \
                 an invitation",
                client.url
            );
        }
        // Nothing was withdrawn, so nothing is forgotten: asked again
        // once the factory answers, the credential is still here to
        // give up.
        Err(why) if Client::is_unreachable(&why) => {
            return Err(format!("{why}. Nothing was withdrawn there, and the credential is still kept here"));
        }
        Err(why) => {
            crate::joined::forget(name, &client.url);
            println!(
                "the factory at {} would not take `{name}`'s credential ({why}); what was kept here is forgotten",
                client.url
            );
        }
    }
    Ok(())
}
