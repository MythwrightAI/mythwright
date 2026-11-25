//! A project's files, written, read and taken away from a terminal, as the
//! client does through the same routes (`/api/projects/<id>/raw` and
//! `/file`): as whoever is signed in, or an agent by `--as`, held to
//! what the daemon holds anybody to, a file a table keeps private
//! included. What is written and read is bytes, so a picture arrives as
//! it was, and a file taken away goes where the factory keeps what is
//! deleted. A folder is made empty (`mkdir`), as the client makes one.

use std::io::{IsTerminal, Read, Write};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{show, text};

pub async fn file(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    let at = |project: &str, path: &str, route: &str| {
        format!("/api/projects/{}/{route}?path={}", encode(project), encode(path))
    };
    match args.word(1) {
        Some("write") => {
            let (project, path) = (args.need(2, "the project")?, args.need(3, "the file's path in it")?);
            let bytes = written_from(args, project, path)?;
            let size = bytes.len();
            let written = client.put_bytes(&at(project, path, "raw"), bytes).await?;
            show(client, &written, || {
                format!(
                    "wrote `{}` in {project} ({size} byte{})",
                    text(&written, "path"),
                    if size == 1 { "" } else { "s" }
                )
            });
        }
        Some("read") => {
            let (project, path) = (args.need(2, "the project")?, args.need(3, "the file's path in it")?);
            if client.json {
                let read = client.get(&at(project, path, "file")).await?;
                show(client, &read, String::new);
                return Ok(());
            }
            let bytes = client.bytes(&at(project, path, "raw")).await?;
            let mut out = std::io::stdout().lock();
            out.write_all(&bytes)
                .and_then(|()| out.flush())
                .map_err(|err| format!("writing it out failed: {err}"))?;
        }
        Some("remove") => {
            let (project, path) = (args.need(2, "the project")?, args.need(3, "the file's path in it")?);
            let gone = client.call(reqwest::Method::DELETE, &at(project, path, "file"), None).await?;
            show(client, &gone, || match (gone["trash"].as_str(), gone["kept"].as_u64()) {
                (Some(trash), days) => format!(
                    "moved `{path}` of {project} to the Trash as `{trash}`; {}, and `{cli} trash restore {trash}` \
                     gives it back where it was",
                    days.map_or("it is kept until it is deleted for good".to_string(), |days| format!(
                        "it is kept {days} day{}",
                        if days == 1 { "" } else { "s" }
                    )),
                ),
                (None, _) => {
                    format!("removed `{path}` of {project}; nothing deleted is kept in the Trash here")
                }
            });
        }
        Some("mkdir") => {
            let (project, path) = (args.need(2, "the project")?, args.need(3, "the folder's path in it")?);
            let made = client
                .post(
                    &format!("/api/projects/{}/folders", encode(project)),
                    serde_json::json!({ "path": path }),
                )
                .await?;
            show(client, &made, || format!("made the folder `{}` in {project}", text(&made, "path")));
        }
        Some(other) => {
            return Err(format!("`file {other}` is not a command; `{cli} help file` says what is"));
        }
        None => {
            return Err(format!("`file` needs write, read, remove or mkdir; `{cli} help file` says each"));
        }
    }
    Ok(())
}

/// What is to be written: the file `--from` names, else what comes on
/// standard input, read to its end. A terminal is no file: nothing typed
/// at one is taken for what to write.
fn written_from(args: &Args, project: &str, path: &str) -> Result<Vec<u8>, String> {
    if let Some(from) = args.value("from") {
        return std::fs::read(from).map_err(|err| format!("`{from}` could not be read: {err}"));
    }
    let mut input = std::io::stdin().lock();
    if input.is_terminal() {
        return Err(format!(
            "say what to write: `--from <file>`, or the file on standard input (`{} file write {project} {path} \
             < <file>`)",
            brand::CLI
        ));
    }
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).map_err(|err| format!("reading standard input failed: {err}"))?;
    Ok(bytes)
}
