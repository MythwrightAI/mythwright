//! A library's documents: followed where they are, and served as pages.
//!
//! A library is a collection whose records are the Markdown files of a
//! folder (`studio::documents`). Two things about one are the node's:
//!
//! **Being told that a document changed.** Another program writes them: an
//! editor, an agent, whatever keeps the project's history. The folders of
//! every library are followed (`hosting::Watch`), and when the system says
//! something in one changed, the project's files are looked over once, as
//! they are after a run; what that finds is said the usual way
//! (`project.file.changed`), which is what the search and every open page
//! go by. Nothing here runs on a timer. Where the system cannot say, a
//! change is found when the project's files are next looked over.
//!
//! **Pages.** `/library/<project>/<name>/` is the library as web pages: a
//! front page, and each document as the page publishing would make of it,
//! with the links between them and the pictures beside them working as
//! they are written. They are for whoever may read the project's records
//! and for nobody else: nothing here is public. Whatever is asked for is
//! held inside the library's folder. Beside each page are the library's
//! contents in their order, as a library written out has them
//! (`studio::site::navigation`).
//!
//! A browser opens a page with its cookie, which it hands to every port
//! of the machine, and so the cookie reads nothing by itself: a page is
//! asked with what the client's page says beside it (`by`, `api::state`),
//! and every link it is written with that leads somewhere here says the
//! same (`said`, `addressed::saying`), so that a reader goes on from page
//! to page as they came. A page written out to a folder says nothing of
//! the kind.
//!
//! It came out of the daemon (`api::documents`, which keeps the routes,
//! each a call of this) when that crate was at its ceiling, handed the
//! studio, the journal and what follows files.

use std::path::{Path as FilePath, PathBuf};

use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, X_CONTENT_TYPE_OPTIONS};
use axum::response::{IntoResponse, Redirect, Response};
use hosting::Watch;
use journal::Journal;
use kernel::addressed::saying;
use kernel::{Error, Principal, Result};
use studio::documents::{ENDING, folder, folders};
use studio::pages::Leads;
use studio::projects::{MAX_FILE_BYTES, MAX_RAW_BYTES, resolve};
use studio::site::{FRONTS, Listed, Shelf, TO_CONTENTS, listing, served_navigation, shelf_of, written};
use studio::{Collection, Store, Studio, pages};

/// A page of a library is somebody's document, and may hold anything a
/// document can. It is shown and nothing in it runs: no script, nothing
/// fetched from anywhere but pictures from here.
pub const PAGE_POLICY: &str = "default-src 'none'; img-src 'self' data:; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'";

/// The libraries there are: each project that has one, and its folder.
async fn libraries(studio: &Studio) -> Vec<(String, PathBuf)> {
    let declared = studio.collections.list(None).await.unwrap_or_default();
    declared
        .iter()
        // A project's mockups are folders somebody else writes in too.
        .filter(|collection| matches!(collection.store, Store::Documents | Store::Mockups))
        .filter_map(|collection| Some((collection.project.clone(), root_of(studio, collection).ok()?)))
        .collect()
}

fn root_of(studio: &Studio, collection: &Collection) -> Result<PathBuf> {
    let dir = studio
        .projects
        .dir_of(&collection.project)
        .ok_or_else(|| Error::not_found(format!("there is no project `{}`", collection.project)))?;
    Ok(match folder(collection) {
        "" => dir,
        folder => dir.join(folder),
    })
}

/// Follows the folders of every library for as long as the daemon runs.
///
/// Which folders those are changes: a library is declared, a folder is
/// made inside one. So they are named again each time anything is heard,
/// and the project's files are looked over once then, which is also what
/// finds whatever changed before anyone was listening.
pub async fn follow(studio: &Studio, journal: &Journal, files: &Watch) {
    let mut declared = journal.follow_every(journal.now(), "collection.ensured,project.removed");
    loop {
        let libraries = libraries(studio).await;
        let roots: Vec<PathBuf> = libraries.iter().map(|(_, root)| root.clone()).collect();
        let listed = tokio::task::spawn_blocking(move || {
            roots.iter().filter(|root| root.is_dir()).flat_map(|root| folders(root)).collect::<Vec<_>>()
        });
        let followed = listed.await.unwrap_or_default();
        let named: Vec<&FilePath> = followed.iter().map(PathBuf::as_path).collect();
        let mut changes = files.within(&named);
        let mut projects: Vec<&String> = libraries.iter().map(|(project, _)| project).collect();
        projects.sort();
        projects.dedup();
        for project in projects {
            let _ = studio.projects.scan(project).await;
        }
        tokio::select! {
            () = changes.next() => {}
            event = declared.next() => {
                if event.is_err() {
                    return;
                }
            }
        }
    }
}

