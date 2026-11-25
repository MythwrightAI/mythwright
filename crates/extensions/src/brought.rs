//! The blueprints a package brings.
//!
//! A package says how somebody's product is run. Where that product is set
//! up rather than talked to (a way of being reached, a database), the saying
//! is a blueprint, and the package carries it as text in its one file. While
//! the package is installed its blueprints are in the library under
//! `<package>/<name>`, to be applied and called like any other; when it is
//! removed they are gone again. They are given nothing for coming from a
//! package: an instance of one has the authority of whoever applied it.
//!
//! This is what a manifest says of them, read and checked as the package
//! is read; the layer of the library they are found in is the agents'
//! (`agents::brought::Brought`), since which packages are installed is.

use std::collections::BTreeMap;
use std::sync::Arc;

use blueprint::{ActionList, ActionSpec, Catalog, expr};
use serde_json::Value;

use crate::declared::Declared;
use crate::extension::Check;

/// What a package's blueprints are checked against while the package is
/// read: the actions the package itself declares, and nothing else. Which
/// other actions there are is a question for the device a blueprint is
/// applied on, and is asked there.
struct Own<'a> {
    package: &'a str,
    actions: ActionList,
}

impl Catalog for Own<'_> {
    fn action(&self, name: &str) -> Option<&ActionSpec> {
        self.actions.action(name)
    }

    fn action_names(&self) -> Vec<&str> {
        self.actions.action_names()
    }

    fn is_complete(&self) -> bool {
        false
    }

    /// An action under the package's own name that the package does not
    /// declare will not be there on any device.
    fn absent(&self, name: &str) -> Option<String> {
        let (package, action) = name.split_once('.')?;
        if package != self.package {
            return None;
        }
        let has = self.action_names();
        Some(if has.is_empty() {
            format!("`{name}` is not declared: this package has no `actions`")
        } else {
            format!("this package declares no action called `{action}`; it has: {}", has.join(", "))
        })
    }
}

/// Reads what a manifest says under `blueprints`: each name and its text,
/// checked as a blueprint is checked. What it comes to is kept under the
/// names the library knows them by.
pub fn read(
    check: &mut Check,
    package: &str,
    value: Option<&Value>,
    actions: &BTreeMap<String, Arc<Declared>>,
) -> BTreeMap<String, String> {
    let own = Own {
        package,
        actions: ActionList::new(actions.values().map(|action| action.spec.clone()).collect()),
    };
    let mut brought = BTreeMap::new();
    let entries = match value {
        None | Some(Value::Null) => return brought,
        Some(Value::Object(entries)) => entries,
        Some(_) => {
            check.problem("blueprints", "must be a mapping from a name to a blueprint's text");
            return brought;
        }
    };
    if entries.is_empty() {
        check.problem("blueprints", "is empty: leave it out if the package brings none");
    }
    for (key, text) in entries {
        let at = format!("blueprints.{key}");
        if key.is_empty() || *key != expr::slug(key) {
            check.problem(&at, "must be named in lowercase letters, digits and dashes");
            continue;
        }
        let Some(text) = text.as_str() else {
            check.problem(&at, "must be a blueprint's text, written as a block after `|`");
            continue;
        };
        let name = format!("{package}/{key}");
        let checked = blueprint::check(text, &own);
        for wrong in checked.errors() {
            check.problem(&at, format!("does not check: line {}: {}", wrong.span.start.line, wrong.message));
        }
        // Looked up by one name and calling itself another, it would be
        // filed under the second when applied.
        if let Some(read) = &checked.blueprint
            && read.name != name
        {
            check.problem(&at, format!("must call itself `{name}`, not `{}`", read.name));
        }
        brought.insert(name, text.to_string());
    }
    brought
}

/// Reads what a manifest says under `prompts`: the files its blueprints
/// link (`from:` of `text.put`), each by its name within `prompts/` and
/// its text. A package is one file, so what its blueprints would have
/// beside them in a folder is written here. They are kept under the names
/// the library finds them by, `<package>/prompts/<file>`, and come and go
/// with the package as its blueprints do. Whether one has the blanks a
/// blueprint hands is asked where it is linked, when that is applied.
pub fn linked(check: &mut Check, package: &str, value: Option<&Value>) -> BTreeMap<String, String> {
    let mut brought = BTreeMap::new();
    let entries = match value {
        None | Some(Value::Null) => return brought,
        Some(Value::Object(entries)) => entries,
        Some(_) => {
            check.problem("prompts", "must be a mapping from a file's name to its text");
            return brought;
        }
    };
    for (key, text) in entries {
        let at = format!("prompts.{key}");
        let name = format!("{package}/{}/{key}", conductor::words::FOLDER);
        if !conductor::library::is_safe_name(&name) || !conductor::library::is_linked(&name) {
            let kind = conductor::words::KIND;
            check.problem(&at, format!("must be the name of a file that ends `{kind}`, such as `ask{kind}`"));
            continue;
        }
        let Some(text) = text.as_str() else {
            check.problem(&at, "must be the file's text, written as a block after `|`");
            continue;
        };
        brought.insert(name, text.to_string());
    }
    brought
}

/// Holds each file a package's blueprints link to being there and to
/// having its blanks handed, when the package is read: what is asked of a
/// project's blueprint when it is applied is asked of a package's when
/// the package is added, since nobody can put the file right afterwards.
pub fn fitted(check: &mut Check, blueprints: &BTreeMap<String, String>, prompts: &BTreeMap<String, String>) {
    for (name, text) in blueprints {
        let Some(blueprint) = blueprint::check(text, &blueprint::NoCatalog).blueprint else { continue };
        let key = name.rsplit('/').next().unwrap_or(name);
        for link in conductor::words::linked(&blueprint) {
            let at = format!("blueprints.{key}");
            let file = conductor::words::linked_as(name, link.from);
            let Some(words) = prompts.get(&file) else {
                check.problem(
                    &at,
                    format!(
                        "links `{}` (its step `{}`), which the package does not bring: write it under \
                         `prompts:`, as `{}: |` and its text",
                        link.from, link.step, link.from
                    ),
                );
                continue;
            };
            let handed: Vec<&str> = link
                .with
                .as_object()
                .map(|with| with.keys().map(String::as_str).collect())
                .unwrap_or_default();
            if let Some(why) = conductor::words::unfit(words, &handed) {
                check.problem(&at, format!("links `{}` (its step `{}`): {why}", link.from, link.step));
            }
        }
    }
}
