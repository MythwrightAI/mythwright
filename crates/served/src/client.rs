//! Serving the client, and noticing when it is replaced.
//!
//! The client is a folder of static files. It is served from disk rather
//! than compiled in, so changing it never rebuilds the daemon; a release
//! ships the folder beside the binary.
//!
//! It also never restarts the daemon. A new client is put in place by
//! replacing the folder's entry page, last and in one step, after the files
//! it names are there. The system tells the daemon that the page changed
//! (nothing looks for it on a timer), the daemon records `client.updated`,
//! and pages that are open offer to reload. Nothing the daemon runs is
//! touched.
//!
//! It came out of the daemon (`api::ui`) when that crate was at its
//! ceiling, handed the client, the journal and what follows files.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use axum::http::header::{CACHE_CONTROL, CONTENT_SECURITY_POLICY, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use hosting::{Changes, Watch};
use journal::Journal;
use kernel::Source;
use serde_json::json;
use tokio::sync::Notify;

/// Where the last build seen is remembered, so a client replaced while the
/// daemon was down is announced when it comes back.
pub const REMEMBERED: (&str, &str) = (folders::snapshot::CLIENT, "build");

/// The build a client's entry page says it is, which the client's build
/// writes there as `<meta name="build" content="...">`.
pub fn build_of(page: &str) -> Option<String> {
    let marker = "name=\"build\" content=\"";
    let rest = &page[page.find(marker)? + marker.len()..];
    let id = &rest[..rest.find('"')?];
    let plain = !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    plain.then(|| id.to_string())
}

/// The client this daemon serves: where it is, and which build it was when
/// last looked at.
pub struct Client {
    configured: Option<PathBuf>,
    build: Mutex<Option<String>>,
    /// Where the client was the last time anyone asked, and the signal that
    /// it is now somewhere else (or exists at last).
    found: Mutex<Option<PathBuf>>,
    moved: Notify,
}

impl Client {
    /// Finds the client and compares it with what the last daemon on this
    /// data saw.
    pub async fn open(journal: &Journal, configured: Option<PathBuf>) -> Client {
        let remembered = journal.kv_get(REMEMBERED.0, REMEMBERED.1).await.ok().flatten();
        let build = remembered.and_then(|value| value.as_str().map(str::to_string));
        let client =
            Client { configured, build: Mutex::new(build), found: Mutex::default(), moved: Notify::new() };
        client.look(journal).await;
        client
    }

    /// Where the client's files are, if it has been built.
    pub fn dir(&self) -> Option<PathBuf> {
        let dir = locate(self.configured.as_deref());
        let mut found = self.found.lock().expect("client lock");
        if *found != dir {
            found.clone_from(&dir);
            self.moved.notify_one();
        }
        dir
    }

    /// The build being served, read now rather than remembered: whoever
    /// asks is about to load it.
    pub async fn build(&self, journal: &Journal) -> Option<String> {
        self.look(journal).await;
        self.build.lock().expect("client lock").clone()
    }

    /// Reads the entry page and, if it is a different build from the last
    /// one seen, records that.
    async fn look(&self, journal: &Journal) {
        let Some(dir) = self.dir() else { return };
        // An entry page that cannot be read, or says nothing about its
        // build, changes nothing: the last build seen stands.
        let Ok(page) = tokio::fs::read_to_string(dir.join("index.html")).await else { return };
        let Some(found) = build_of(&page) else { return };
        let was = {
            let mut build = self.build.lock().expect("client lock");
            if build.as_deref() == Some(found.as_str()) {
                return;
            }
            build.replace(found.clone())
        };
        let _ = journal.kv_set(REMEMBERED.0, REMEMBERED.1, &json!(found)).await;
        // The first client a daemon ever sees replaced nothing, and nobody
        // has a page open to tell.
        if was.is_some() {
            let data = json!({ "build": found, "was": was });
            let _ = journal.emit("client.updated", Source::actor("system"), data).await;
        }
    }
}

async fn next(changes: &mut Option<Changes>) {
    match changes {
        Some(changes) => changes.next().await,
        None => std::future::pending().await,
    }
}

/// Follows the client's folder for as long as the daemon runs, and looks at
/// the entry page when the system says it changed.
///
/// Nothing here runs on a timer. If the system cannot say (see
/// `hosting::Watch`), a new client is noticed when someone next asks which
/// build is being served, which every page does as it loads.
pub async fn watch(client: &Client, journal: &Journal, files: &Watch) {
    // The folder is followed by name even while it is not there, so that one
    // removed and built again is seen coming back.
    let mut last: Option<PathBuf> = None;
    loop {
        let dir = client.dir();
        if dir.is_some() {
            last.clone_from(&dir);
        }
        let mut page = dir.as_ref().map(|dir| files.files(&[&dir.join("index.html")]));
        let mut folder = last.as_ref().map(|dir| files.files(&[dir]));
        loop {
            // First, for whatever changed before anyone was listening.
            client.look(journal).await;
            tokio::select! {
                () = next(&mut page) => {}
                () = next(&mut folder) => {
                    // A folder that was replaced is a different folder to
                    // the system, which goes on watching the old one.
                    if let Some(dir) = &last {
                        files.refresh(dir);
                    }
                    break;
                }
                () = client.moved.notified() => break,
            }
        }
    }
}