/// A library's address is a folder's: what its pages link to is beside it.
/// What was said in the address goes with it.
pub fn to_front(project: &str, name: &str, query: Option<&str>) -> Redirect {
    let to = format!("/library/{}/{}/", written(project), written(name));
    Redirect::temporary(&saying(&to, query.unwrap_or_default()))
}

/// The library asked for, and its folder, for someone who may read it.
async fn library(
    studio: &Studio,
    who: &Principal,
    project: &str,
    name: &str,
) -> Result<(Collection, PathBuf)> {
    who.require("records:read")?;
    let collection = studio.collections.get(project, name).await?;
    collection.open_to(who)?;
    if collection.store != Store::Documents {
        return Err(Error::not_found(format!(
            "`{name}` in `{project}` is not a library: its records are not documents, and there are no pages of them"
        )));
    }
    let root = root_of(studio, &collection)?;
    Ok((collection, root))
}

fn html(page: String) -> Response {
    let headers = [
        (CONTENT_TYPE, "text/html; charset=utf-8"),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (CONTENT_SECURITY_POLICY, PAGE_POLICY),
        (CACHE_CONTROL, "no-cache"),
    ];
    (headers, page).into_response()
}

/// A library's front page, for whoever may read it. Each link on it that
/// leads somewhere here says `said` (`name=value`, or nothing) as well.
pub async fn front(
    studio: &Studio,
    who: &Principal,
    project: &str,
    name: &str,
    said: &str,
) -> Result<Response> {
    let (collection, root) = library(studio, who, project, name).await?;
    folder_page(studio, &collection, &root, "", said).await
}

