//! Commands about the records of a collection a blueprint declared: a
//! project's sprints, its releases, whatever it keeps beside its board.

use reqwest::Method;
use serde_json::{Map, Value, json};

use crate::args::Args;
use crate::client::{Client, encode, table};
use crate::work::{list, show, text};

/// A value as it is shown: words as they are, anything else as it is written.
fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(shown).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

/// How something shown begins, where it is too long for a line of a list:
/// its first line, and of that no more than fits.
fn begins(shown: &str) -> String {
    const MOST: usize = 60;
    let first = shown.lines().next().unwrap_or_default();
    let cut: String = first.chars().take(MOST).collect();
    if cut.len() < shown.len() { format!("{}…", cut.trim_end()) } else { cut }
}

/// What a record holds, a field after another, leaving out what it has none of.
fn held(record: &Value) -> Vec<(String, String)> {
    record["data"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, value)| !value.is_null())
        .map(|(name, value)| (name.clone(), shown(value)))
        .collect()
}

/// What a project's collection is made of: its fields, each with its `kind`.
async fn fields_of(client: &Client, project: &str, collection: &str) -> Result<Vec<Value>, String> {
    let declared = client.get(&format!("/api/collections?project={}", encode(project))).await?;
    Ok(list(&declared["collections"])
        .iter()
        .find(|one| one["name"] == collection)
        .map(|one| list(&one["fields"]).to_vec())
        .unwrap_or_default())
}

/// What the words `name=value` say a record holds, each value made what
/// its field keeps: a number for a number, a list for a list. A field
/// that is not one of the collection's is sent as it was written, and
/// the daemon says which fields there are. `was` is what the record
/// holds now, where it is there already, which `name+=value` adds to.
fn written(
    args: &Args,
    from: usize,
    fields: &[Value],
    was: Option<&Value>,
) -> Result<Map<String, Value>, String> {
    let mut pairs = Vec::new();
    while let Some(pair) = args.word(from + pairs.len()) {
        pairs.push(pair);
    }
    if pairs.is_empty() {
        return Err("missing what it holds: one or more of `field=value`, as in `title=November`".into());
    }
    said(&pairs, fields, was)
}

/// The same, of a collection that is asked what it is made of first: for
/// a command that takes what a record holds some other way than as its
/// last words (`board add ... --field after=shop-1,shop-2`), or that
/// reads the record itself (`board set`).
pub async fn held_by(
    client: &Client,
    project: &str,
    collection: &str,
    pairs: &[&str],
    was: Option<&Value>,
) -> Result<Map<String, Value>, String> {
    said(pairs, &fields_of(client, project, collection).await?, was)
}

/// The things a list is written as after `=`: the words between commas.
pub fn between_commas(value: &str) -> Vec<Value> {
    value.split(',').map(str::trim).filter(|item| !item.is_empty()).map(|item| json!(item)).collect()
}

/// What a list comes to with `value` added to its end as one thing, as it
/// is written, commas and all (`name+=value`): after what was said of
/// the field before it among the same words, and where nothing was, after
/// what the record holds now (`was`, its fields), which is nothing for
/// one that is being added.
pub fn added(given: &Map<String, Value>, was: Option<&Value>, name: &str, value: &str) -> Value {
    let before = match given.get(name) {
        Some(said) => said.clone(),
        None => was.map(|was| was[name].clone()).unwrap_or(Value::Null),
    };
    let mut items = match before {
        Value::Array(items) => items,
        Value::Null => Vec::new(),
        one => vec![one],
    };
    if !value.is_empty() {
        items.push(json!(value));
    }
    Value::Array(items)
}

fn said(pairs: &[&str], fields: &[Value], was: Option<&Value>) -> Result<Map<String, Value>, String> {
    let mut given = Map::new();
    for pair in pairs {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("`{pair}` should be `field=value`, as in `title=November`"))?;
        let kind_of =
            |name: &str| fields.iter().find(|field| field["name"] == name).map(|field| text(field, "kind"));
        // One thing more of a list, whole: what holds a comma is said so.
        if let Some(name) = name.strip_suffix('+') {
            if kind_of(name) != Some("list") {
                return Err(format!(
                    "`{name}+=` adds one thing to a list, and `{name}` is no list of this table's: say                      `{name}=<value>`"
                ));
            }
            let value = added(&given, was, name, value);
            given.insert(name.to_string(), value);
            continue;
        }
        let value = match kind_of(name) {
            _ if value.is_empty() => Value::Null,
            Some("number") => {
                serde_json::from_str::<serde_json::Number>(value).map_or_else(|_| json!(value), Value::Number)
            }
            Some("flag") => value.parse::<bool>().map_or_else(|_| json!(value), Value::Bool),
            Some("list") => Value::Array(between_commas(value)),
            _ => json!(value),
        };
        given.insert(name.to_string(), value);
    }
    Ok(given)
}