/// Where the client's files are: where the node was told, else beside the
/// program, else, for a program run from the checkout it was built in,
/// where the client is built there. A copy of the program anywhere else
/// never looks in a checkout ((`folders::place`)).
pub fn locate(configured: Option<&Path>) -> Option<PathBuf> {
    let beside = folders::place::program_dir().map(|dir| dir.join("ui"));
    let checkout = folders::place::checkout().map(|root| root.join("ui").join("dist"));
    configured
        .map(Path::to_path_buf)
        .into_iter()
        .chain(beside)
        .chain(checkout)
        .find(|dir| dir.join("index.html").is_file())
}

/// Why there is no client, and what to do about it: which differs for a
/// program run from its checkout and a copy on its own.
fn no_client() -> &'static str {
    if folders::place::checkout().is_some() {
        "it has not been built. In the repository, run <code>npm install &amp;&amp; npm run build</code> \
         in <code>ui/</code>, then reload."
    } else {
        "this is a copy of the program on its own, and a copy serves the client that is in a folder \
         called <code>ui</code> beside it. Install it again, or start it with <code>--ui</code> and \
         where the client is."
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

/// The file of the client that a request's path names, when it names one
/// that is there and inside the client's folder.
///
/// A path is taken apart as the system would take it apart, and only
/// ordinary names are let through: no `..`, nothing that starts again from
/// the top, no drive (`C:`), which on Windows makes whatever it is joined to
/// irrelevant, and no backslash, which is a separator there and part of a
/// name everywhere else. What is left is then resolved, links and all, and
/// must still be under the folder. This route answers without a credential,
/// so what it can be made to read is what anyone can read.
pub fn within(root: &Path, requested: &str) -> Option<PathBuf> {
    if requested.is_empty() || requested.contains('\\') || requested.contains(':') {
        return None;
    }
    let ordinary =
        Path::new(requested).components().all(|part| matches!(part, std::path::Component::Normal(_)));
    if !ordinary || requested.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
        return None;
    }
    let file = root.join(requested);
    let (real, top) = (kernel::whole::whole(&file).ok()?, kernel::whole::whole(root).ok()?);
    (kernel::whole::within(&real, &top) && real.is_file()).then_some(file)
}

/// Whether a path that names no file of the client's is one of its files
/// that is not there, and not an address of a thing: a name with a dot in
/// it at the top of the client's folder, or anything under one of its
/// folders.
fn missing_asset(root: &Path, requested: &str) -> bool {
    match requested.split_once('/') {
        None => requested.contains('.'),
        Some((first, _)) => !first.is_empty() && root.join(first).is_dir(),
    }
}

/// Serves a file of the client, or its entry page for any path that is not a
/// file: the client does its own routing.
/// What the client's own page may fetch and frame: this daemon and no other.
pub const CLIENT_POLICY: &str = "frame-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; \
                                 connect-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self' data:; \
                                 object-src 'none'; base-uri 'self'; form-action 'self'";

pub async fn serve(client: &Client, uri: Uri) -> Response {
    let Some(root) = client.dir() else {
        let page = format!(
            "<!doctype html><meta charset=utf-8><title>{name}</title>\
             <body style=\"font:16px system-ui;max-width:40rem;margin:4rem auto;padding:0 1rem\">\
             <h1>{name} is running</h1>\
             <p>The daemon is up, but it has no client to serve: {why}</p>\
             <p>The API is at <code>/api</code>; the command line works without the client.</p>",
            name = brand::DISPLAY,
            why = no_client()
        );
        return Html(page).into_response();
    };
    let requested = uri.path().trim_start_matches('/');
    // Only a file that is inside the client's folder is looked up; anything
    // else gets the entry page.
    let (file, fallback) = match within(&root, requested) {
        Some(file) => (file, false),
        None => (root.join("index.html"), true),
    };
    // A request for a missing asset is a 404, not the entry page: serving
    // HTML where a script was asked for hides the real problem. An asset is
    // a file at the top of the client's folder or in a folder of it; any
    // other path is an address of the client's, which names a thing by
    // where it is (`/shop/docs/guides/install.md`), and no project is
    // called what a folder of the client's is (`kernel::paths`).
    if fallback && missing_asset(&root, requested) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(&file).await {
        Ok(bytes) => {
            // Built assets carry a hash in their name and never change; the
            // entry page must always be fresh.
            let cache = if requested.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            let mut response =
                ([(CONTENT_TYPE, content_type(&file)), (CACHE_CONTROL, cache)], bytes).into_response();
            // The client frames only what this daemon serves. A view shown
            // in a channel is in a frame, and one that tried to go somewhere
            // else would otherwise take its frame there. And it fetches
            // nothing from anywhere else: a picture in a document somebody
            // else wrote is a request the reader's browser makes, with the
            // reader's session if it is to another port of this machine,
            // and nothing leaves the machine unasked. What is shown in a
            // frame made here (a document as a page, a view) is held to
            // the same.
            if fallback || requested.is_empty() || requested == "index.html" {
                response
                    .headers_mut()
                    .insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CLIENT_POLICY));
            }
            response
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