/// A page of a library, a folder of it or a file beside its documents,
/// for whoever may read it, its links saying `said` as [`front`]'s do.
pub async fn page(
    studio: &Studio,
    who: &Principal,
    project: &str,
    name: &str,
    path: &str,
    said: &str,
) -> Result<Response> {
    let (collection, root) = library(studio, who, project, name).await?;
    let asked = path.trim_end_matches('/').to_string();
    let (held, wanted) = (root.clone(), asked.clone());
    // Held inside the library's folder, whatever was asked for: `..`, a
    // path from the top of the disk, a link that leads out.
    let found = tokio::task::spawn_blocking(move || {
        let outside = |err: Error| {
            Error::invalid(err.message.replace("inside the project", "inside this library's folder"))
        };
        let file = resolve(&held, &wanted).map_err(outside)?;
        let meta = std::fs::metadata(&file)
            .map_err(|_| Error::not_found(format!("this library has no `{wanted}`")))?;
        Ok::<_, Error>((file, meta))
    })
    .await
    .map_err(|err| Error::internal(format!("looking for the page crashed: {err}")))??;
    let (file, meta) = found;
    if meta.is_dir() {
        // A folder is asked for as a folder, so that what its page links
        // to is looked for inside it.
        if !path.ends_with('/') {
            let to = format!("/library/{}/{}/{}/", written(project), written(name), written(&asked));
            return Ok(Redirect::temporary(&saying(&to, said)).into_response());
        }
        return folder_page(studio, &collection, &root, &format!("{asked}/"), said).await;
    }
    if asked.ends_with(ENDING) {
        return document_page(studio, &collection, &root, &asked, file, meta.len(), said).await;
    }
    if meta.len() > MAX_RAW_BYTES {
        return Err(Error::invalid(format!(
            "`{asked}` is too large to hand over here ({} bytes)",
            meta.len()
        )));
    }
    let bytes = tokio::fs::read(&file).await.map_err(Error::from)?;
    let ending = asked.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    let kind = match ending.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "txt" | "csv" | "toml" | "yaml" | "yml" | "json" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    // Whatever it is, it is somebody's file: the browser is told not to
    // guess what it is and not to run it.
    let headers = [
        (CONTENT_TYPE, kind),
        (X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (CONTENT_SECURITY_POLICY, "sandbox; default-src 'none'; style-src 'unsafe-inline'"),
    ];
    Ok((headers, bytes).into_response())
}

/// What a library is called on its pages: the project's name and its own.
async fn called(studio: &Studio, collection: &Collection) -> String {
    let project = studio.projects.get(&collection.project).await;
    let project = project.map_or_else(|_| collection.project.clone(), |project| project.name);
    format!("{project} {}", collection.name)
}

/// The line at the head of a page that leads back to the library's front,
/// from a page `deep` folders down. A library that is served is a folder
/// of an address, and its front page is that folder.
fn way_up(called: &str, deep: usize, said: &str) -> String {
    let up = if deep == 0 { "./".to_string() } else { "../".repeat(deep) };
    studio::site::way_up(called, &saying(&up, said))
}

async fn document_page(
    studio: &Studio,
    collection: &Collection,
    root: &FilePath,
    path: &str,
    file: PathBuf,
    size: u64,
    said: &str,
) -> Result<Response> {
    if size > MAX_FILE_BYTES {
        return Err(Error::invalid(format!("`{path}` is too large to make a page of ({size} bytes)")));
    }
    let text = tokio::fs::read_to_string(&file)
        .await
        .map_err(|_| Error::invalid(format!("`{path}` is not text, so there is no page of it")))?;
    let name = path.rsplit('/').next().unwrap_or(path);
    let fallback = name.strip_suffix(ENDING).unwrap_or(name);
    let title = pages::title(&text).unwrap_or_else(|| fallback.to_string());
    let called = called(studio, collection).await;
    let up = way_up(&called, path.matches('/').count(), said);
    let shelf = shelf(collection, root.to_path_buf()).await?;
    let beside = served_navigation(&called, &shelf, path, &shelf.here(path), said);
    let body = body(&text, said);
    Ok(html(pages::shell_beside(&title, &format!("{TO_CONTENTS}{up}{body}"), &beside)))
}

/// A document's blocks, each of its links and pictures that leads
/// somewhere here saying `said` as well.
fn body(text: &str, said: &str) -> String {
    pages::body_with(text, &mut |_, dest| Leads::To(saying(dest, said)))
}

/// The library's contents as its files are now, in their order.
async fn shelf(collection: &Collection, root: PathBuf) -> Result<Shelf> {
    let collection = collection.clone();
    tokio::task::spawn_blocking(move || shelf_of(&collection, &root))
        .await
        .map_err(|err| Error::internal(format!("reading the library's contents crashed: {err}")))
}

/// The page of a folder of a library (`within` is that folder from the
/// library's, ending in `/`; nothing for the library itself): the document
/// that is its front page where it has one, else what is in it, by folder.
async fn folder_page(
    studio: &Studio,
    collection: &Collection,
    root: &FilePath,
    within: &str,
    said: &str,
) -> Result<Response> {
    let called = called(studio, collection).await;
    let deep = within.matches('/').count();
    let (held, inside_of) = (root.to_path_buf(), within.to_string());
    // Through `resolve`, as every file of a library is read: a front page
    // that is a link leading out of the folder is not one.
    let front = tokio::task::spawn_blocking(move || {
        FRONTS.iter().find_map(|front| {
            let file = resolve(&held, &format!("{inside_of}{front}")).ok()?;
            std::fs::read_to_string(file).ok()
        })
    })
    .await
    .map_err(|err| Error::internal(format!("looking for the front page crashed: {err}")))?;
    let shelf = shelf(collection, root.to_path_buf()).await?;
    let beside = served_navigation(&called, &shelf, within, within, said);
    if let Some(text) = front {
        let title = pages::title(&text).unwrap_or_else(|| called.clone());
        let up = if deep == 0 { String::new() } else { way_up(&called, deep, said) };
        let inside = format!("{TO_CONTENTS}{up}{}", body(&text, said));
        return Ok(html(pages::shell_beside(&title, &inside, &beside)));
    }
    let records = studio.collections.records(collection).await?;
    let mut listed: Vec<Listed> = records
        .iter()
        .filter_map(|record| {
            let path = studio::documents::path_of(collection, &record.id)?;
            let title = record.data.get("title").and_then(serde_json::Value::as_str).unwrap_or_default();
            Some(Listed { path, title: title.to_string() })
        })
        .collect();
    // In the order of the library's contents, as a library written out lists them.
    let order = kernel::contents::in_order(&shelf.contents, &shelf.fronts);
    listed.sort_by_key(|listed| order.iter().position(|one| *one == listed.path).unwrap_or(usize::MAX));
    let heading = if within.is_empty() { called.clone() } else { within.trim_end_matches('/').to_string() };
    let up = if deep == 0 { String::new() } else { way_up(&called, deep, said) };
    // Each document is at the address of its file, from this folder.
    let lead = |here: &str| saying(&written(here), said);
    let inside = format!("{TO_CONTENTS}{up}{}", listing(&heading, within, &listed, &lead));
    Ok(html(pages::shell_beside(&heading, &inside, &beside)))
}
