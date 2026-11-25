//! A library as pages: the ones the daemon serves, and the same written
//! out as plain files that anything can serve.
//!
//! How a page is made, a library's contents beside it and the folder of
//! pages built and put in place are the `pages` crate's
//! (`::pages::site`), which knows no table, and are offered here under
//! the names they had. What is the studio's is what a document is called,
//! as its library reads it, and the asking: [`export`], which says who may
//! write a library out and where, and what was written.

use std::path::Path;
use std::time::Instant;

use kernel::{Error, Principal, Result, Source};
use serde_json::{Value, json};

pub use ::pages::site::{
    Exported, FRONTS, LeftOut, Listed, Shelf, TO_CONTENTS, WRITTEN, listing, navigation, put,
    served_navigation, way_up, written,
};
use ::pages::site::{build, shelf_from};
use kernel::addressed::parts;

use crate::documents::{folder, read};
use crate::{Collection, Store, Studio};

/// What a document of `collection` at `path` is called, as the library
/// reads it.
fn title_of(collection: &Collection, path: &str, text: &str) -> String {
    let record = read(collection, path, text, (0, 0), 0);
    record.data.get("title").and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The shelf of every document of a library as its files are now, as the
/// daemon serves it: a folder's front page as [`FRONTS`] says.
pub fn shelf_of(collection: &Collection, root: &Path) -> Shelf {
    shelf_from(root, &|path, text| title_of(collection, path, text))
}

/// One export at a time: two that wrote the same folder at once would each
/// put the other's half-built one in its place.
static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Writes the library `name` of `project` out as pages, to the folder `to`
/// of the project (`site`, unless said): the documents that say
/// `public: true` of themselves, or with `publish: all` every one.
pub async fn export(
    studio: &Studio,
    project: &str,
    name: &str,
    to: Option<&str>,
    publish: Option<&str>,
    by: &Principal,
    source: Source,
) -> Result<Exported> {
    by.require("records:read")?;
    by.require("projects:write")?;
    let all = match publish.map(str::trim).filter(|said| !said.is_empty()) {
        None | Some("marked") => false,
        Some("all") => true,
        Some(other) => {
            return Err(Error::invalid(format!(
                "`publish: {other}` is not a way to say which documents are written out; there are `marked` (the ones that say `public: true` of themselves) and `all`"
            )));
        }
    };
    let collection = studio.collections.get(project, name).await?;
    if collection.store != Store::Documents {
        return Err(Error::invalid(format!(
            "`{name}` in `{project}` is not a library: its records are not documents, and there are no pages of them to write out"
        )));
    }
    let to = to.map(str::trim).filter(|to| !to.is_empty()).unwrap_or("site");
    let refuse =
        |why: String| Error::invalid(format!("`{to}` is not where `{name}` can be written out: {why}"));
    let wanted = parts(to).ok_or_else(|| {
        refuse("say a folder as it is from the top of the project, like `site` or `out/pages`".into())
    })?;
    if wanted.is_empty() {
        return Err(refuse(
            "that is the project itself, and writing a library out replaces what is in the folder".into(),
        ));
    }
    // Not in a folder a library reads from, and not around one: the pages
    // would be taken for part of the library, or the library taken away.
    for library in studio.collections.list(Some(project)).await? {
        if library.store != Store::Documents {
            continue;
        }
        let its = parts(folder(&library)).unwrap_or_default();
        let (inside, around) = (wanted.starts_with(&its), its.starts_with(&wanted));
        if its.is_empty() {
            return Err(refuse(format!(
                "the library `{}` is the whole project, so every folder of the project is part of it. Give the library a folder of its own (`at`), and write it out beside that",
                library.name
            )));
        }
        if inside || around {
            return Err(refuse(format!(
                "it is {} the folder the library `{}` is the documents of (`{}`). Say a folder beside it",
                if inside { "in" } else { "around" },
                library.name,
                its.join("/")
            )));
        }
    }
    let to = wanted.join("/");
    let dir = studio
        .projects
        .dir_of(project)
        .ok_or_else(|| Error::not_found(format!("there is no project `{project}`")))?;
    let from = folder(&collection).to_string();
    let root = dir.join(&from);
    let called = match studio.projects.get(project).await {
        Ok(project) => format!("{} {}", project.name, collection.name),
        Err(_) => format!("{project} {}", collection.name),
    };

    let _one = ONE_AT_A_TIME.lock().await;
    let started = Instant::now();
    let (about, into, folder) = (collection.clone(), to.clone(), from.clone());
    let made = tokio::task::spawn_blocking(move || {
        // A library whose folder is not there has not become one with no
        // documents: what was written out before is left as it is.
        if !root.is_dir() {
            return Err(Error::not_found(format!(
                "the folder `{folder}` that `{}` is the documents of is not there, so there is nothing to write out; what was written out before is left as it is",
                about.name
            )));
        }
        let built = build(&root, &called, all, &|path, text| title_of(&about, path, text));
        let changed = put(&dir, &into, &built.files)?;
        Ok::<_, Error>((built.pages, built.files.len() - built.pages, changed, built.left_out))
    })
    .await
    .map_err(|err| Error::internal(format!("writing the library out crashed: {err}")))
    .and_then(|made| made);
    let took = started.elapsed().as_secs_f64() * 1000.0;
    studio.journal.measure(
        "library.export",
        &[("project", project), ("collection", name)],
        took,
        made.is_ok(),
    );
    let (pages, files, changed, left_out) = made?;
    if changed {
        let data = json!({
            "project": project, "name": name, "to": to, "pages": pages, "files": files,
            "left_out": left_out.len(),
        });
        let _ = studio.journal.emit("collection.exported", source, data).await;
        // What was written is files of the project like any other, and is
        // said to have changed the way any are.
        let _ = studio.projects.scan(project).await;
    }
    Ok(Exported { to, from, pages, files, changed, left_out })
}