pub async fn record(client: &Client, args: &Args) -> Result<(), String> {
    match args.word(1) {
        Some("list") => {
            let (project, collection) = (args.need(2, "the project")?, args.need(3, "the collection")?);
            let found = client
                .get(&format!("/api/collections/{}/{}/records", encode(project), encode(collection)))
                .await?;
            show(client, &found, || {
                let rows: Vec<Vec<String>> = list(&found["records"])
                    .iter()
                    .map(|record| {
                        // A line a record: of something long (a document's
                        // text), how it begins. `show` has the whole of it.
                        let said: Vec<String> = held(record)
                            .into_iter()
                            .map(|(name, value)| format!("{name} {}", begins(&value)))
                            .collect();
                        vec![text(record, "id").to_string(), said.join("  ")]
                    })
                    .collect();
                if rows.is_empty() { format!("`{collection}` has no records yet") } else { table(&rows) }
            });
        }
        Some("add") => {
            let (project, collection) = (args.need(2, "the project")?, args.need(3, "the collection")?);
            let fields = written(args, 4, &fields_of(client, project, collection).await?, None)?;
            let path = format!("/api/collections/{}/{}/records", encode(project), encode(collection));
            let added = client.post(&path, json!({ "fields": fields })).await?;
            show(client, &added, || {
                format!("added {} to {}", text(&added, "id"), text(&added, "collection"))
            });
        }
        Some("show") => {
            let id = args.need(2, "the record")?;
            let found = client.get(&format!("/api/records/{}", encode(id))).await?;
            show(client, &found, || {
                let mut lines = vec![format!("{}  of {}", text(&found, "id"), text(&found, "collection"))];
                lines.extend(held(&found).into_iter().map(|(name, value)| format!("  {name}: {value}")));
                lines.join("\n")
            });
        }
        Some("set") => {
            let id = args.need(2, "the record")?;
            let path = format!("/api/records/{}", encode(id));
            let was = client.get(&path).await?;
            let declared = fields_of(client, text(&was, "project"), text(&was, "collection")).await?;
            let fields = written(args, 3, &declared, Some(&was["data"]))?;
            let now = client.call(Method::PATCH, &path, Some(json!({ "fields": fields }))).await?;
            show(client, &now, || format!("changed {}", text(&now, "id")));
        }
        Some("remove") => {
            let id = args.need(2, "the record")?;
            let gone = client.call(Method::DELETE, &format!("/api/records/{}", encode(id)), None).await?;
            show(client, &gone, || match gone["with"].as_u64() {
                Some(1) => format!("took {id} away, with the record under it"),
                Some(with) => format!("took {id} away, with the {with} records under it"),
                None => format!("took {id} away"),
            });
        }
        Some(doing @ ("attach" | "attachments" | "detach")) => {
            crate::attached::record(client, args, doing).await?
        }
        Some("comment") => {
            let id = args.need(2, "the record")?;
            let said = args.rest(3);
            if said.is_empty() {
                return Err("missing what to say: `record comment <record> <words...>`".into());
            }
            // Private unless said: `--public` marks it, wherever among
            // the words it is written, and after `--` it is a word.
            let public = args.has("public");
            // About a few words of a document (`--on`), with what is either side of them and the headings over
            // them, which the daemon checks; without, it is about the whole record.
            let mut asked = json!({ "body": said, "public": public });
            if let Some(quote) = args.value("on") {
                let (before, after) = (args.value("before").unwrap_or(""), args.value("after").unwrap_or(""));
                let under = args.values("under");
                asked["anchor"] = json!({ "quote": quote, "before": before, "after": after, "under": under });
            }
            let path = format!("/api/records/{}/comments", encode(id));
            let on = client.post(&path, asked).await?;
            show(client, &on, || {
                format!("said {}on {}", if public { "publicly " } else { "" }, text(&on, "id"))
            });
        }
        Some("title") => {
            // What a table is called for people, for everybody; its name stays what everything knows it by.
            let (project, collection) = (args.need(2, "the project")?, args.need(3, "the table")?);
            let (said, usual) = (args.rest(4), args.has("usual"));
            if said.trim().is_empty() == !usual {
                return Err(format!(
                    "say what `{collection}` is to be called, or `--usual` to call it what its blueprint does again, \
                     not both: `record title {project} {collection} <words...>`"
                ));
            }
            let path = format!("/api/collections/{}/{}/title", encode(project), encode(collection));
            let table = match usual {
                true => client.call(Method::DELETE, &path, None).await?,
                false => client.call(Method::PUT, &path, Some(json!({ "title": said }))).await?,
            };
            show(client, &table, || {
                let called = table["title_chosen"].as_str().or(table["title"].as_str()).unwrap_or(collection);
                format!("`{collection}` of `{project}` is called {called} now; its name is as it was")
            });
        }
        Some(other) => {
            return Err(format!(
                "`record {other}` is not a command; try list, add, show, set, comment, attach, attachments, detach, title or remove"
            ));
        }
        None => {
            return Err("missing what to do: `record list <project> <collection>`, `record add <project> <collection> <field=value>...`, `record show <record>`, `record set <record> <field=value>...`, `record comment <record> <words...>`, `record title <project> <collection> <words...>` or `record remove <record>`".into());
        }
    }
    Ok(())
}
