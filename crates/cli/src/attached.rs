//! What is attached to a record, a card included: a file attached, what a
//! record carries, one taken away; and anything attached read, to a record
//! or to a message, as whoever asks may read what it is attached to.

use std::io::Write;

use reqwest::Method;
use serde_json::Value;

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{show, text};

/// How big something is, in words.
fn sized(size: u64) -> String {
    match size {
        size if size >= 1024 * 1024 => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
        size if size >= 1024 => format!("{} KB", size.div_ceil(1024)),
        size => format!("{size} bytes"),
    }
}

/// `record attach`, `record attachments` and `record detach`.
pub async fn record(client: &Client, args: &Args, doing: &str) -> Result<(), String> {
    let id = args.need(2, "the record")?;
    let at = format!("/api/records/{}/attachments", encode(id));
    match doing {
        "attach" => {
            let files = args.words_from(3);
            if files.is_empty() {
                return Err(format!(
                    "missing what to attach: `{} record attach <record> <file>...`",
                    brand::CLI
                ));
            }
            for file in &files {
                let bytes =
                    std::fs::read(file).map_err(|err| format!("cannot read {file} to attach it: {err}"))?;
                let name =
                    std::path::Path::new(file).file_name().map(|name| name.to_string_lossy().into_owned());
                let path = format!("{at}?name={}", encode(&name.unwrap_or_else(|| file.to_string())));
                let kept = client.upload(&path, bytes).await?;
                show(client, &kept, || {
                    let size = sized(kept["size"].as_u64().unwrap_or_default());
                    format!("attached {} ({size}) to {id} as {}", text(&kept, "name"), text(&kept, "id"))
                });
            }
        }
        "detach" => {
            let which = args.need(3, "what is attached, by its id")?;
            let gone = client.call(Method::DELETE, &format!("{at}/{}", encode(which)), None).await?;
            let name = text(&gone["attachment"], "name");
            show(client, &gone, || match gone["trash"].as_str() {
                Some(trash) => format!(
                    "moved {name} of {id} to the Trash as `{trash}`; `{} trash restore {trash}` gives it back",
                    brand::CLI
                ),
                None => format!("took {name} away from {id}"),
            });
        }
        _ => {
            let listed = client.get(&at).await?;
            show(client, &listed, || {
                let rows: Vec<Vec<String>> = listed["attachments"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|one| {
                        vec![
                            text(one, "id").to_string(),
                            text(one, "name").to_string(),
                            sized(one["size"].as_u64().unwrap_or_default()),
                            text(one, "who").to_string(),
                        ]
                    })
                    .collect();
                let head = ["id", "name", "size", "by"].map(str::to_string).to_vec();
                match rows.is_empty() {
                    true => format!("nothing is attached to {id}"),
                    false => table(&[vec![head], rows].concat()),
                }
            });
        }
    }
    Ok(())
}

/// `attachment <id> [--out FILE]`: what is attached, to a record or to a
/// message, written to standard output or to a file.
pub async fn attachment(client: &Client, args: &Args) -> Result<(), String> {
    let out = args.value("out").map(str::to_string);
    let id = args.need(1, "what is attached, by its id")?;
    let bytes = client.bytes(&format!("/api/attachments/{}", encode(id))).await?;
    match out {
        Some(file) => {
            std::fs::write(&file, &bytes).map_err(|err| format!("cannot write {file}: {err}"))?;
            show(client, &Value::Null, || format!("wrote {} to {file}", sized(bytes.len() as u64)));
        }
        None => {
            let mut out = std::io::stdout().lock();
            out.write_all(&bytes)
                .and_then(|()| out.flush())
                .map_err(|err| format!("writing it out failed: {err}"))?;
        }
    }
    Ok(())
}
