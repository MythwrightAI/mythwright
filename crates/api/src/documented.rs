//! What the program says of itself, written out as pages: handed what this
//! program has (`manual::pages`), and put in a folder.
//!
//! Two things write them, and they come to the same pages. The command
//! (`docs export <folder>`) asks no daemon: it starts a factory of its own
//! in a folder nobody else has, with nobody in it and nothing installed,
//! for as long as it takes to read what that factory's actions and events
//! say of themselves ([`alone`]), and puts the pages in the folder it was
//! told ([`put_in`]). The action (`docs.export`) is a daemon's, and writes
//! into a folder of the project it is applied for, as whoever applied it
//! ([`register`]). Either way what is described is what this program
//! brings, never what a package installed here brings, so that two copies
//! of one version write the same.
//!
//! The folder is replaced as a whole and only where an export wrote what
//! is in it, as a library written out as pages is (`studio::site::put`).

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::Path;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionResult, Library, Registry};
use kernel::{BoxFut, Error, Result};
use serde_json::{Value, json};
use studio::Studio;

use crate::{Config, Node};

/// A whole blueprint that asks it, for the Reference and the editor.
const ASKED: &str = "\
blueprint: reference
description: Write what the program says of itself into the project's library, for its site.
permissions: [projects]

steps:
  written:
    do: docs.export
    with:
      to: docs/reference
";

/// Every page, by where it is, made from what `registry` says of its
/// actions and events and from the blueprints that ship.
pub fn pages(registry: &Registry) -> BTreeMap<String, String> {
    let mut actions = blueprint::intrinsics();
    actions.extend(registry.specs().specs().iter().cloned());
    let events = registry.events();
    let library = conductor::standard();
    let blueprints: Vec<(String, String)> = library
        .names(None)
        .into_iter()
        .filter(|name| name.starts_with("std/") && !conductor::library::is_linked(name))
        .filter_map(|name| library.load(&name, None).map(|text| (name, text)))
        .collect();
    let device = manual::pages::Device { actions: &actions, events: events.specs(), blueprints: &blueprints };
    manual::pages::pages(&device)
}

/// The pages, with no daemon: a factory is started in a folder of its own
/// under the system's folder for such things, listening only on this
/// machine, read, stopped and taken away.
pub async fn alone() -> Result<BTreeMap<String, String>> {
    let dir = std::env::temp_dir().join(format!("{}-docs-{}", brand::CLI, &kernel::id::secret()[..12]));
    let config = Config {
        listen: SocketAddr::from(([127, 0, 0, 1], 0)),
        device: Some("docs".into()),
        // Nothing is started in it, so nothing is walled.
        walls: Some(crate::Walls::Open),
        disk: journal::Disk::Unwaited,
        ..Config::new(dir.join("data"))
    };
    let made = match Node::start(config).await {
        Ok(node) => {
            let made = pages(node.app.engine.registry());
            let left = node.stop().await;
            match left.first() {
                Some(why) => {
                    Err(Error::internal(format!("the factory the pages were read from did not stop: {why}")))
                }
                None => Ok(made),
            }
        }
        Err(err) => Err(err.context("starting a factory of its own to read what this program has")),
    };
    let _ = std::fs::remove_dir_all(&dir);
    made
}

/// Puts `pages` in the folder `to`, replacing it only where an export
/// wrote what is in it. Says whether anything was written.
pub fn put_in(to: &Path, pages: &BTreeMap<String, String>) -> Result<bool> {
    let files: BTreeMap<String, Vec<u8>> =
        pages.iter().map(|(path, text)| (path.clone(), text.clone().into_bytes())).collect();
    let name = to.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    if name.is_empty() || name == ".." {
        return Err(Error::invalid(format!(
            "`{}` is not a folder the pages can be written to: say one by its name, such as `site/reference`",
            to.display()
        )));
    }
    let parent = match to.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => std::path::PathBuf::from("."),
    };
    std::fs::create_dir_all(&parent).map_err(|err| Error::from(err).context(parent.display()))?;
    studio::site::put(&parent, &name, &files)
}

/// Writes the pages into the folder `to` of `project`, as `by`, who may
/// write the project's files: a folder of the project as a library's is,
/// never the factory's place for its own blueprints.
async fn export(
    studio: &Studio,
    registry: &Registry,
    project: &str,
    to: &str,
    by: &kernel::Principal,
) -> Result<Value> {
    by.require("projects:write")?;
    let wanted = kernel::addressed::parts(to.trim())
        .filter(|parts| !parts.is_empty() && !parts[0].starts_with('@'))
        .ok_or_else(|| {
            Error::invalid(format!(
                "`{to}` is not a folder of the project the pages can be written to: say one from the top of the \
                 project, such as `docs/reference`"
            ))
        })?;
    let to = wanted.join("/");
    // What a table keeps private is not written over by whoever may not read it.
    studio.projects.may_read_path(project, &to, by).await?;
    let dir = studio
        .projects
        .dir_of(project)
        .ok_or_else(|| Error::not_found(format!("there is no project `{project}`")))?;
    let made = pages(registry);
    let files: BTreeMap<String, Vec<u8>> =
        made.iter().map(|(path, text)| (path.clone(), text.clone().into_bytes())).collect();
    let into = to.clone();
    let changed = tokio::task::spawn_blocking(move || studio::site::put(&dir, &into, &files))
        .await
        .map_err(|err| Error::internal(format!("writing the pages out crashed: {err}")))??;
    if changed {
        // What was written is files of the project like any other, and is
        // said to have changed the way any are.
        let _ = studio.projects.scan(project).await;
    }
    Ok(json!({ "to": to, "pages": made.len(), "version": manual::VERSION, "changed": changed }))
}

/// Adds `docs.export` to what a blueprint can do here.
pub(crate) fn register(registry: &mut Registry, studio: Studio) {
    let spec = ActionSpec::new(
        "docs.export",
        "Write everything this program says of itself into a folder of the project, as pages a library reads: its commands, its language, its actions and events, the blueprints that ship and its guides.",
    )
    .example(ASKED)
    .making("the pages of what the program says of itself, in the folder", "to")
    .required(
        "to",
        T::String,
        "the folder of the project it is written to, from the top of the project, replaced as a whole and only where an export wrote what is in it",
    )
    .arg("project", T::String, "which project (default: this instance's)")
    .capability("projects:write")
    .output("to", "the folder it was written to, from the top of the project")
    .output("pages", "how many pages")
    .output("version", "the version of the program that wrote them, which every page says")
    .output("changed", "whether anything was written; what is already as it would be written is left alone")
    .idempotent();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let studio = studio.clone();
        Box::pin(async move {
            let said = |name: &str| args.get(name).and_then(Value::as_str).map(str::to_string);
            let project = said("project")
                .filter(|project| !project.is_empty())
                .or_else(|| cx.project().map(str::to_string))
                .ok_or_else(|| {
                    Error::invalid("which project? give `project`, or apply this blueprint within one")
                })?;
            let to = said("to").unwrap_or_default();
            Ok(export(&studio, cx.engine().registry(), &project, &to, cx.principal()).await?)
        })
    });
}
