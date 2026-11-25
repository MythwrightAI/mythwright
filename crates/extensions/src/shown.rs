//! What a package says it can show of a repository, beside what git says
//! of it: the parts of a project's Git page that are a service's (the
//! pull requests, the issues, the checks of the service a repository is
//! hosted on), each read by one of the package's own actions.
//!
//! The application knows no service. A package says, under `repository:`,
//! which repositories it serves, as a condition of each of their remotes
//! (`serves`, which reads `remote.name` and `remote.url`, the address as
//! it may be handed on), and each part it shows of one: a title, the
//! action of its own that reads it, which takes nothing it must be given
//! and gives out `rows`, the columns of a row that are shown, each a field
//! and its heading, and the field of a row that is an address to read it
//! at. The page asks which parts there are for a repository's remotes
//! ([`offered`]), and then each part's rows ([`read`]), which runs the
//! package's program as a step would run that action, in the project's
//! folder, with nothing handed to it: it acts as whoever runs the daemon,
//! signed in there or not, and what it says of not being signed in is the
//! package's own sentence. A row is handed on as the columns say and no
//! more: each cell plain text or a number, and the address only where it
//! is one a browser opens as a page (`http` or `https`).
//!
//! Nothing here is asked unless somebody asks for that part: reading a
//! part is what reaches the service, and the page does that only when it
//! is opened and when it is asked to read again.
//!
//! This is what a package says of it, and how a row is handed on; which
//! packages offer a part, and a part read for somebody, are the agents'
//! (`Agents::shown_for`, `Agents::shown`), since which are installed is.

use std::sync::Arc;
use std::time::Duration;

use blueprint::{Cond, expr};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::declared::Declared;
use crate::extension::Check;

/// How many rows of a part are handed on.
pub const MOST_ROWS: usize = 200;

/// How long a part's program is given.
pub const READING: Duration = Duration::from_secs(60);

/// What a package says it shows of the repositories it serves.
pub struct Shows {
    serves: Cond,
    pub parts: Vec<Part>,
}

/// One part it shows.
#[derive(Clone, Debug, Serialize)]
pub struct Part {
    /// What it is known by in the package, which the page asks it by.
    pub name: String,
    pub title: String,
    /// The action of the package that reads it.
    #[serde(skip)]
    pub action: String,
    /// Each column of a row: the field, and its heading.
    pub columns: Vec<Column>,
    /// The field of a row that is the address to read it at, where one is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Column {
    pub field: String,
    pub title: String,
}

