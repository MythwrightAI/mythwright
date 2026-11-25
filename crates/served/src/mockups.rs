//! Mockups, served: a page somebody made to try an idea with, shown so
//! that it can do nothing as whoever looks at it.
//!
//! A mockup is a folder of plain web files (`studio::mockups`). Its page
//! may hold scripts, and they run: a prototype is no use otherwise. So it
//! is served in a place of its own.
//!
//! **A way in.** Whoever may read the project's records asks for one
//! (`POST /api/mockups/way`), and is given an address that holds a name
//! nobody could guess: `/mockups/in/<name>/`. That name lets its holder
//! read the files of that one mockup's folder and nothing else, for a
//! while, and is no credential: it is not a session, it opens no route of
//! the API, and what it reads it cannot change. It is how the page's own
//! files (its style sheet, its scripts, its pictures) are let through
//! without the session: a page in a place of its own is not sent the
//! session's cookie, so the cookie could never have fetched them.
//!
//! **A place of its own.** Everything served under a way in carries a
//! policy that makes the browser treat it as belonging to nowhere
//! (`sandbox`): it has no cookies and no storage, what it asks of this
//! daemon is asked as a stranger, and it may fetch only from its own way
//! in. That holds whether it is shown in the client's frame or opened at
//! its own address, and for every kind of file, since a picture that is a
//! drawing can hold a script too.
//!
//! **An address of its own.** `/mockups/<id>` is where a mockup is opened
//! outside the client, on a phone say, by whoever is signed in and may
//! read the project: it makes a way in and sends the browser there.
//!
//! Whatever is asked for is held inside the mockup's folder.
//!
//! It came out of the daemon (`api::mockups`, which keeps the routes,
//! each a call of this) when that crate was at its ceiling, handed the
//! studio and the ways in there are.

use std::collections::HashMap;
use std::path::{Component, Path as FilePath};
use std::sync::Mutex;

use axum::http::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE, REFERRER_POLICY,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::response::{IntoResponse, Response};
use kernel::{Error, Principal, Result};
use studio::mockups::{PAGE, folder_of, name_of};
use studio::projects::{MAX_RAW_BYTES, resolve};
use studio::{Collection, Store, Studio};

/// How long a way in lasts, in milliseconds. Asked for again, one with
/// more than half of that left is given again, so that a page that shows
/// many mockups, or shows them often, does not make many.
const LASTS_MS: i64 = 60 * 60 * 1000;

/// How many ways in are kept at once. Past it, the ones that have run out
/// go first, and then the ones that will soonest.
const MOST: usize = 2048;

struct Way {
    project: String,
    /// The mockup's folder, from the top of its project.
    folder: String,
    until: i64,
}

/// The ways in there are. They are kept in memory and nowhere else: a
/// daemon that starts again has none, and whoever shows a mockup asks again.
#[derive(Default)]
pub struct Ways {
    held: Mutex<HashMap<String, Way>>,
}

impl Ways {
    /// A way in to one mockup's folder: its name, and when it runs out.
    pub fn open(&self, project: &str, folder: &str, now: i64) -> (String, i64) {
        let mut held = self.held.lock().expect("ways lock");
        let there = held.iter().find(|(_, way)| {
            way.project == project && way.folder == folder && way.until - now > LASTS_MS / 2
        });
        if let Some((name, way)) = there {
            return (name.clone(), way.until);
        }
        held.retain(|_, way| way.until > now);
        while held.len() >= MOST {
            let Some(soonest) = held.iter().min_by_key(|(_, way)| way.until).map(|(name, _)| name.clone())
            else {
                break;
            };
            held.remove(&soonest);
        }
        let name = kernel::id::secret();
        held.insert(
            name.clone(),
            Way { project: project.to_string(), folder: folder.to_string(), until: now + LASTS_MS },
        );
        (name, now + LASTS_MS)
    }

    /// The project and folder a way in leads to, while it lasts.
    pub fn leads(&self, name: &str, now: i64) -> Option<(String, String)> {
        let held = self.held.lock().expect("ways lock");
        held.get(name).filter(|way| way.until > now).map(|way| (way.project.clone(), way.folder.clone()))
    }
}

/// The mockup known by `id`, for someone who may read it: its collection
/// and its folder, from the top of its project.
pub async fn mockup(studio: &Studio, who: &Principal, id: &str) -> Result<(Collection, String)> {
    who.require("records:read")?;
    let (collection, _) = studio.collections.record(id).await?;
    collection.open_to(who)?;
    let name = name_of(&collection, id).filter(|_| collection.store == Store::Mockups).ok_or_else(|| {
        Error::not_found(format!(
            "`{id}` is not a mockup: it is a record of `{}`, which has no page to show",
            collection.name
        ))
    })?;
    let folder = folder_of(&collection, &name);
    Ok((collection, folder))
}

