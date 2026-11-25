//! A record as a file: what is kept about it between two lines of dashes,
//! then its `title` as a heading and its `body` as the text under it. It
//! is written so that a person can read it and change it, and is read back
//! forgivingly: whatever of it makes sense is kept. A table kept as files
//! is the studio's (`Store::Files`), which hands what its fields are.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use crate::{Record, record_id};

/// The file of `record` of a table whose fields are `fields`, by name.
pub fn write(fields: &[String], record: &Record) -> String {
    let has = |name: &str| fields.iter().any(|field| field == name);
    let mut text = String::from("---\n");
    for field in fields.iter().filter(|field| *field != "title" && *field != "body") {
        let value = record.data.get(field).cloned().unwrap_or(Value::Null);
        text.push_str(&format!("{field}: {value}\n"));
    }
    text.push_str(&format!("position: {}\n", record.position));
    text.push_str(&format!("created_by: {}\n", json!(record.created_by)));
    text.push_str(&format!("created: {}\nupdated: {}\n---\n", record.created, record.updated));
    let text_of = |name: &str| record.data.get(name).and_then(Value::as_str).unwrap_or_default();
    if has("title") {
        text.push_str(&format!("# {}\n", text_of("title")));
    }
    if has("body") && !text_of("body").is_empty() {
        text.push_str(&format!("\n{}\n", text_of("body").trim_end()));
    }
    text
}

/// The record numbered `number` of the table `collection` of `project`,
/// whose fields are `fields`, from its file.
pub fn read(project: &str, collection: &str, fields: &[String], number: i64, text: &str) -> Record {
    let has = |name: &str| fields.iter().any(|field| field == name);
    let (head, rest) = match text.strip_prefix("---\n").and_then(|after| after.split_once("\n---")) {
        Some((head, rest)) => (head, rest.trim_start_matches('\n')),
        None => ("", text),
    };
    // Each line is a name and a value. A value is JSON where it can be
    // read as that, and otherwise the words as they are.
    let kept: BTreeMap<&str, Value> = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| {
            let value = value.trim();
            (name.trim(), serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string())))
        })
        .collect();
    let mut data = Map::new();
    for field in fields {
        if let Some(value) = kept.get(field.as_str()) {
            data.insert(field.clone(), value.clone());
        }
    }
    let rest = rest.trim();
    let (title, body) = match rest.strip_prefix("# ") {
        Some(after) => match after.split_once('\n') {
            Some((title, body)) => (title.trim(), body.trim()),
            None => (after.trim(), ""),
        },
        None => ("", rest),
    };
    if has("title") {
        data.insert("title".into(), json!(title));
    }
    if has("body") {
        data.insert("body".into(), json!(body));
    }
    let number_of = |name: &str| kept.get(name).and_then(Value::as_i64).unwrap_or(0);
    Record {
        id: record_id(project, collection, number),
        project: project.to_string(),
        collection: collection.to_string(),
        number,
        data,
        position: kept.get("position").and_then(Value::as_f64).unwrap_or(number as f64),
        created_by: kept.get("created_by").and_then(Value::as_str).unwrap_or("system").to_string(),
        created: number_of("created"),
        updated: number_of("updated"),
    }
}