/// A package's parts for a repository, as the page is offered them.
#[derive(Clone, Debug, Serialize)]
pub struct Offered {
    pub package: String,
    /// What the package is called where people are shown it.
    pub title: String,
    pub parts: Vec<Part>,
    /// Why whoever asked may not read them, where they may not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// What one part came to: its rows, each a field and what is shown of it.
#[derive(Clone, Debug, Serialize)]
pub struct Read {
    pub part: Part,
    pub rows: Vec<Map<String, Value>>,
    /// How many more rows the program gave than are handed on.
    pub more: usize,
}

/// Reads what a manifest says under `repository`.
pub fn read_shows(
    check: &mut Check,
    value: Option<&Value>,
    actions: &std::collections::BTreeMap<String, Arc<Declared>>,
) -> Option<Shows> {
    let value = value.filter(|value| !value.is_null())?;
    let top = check.fields("repository", value, &["serves", "parts"])?;
    let serves = check.cond("repository.serves", top.get("serves"));
    if serves.is_none() {
        check.problem("repository.serves", "is needed: the remotes it serves, as a condition of `remote`");
    }
    if let Some(serves) = &serves {
        for path in serves.expr.paths() {
            let fine = path[0] == "remote"
                && path.get(1).is_none_or(|member| ["name", "url"].contains(&member.as_str()));
            if !fine {
                check.problem("repository.serves", "reads only `remote.name` and `remote.url`");
            }
        }
    }
    let Some(Value::Array(listed)) = top.get("parts") else {
        check.problem(
            "repository.parts",
            "is needed: a list of what it shows, each a title, an action and its columns",
        );
        return None;
    };
    let mut parts = Vec::new();
    for (n, entry) in listed.iter().enumerate() {
        let at = format!("repository.parts[{n}]");
        let Some(fields) = check.fields(&at, entry, &["name", "title", "action", "columns", "link"]) else {
            continue;
        };
        let name = check.text(&format!("{at}.name"), fields.get("name"), true);
        let title = check.text(&format!("{at}.title"), fields.get("title"), true);
        let action = check.text(&format!("{at}.action"), fields.get("action"), true);
        let link = Some(check.text(&format!("{at}.link"), fields.get("link"), false))
            .filter(|link| !link.is_empty());
        if !name.is_empty() && name != expr::slug(&name) {
            check.problem(&format!("{at}.name"), "must be lowercase letters, digits and dashes");
        }
        if parts.iter().any(|part: &Part| part.name == name) {
            check.problem(&format!("{at}.name"), format!("is `{name}` a second time"));
        }
        match actions.get(&action) {
            None if !action.is_empty() => check.problem(
                &format!("{at}.action"),
                format!("names `{action}`, which is not an action of this package"),
            ),
            None => {}
            Some(declared) => {
                if let Some(needed) = declared.spec.args.iter().find(|arg| arg.required) {
                    check.problem(
                        &format!("{at}.action"),
                        format!(
                            "names `{action}`, which needs `{}`: a part's action is given nothing",
                            needed.name
                        ),
                    );
                }
                if !declared.gives_out("rows") {
                    check.problem(
                        &format!("{at}.action"),
                        format!("names `{action}`, which gives out no `rows`"),
                    );
                }
            }
        }
        let mut columns = Vec::new();
        match fields.get("columns") {
            Some(Value::Array(listed)) if !listed.is_empty() => {
                for (c, column) in listed.iter().enumerate() {
                    let at = format!("{at}.columns[{c}]");
                    let Some(column) = check.fields(&at, column, &["field", "title"]) else { continue };
                    let field = check.text(&format!("{at}.field"), column.get("field"), true);
                    let title = check.text(&format!("{at}.title"), column.get("title"), false);
                    let title = if title.is_empty() { field.clone() } else { title };
                    columns.push(Column { field, title });
                }
            }
            _ => check.problem(
                &format!("{at}.columns"),
                "is needed: a list of columns, each a `field` and its `title`",
            ),
        }
        parts.push(Part { name, title, action, columns, link });
    }
    if parts.is_empty() {
        check.problem("repository.parts", "is empty: leave `repository` out if the package shows nothing");
    }
    Some(Shows { serves: serves?, parts })
}

impl Shows {
    /// Whether the package serves a remote: what its `serves` says, and
    /// nothing where that cannot be worked out.
    pub fn serves(&self, name: &str, url: &str) -> bool {
        let scope = crate::extension::scope("remote", json!({ "name": name, "url": url }));
        self.serves.test(&scope).unwrap_or(false)
    }
}

/// A cell as it is handed on: text or a number, a list as its items
/// joined, and nothing else.
pub fn cell(value: &Value) -> Value {
    match value {
        Value::String(_) | Value::Number(_) => value.clone(),
        Value::Bool(yes) => json!(if *yes { "yes" } else { "no" }),
        Value::Array(items) => {
            let said: Vec<String> =
                items.iter().map(|item| expr::display(&cell(item))).filter(|item| !item.is_empty()).collect();
            json!(said.join(", "))
        }
        Value::Null => json!(""),
        Value::Object(_) => json!(expr::display(value)),
    }
}

/// An address a browser opens as a page, or nothing.
pub fn address(value: &Value) -> Option<Value> {
    let said = value.as_str()?.trim();
    let page =
        (said.starts_with("https://") || said.starts_with("http://")) && !said.contains(char::is_whitespace);
    page.then(|| json!(said))
}