/// The address of a way in.
pub fn way_into(name: &str) -> String {
    format!("/mockups/in/{name}/")
}

/// What kind of thing a file is, by what its name ends in.
fn media(path: &str) -> &'static str {
    let ending = path.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
    match ending.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "txt" | "md" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "xml" => "application/xml",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// What everything under a way in is sent with.
///
/// `sandbox` puts the page in a place of its own: no cookies, no storage,
/// and whatever it asks of this daemon it asks as a stranger. Scripts,
/// forms and dialogs work there, since a prototype needs them; a form is
/// sent nowhere. Then it may fetch from its own way in and from nowhere
/// else, this daemon's API included. `host` is what the browser asked
/// this daemon as, taken only when it is plainly an address.
fn policy(host: Option<&str>, name: &str) -> String {
    let plain = |host: &&str| {
        !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
    };
    let own = match host.filter(plain) {
        Some(host) => format!("{host}{}", way_into(name)),
        None => String::new(),
    };
    format!(
        "sandbox allow-scripts allow-forms allow-modals; default-src 'none'; \
         script-src {own} 'unsafe-inline' 'unsafe-eval'; style-src {own} 'unsafe-inline'; \
         img-src {own} data: blob:; font-src {own} data:; media-src {own} data: blob:; \
         connect-src {own}; frame-src {own}; worker-src 'none'; \
         form-action 'none'; base-uri 'none'; frame-ancestors 'self'"
    )
}

/// A file of the mockup a way in leads to, for whoever holds the way in:
/// `host` is what the browser asked this daemon as.
pub async fn serve(
    studio: &Studio,
    ways: &Ways,
    now: i64,
    host: Option<&str>,
    name: &str,
    path: &str,
) -> Result<Response> {
    let gone = || {
        Error::not_found(
            "there is no such way in to a mockup: it has run out, or the daemon has started again since. Open the mockup again from the client, or at its own address",
        )
    };
    let (project, folder) = ways.leads(name, now).ok_or_else(gone)?;
    // A folder's address is its page.
    let wanted = match path {
        "" => PAGE.to_string(),
        path if path.ends_with('/') => format!("{path}{PAGE}"),
        path => path.to_string(),
    };
    // Only ordinary names: nothing that goes up, starts again from the
    // top, or could be read as either on another system.
    let ordinary = !wanted.contains(['\\', ':'])
        && FilePath::new(&wanted).components().all(|part| matches!(part, Component::Normal(_)));
    if !ordinary {
        return Err(Error::invalid(format!(
            "`{wanted}` is not a file of this mockup: only what is in its own folder is shown"
        )));
    }
    let dir = studio
        .projects
        .dir_of(&project)
        .ok_or_else(|| Error::not_found(format!("there is no project `{project}`")))?;
    let (root, asked) = (dir.join(&folder), wanted.clone());
    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        // Against the mockup's own folder, so that a link in it that leads
        // elsewhere in the project is not followed out of it.
        let target = resolve(&root, &asked)?;
        let meta = std::fs::metadata(&target)
            .ok()
            .filter(|meta| meta.is_file())
            .ok_or_else(|| Error::not_found(format!("this mockup has no file `{asked}`")))?;
        if meta.len() > MAX_RAW_BYTES {
            return Err(Error::invalid(format!(
                "`{asked}` is too large to show here ({} bytes)",
                meta.len()
            )));
        }
        Ok(std::fs::read(&target)?)
    })
    .await
    .map_err(|err| Error::internal(format!("reading the mockup crashed: {err}")))??;
    let sent = [
        (CONTENT_TYPE, media(&wanted).to_string()),
        (X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
        (CONTENT_SECURITY_POLICY, policy(host, name)),
        // The address holds the way in: it is told to nobody the page
        // sends its reader on to.
        (REFERRER_POLICY, "no-referrer".to_string()),
        (CACHE_CONTROL, "no-store".to_string()),
        // A page in a place of its own asks for its own files as a
        // stranger would. They are let through to whoever holds the way
        // in, which is all that reading them ever took.
        (ACCESS_CONTROL_ALLOW_ORIGIN, "*".to_string()),
    ];
    Ok((sent, bytes).into_response())
}
