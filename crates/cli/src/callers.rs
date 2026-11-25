//! Callers: the services outside that sign what they send.
//!
//! One is described here once, by the owner: the topic its calls are said
//! as, which secret it signs with, and where in a call its signature is.
//! The secret itself is set as any other is (`secret set`), and is named
//! here, never given.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

/// What is said of where a caller puts things, as the daemon is told it:
/// each flag that was given, under the name the daemon knows it by.
fn described(args: &Args) -> Result<Value, String> {
    let cli = brand::CLI;
    let need = |flag: &str, what: &str| {
        args.value(flag)
            .ok_or_else(|| format!("missing `--{flag}`: {what}. `{cli} help caller` shows one whole"))
    };
    let mut described = json!({
        "topic": need("topic", "the topic its calls are said as, such as `shop.paid`")?,
        "secret": need("secret", "the name of the secret it signs with")?,
        "header": need("header", "the header its signature is sent in")?,
    });
    for (flag, key) in [
        ("prefix", "prefix"),
        ("encoding", "encoding"),
        ("key", "key"),
        ("signed", "signed"),
        ("time-header", "time_header"),
        ("time-prefix", "time_prefix"),
        ("within", "within"),
    ] {
        if let Some(value) = args.value(flag) {
            described[key] = json!(value);
        }
    }
    Ok(described)
}

/// `caller list | set <name> --topic T --secret S --header H … | remove <name>`.
pub async fn caller(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/callers").await?;
            show(client, &found, || {
                let rows: Vec<Vec<String>> = list(&found["callers"])
                    .iter()
                    .map(|caller| {
                        let timed = !text(caller, "time_header").is_empty()
                            || !text(caller, "time_prefix").is_empty();
                        vec![
                            text(caller, "name").into(),
                            format!("says {}", text(caller, "topic")),
                            format!("signs with {}", text(caller, "secret")),
                            match timed {
                                true => format!("good for {}", text(caller, "within")),
                                false => "says no time".into(),
                            },
                            format!("{}/in/{}", client.url, text(caller, "name")),
                        ]
                    })
                    .collect();
                if rows.is_empty() {
                    format!(
                        "no service outside is let in by its signature. `{cli} help caller` says how one is"
                    )
                } else {
                    table(&rows)
                }
            });
            Ok(())
        }
        Some("set") => {
            let name = args.need(2, "the caller's name, such as `payments`")?;
            let path = format!("/api/callers/{}", encode(name));
            let made = client.call(reqwest::Method::PUT, &path, Some(described(args)?)).await?;
            show(client, &made, || {
                format!(
                    "{name} is let in at {}{}\nGive the service that address. What it sends there is said as `{}` \
                     when it is signed with `{}`, and refused when it is not.",
                    client.url,
                    text(&made, "address"),
                    text(&made, "topic"),
                    text(&made, "secret"),
                )
            });
            Ok(())
        }
        Some("remove") => {
            let name = args.need(2, "the caller's name")?;
            client.call(reqwest::Method::DELETE, &format!("/api/callers/{}", encode(name)), None).await?;
            println!("{name} is gone, and its address answers nobody");
            Ok(())
        }
        Some(other) => Err(format!(
            "`caller {other}` is not a command; there are `list`, `set <name>` and `remove <name>`"
        )),
    }
}
