//! The Trash: what was deleted, given back, deleted for good, and how
//! long what is deleted is kept (`studio::trash`).

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, encode};
use crate::work::{list, show, text};

/// When something goes for good, as a day and a time; never, where
/// what is deleted is kept until somebody deletes it for good.
fn goes(entry: &Value) -> String {
    match entry["goes"].as_i64() {
        Some(at) => format!("goes {}", kernel::clock::rfc3339(at)),
        None => "kept until deleted for good".to_string(),
    }
}

/// Who deleted it, by name as the page says them, with who that is only
/// where two who deleted something here share the name.
fn by(entry: &Value, all: &[Value]) -> String {
    let (who, actor) = (text(entry, "who"), text(entry, "by"));
    let shared = all.iter().any(|other| text(other, "who") == who && text(other, "by") != actor);
    match (who.is_empty(), shared) {
        (true, _) => actor.to_string(),
        (false, true) => format!("{who} ({actor})"),
        (false, false) => who.to_string(),
    }
}

fn printed(entry: &Value, all: &[Value]) -> String {
    let of = match text(entry, "kind") {
        // A channel is the factory's, whatever project it says.
        "project" | "channel" => String::new(),
        _ => format!(" of {}", text(entry, "project")),
    };
    // What went in with it: the pages under a page.
    let with = match entry["with"].as_u64() {
        Some(1) => ", with the record under it".to_string(),
        Some(with) => format!(", with the {with} records under it"),
        None => String::new(),
    };
    format!(
        "{}  {} `{}`{of}{with} ({})  by {}  {}",
        text(entry, "id"),
        text(entry, "kind"),
        text(entry, "called"),
        text(entry, "thing"),
        by(entry, all),
        goes(entry)
    )
}

fn days_said(found: &Value) -> String {
    match found["days"].as_u64() {
        Some(0) => "what is deleted goes at once, and nothing is kept in the Trash".to_string(),
        Some(1) => "what is deleted is kept in the Trash for a day".to_string(),
        Some(days) => format!("what is deleted is kept in the Trash for {days} days"),
        None => "what is deleted is kept in the Trash until somebody deletes it for good".to_string(),
    }
}

pub async fn trash(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    if let Some(keep) = args.value("keep") {
        let days = match keep.parse::<u64>() {
            Ok(days) => json!(days),
            Err(_) if keep == "always" => json!("always"),
            Err(_) => {
                return Err(format!("`--keep` is a number of days, or `always`, and `{keep}` is neither"));
            }
        };
        let said = client.post("/api/system/trash", json!({ "days": days })).await?;
        show(client, &said, || format!("{}; {} went for good now", days_said(&said), said["gone"]));
        return Ok(());
    }
    match args.word(1) {
        None | Some("list") => {
            let found = client.get("/api/trash").await?;
            show(client, &found, || {
                let all = list(&found["trash"]);
                let lines: Vec<String> = all.iter().map(|entry| printed(entry, all)).collect();
                match lines.is_empty() {
                    true => format!("the Trash is empty; {}", days_said(&found)),
                    false => format!("{}\n{}", lines.join("\n"), days_said(&found)),
                }
            });
        }
        Some("restore") => {
            let id = args.need(2, "what to give back, by its id as the Trash lists it")?;
            let back = client
                .post(&format!("/api/trash/{}/restore", encode(id)), json!({ "beside": args.has("beside") }))
                .await?;
            show(client, &back, || {
                let with = match back["restored"]["with"].as_u64() {
                    Some(1) => ", with the record that went with it".to_string(),
                    Some(with) => format!(", with the {with} records that went with it"),
                    None => String::new(),
                };
                let said = back["said"].as_str().map(|said| format!(". {said}")).unwrap_or_default();
                format!(
                    "gave back {} `{}`{with}: it is at `{}`{said}",
                    text(&back["restored"], "kind"),
                    text(&back["restored"], "called"),
                    text(&back, "at")
                )
            });
        }
        Some("forget") => {
            let id = args.need(2, "what to delete for good, by its id as the Trash lists it")?;
            let gone =
                client.call(reqwest::Method::DELETE, &format!("/api/trash/{}", encode(id)), None).await?;
            show(client, &gone, || format!("deleted `{}` for good", text(&gone["gone"], "called")));
        }
        Some(other) => {
            return Err(format!(
                "`trash {other}` is not a command; try `{cli} trash`, `{cli} trash restore <id> [--beside]`, \
                 `{cli} trash forget <id>` or `{cli} trash --keep DAYS|always`"
            ));
        }
    }
    Ok(())
}
