//! What the program says of itself, for the client's page about it: what it is for (`docs/about.md`), the guides of
//! `docs/` marked public, each as a page, the packages installed and
//! whether each is the one that came, to whoever may read them, and where
//! this factory keeps its data, to whoever may manage it. All of it came with the program
//! (`manual::guides`), as the notes of each release did, and nothing is
//! fetched to show it.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

/// The guide that says what the program is for: a page of its own, and no
/// guide among the others.
const ABOUT: &str = "about";

pub fn routes() -> Router<App> {
    Router::new().route("/about", get(about)).route("/about/guides/{name}", get(guide))
}

/// A guide as a page, made as any document's page is, without the block
/// of settings at its head. No picture is drawn (one beside a guide in the
/// repository is no part of the program, and one on the web would be
/// fetched as the page is read), and a link to a file no copy of the
/// program carries (`../README.md`, a design record) leads nowhere: each
/// keeps its words. A link to another guide, to a place in the page or to
/// the web is kept, and the client follows it only when it is pressed.
fn page_of(name: &str, text: &str) -> String {
    let markdown = manual::guides::body(text);
    let inside = studio::pages::body_with(markdown, &mut |picture, to| {
        let guide = |stem: &str| {
            manual::guides::GUIDES
                .iter()
                .any(|(guide, _)| *guide == stem.trim_start_matches("./") && *guide != ABOUT)
        };
        let carried = !picture
            && (to.contains(':')
                || to.split('#').next().and_then(|file| file.strip_suffix(".md")).is_none_or(guide));
        match carried {
            true => studio::pages::Leads::To(to.to_string()),
            false => studio::pages::Leads::Nowhere,
        }
    });
    studio::pages::shell(&studio::pages::title(markdown).unwrap_or_else(|| name.to_string()), &inside)
}

/// What a guide calls itself: its first heading, or its name.
fn title_of(name: &str, text: &str) -> String {
    studio::pages::title(manual::guides::body(text)).unwrap_or_else(|| name.to_string())
}

/// What the program is for, as a page; the guides there are, by name and
/// what each calls itself, in the order they are kept; and, to whoever may
/// manage the factory and nobody else, the folder its data is kept in.
async fn about(State(app): State<App>, Auth(who): Auth) -> Json<Value> {
    let guides: Vec<Value> = manual::guides::GUIDES
        .iter()
        .filter(|(name, _)| *name != ABOUT)
        .map(|(name, text)| json!({ "name": name, "title": title_of(name, text) }))
        .collect();
    let about = manual::guides::guide(ABOUT);
    let mut told =
        json!({ "title": title_of(ABOUT, about), "html": page_of(ABOUT, about), "guides": guides });
    // The packages installed, to whoever may read them: each the one that
    // came with this version, another than the one that came, or one that
    // came with none (installed from a manifest of somebody's own).
    if who.can("agents:read") {
        let (came, behind) = (app.agents.came_names(), app.agents.behind_by());
        let packages: Vec<Value> = app
            .agents
            .extensions()
            .iter()
            .map(|one| {
                let stands = match (came.contains(&one.name), behind.contains_key(&one.name)) {
                    (false, _) => "own",
                    (true, true) => "differs",
                    (true, false) => "came",
                };
                json!({ "name": one.name, "title": one.title, "stands": stands })
            })
            .collect();
        told["packages"] = json!(packages);
    }
    if who.can("admin") {
        let data = kernel::whole::whole(&app.place.data).unwrap_or_else(|_| app.place.data.clone());
        told["data"] = json!(data.to_string_lossy());
    }
    Json(told)
}

/// One guide as a page, by its name, for anybody signed in.
async fn guide(Auth(_): Auth, Path(name): Path<String>) -> ApiResult<Json<Value>> {
    let (name, text) = manual::guides::GUIDES
        .iter()
        .find(|(called, _)| *called == name && *called != ABOUT)
        .ok_or_else(|| Error::not_found(format!("there is no guide called `{name}`")))?;
    Ok(Json(json!({ "name": name, "title": title_of(name, text), "html": page_of(name, text) })))
}
