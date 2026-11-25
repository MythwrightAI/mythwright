//! Beginning with no registration (`people::opened`): the routes that make
//! a link that signs somebody in once, trade one for a session, and give
//! an account that has none a password or another name.
//!
//! A link is asked for two ways. With a credential, by a person for
//! themselves (`POST /api/auth/once`), with their session or a key that
//! holds all they hold. Or with no credential, for the owner, by whoever
//! proves they can read the factory's data folder: they are given something
//! to prove (`POST /api/auth/once/asked`), and answer with what the
//! installation's secret makes of it (`POST /api/auth/once/proved`), which
//! only the command line on this machine, run as whoever runs the factory,
//! can read. Nothing that crosses the network is good for more than that
//! once. A link is traded for a session by a route that takes none
//! (`POST /api/auth/link`), once and within its minute.

use std::path::Path;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{post, put};
use axum::{Json, Router};
use kernel::Error;
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::routes::{required, text};
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    Router::new()
        .route("/auth/once", post(once))
        .route("/auth/once/asked", post(asked))
        .route("/auth/once/proved", post(proved))
        .route("/auth/link", post(link))
        .route("/me/password", put(password))
        .route("/me/name", put(name))
}

/// What says that whoever asks can read the factory's data folder at
/// `data`, for `nonce`: asked by the command line on this machine.
pub fn proves(data: &Path, nonce: &str) -> kernel::Result<String> {
    let own = gateway::own::Own::kept(data).map_err(|err| Error::invalid(err.to_string()))?;
    Ok(own.proves(nonce))
}

fn made(link: people::opened::Link) -> Json<Value> {
    Json(json!({ "once": link.once, "expires": link.expires }))
}

/// A link that signs whoever asks in once, as themselves.
async fn once(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    Ok(made(app.studio.identity.link_for(&who).await?))
}

/// Something to prove, for a link that signs the owner in; and, to a
/// program on this machine, which folder it is proved by. Where the folder
/// is says nothing that opens it: only somebody who can read its secret
/// proves anything, and a command line told only an address finds it so.
async fn asked(
    State(app): State<App>,
    ConnectInfo(from): ConnectInfo<std::net::SocketAddr>,
) -> ApiResult<Json<Value>> {
    let nonce = app.studio.identity.nonce();
    Ok(Json(match from.ip().is_loopback() {
        true => json!({ "nonce": nonce, "data": app.place.data }),
        false => json!({ "nonce": nonce }),
    }))
}

/// The owner's link, for whoever proved they can read the data folder.
async fn proved(State(app): State<App>, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let (nonce, proof) = (required(&body, "nonce")?, required(&body, "proof")?);
    // Taken back before anything is looked at, whatever comes of it: one
    // thing to prove is answered once.
    app.studio.identity.take_nonce(nonce)?;
    let refused = || Error::unauthenticated("that does not prove this factory's data folder can be read");
    let expected = proves(&app.place.data, nonce).map_err(|_| refused())?;
    if !people::identity::same(&expected, proof) {
        return Err(refused().into());
    }
    Ok(made(app.studio.identity.link_for_owner().await?))
}

/// A link traded for a session: a browser's page's, as signing in gives.
async fn link(State(app): State<App>, headers: HeaderMap, Json(body): Json<Value>) -> ApiResult<Response> {
    let signed = app.studio.identity.spend(required(&body, "once")?).await.map_err(|err| {
        Error::unauthenticated(format!("{}: `{} open` makes another", err.message, brand::CLI))
    })?;
    crate::routes::signed_in(&app, signed, &body, &headers).await
}

/// A password to sign in with another way, with an email where the
/// account has none: for an account that has no password.
async fn password(
    State(app): State<App>,
    Auth(who): Auth,
    Json(body): Json<Value>,
) -> ApiResult<Json<Value>> {
    let account =
        app.studio.identity.set_password(&who, text(&body, "email"), required(&body, "password")?).await?;
    Ok(Json(json!({ "account": account })))
}

/// Whoever asks, called something else; and, for the owner named for the
/// first time, the workspace made with them.
async fn name(State(app): State<App>, Auth(who): Auth, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let (account, was) = app.studio.identity.rename(&who, required(&body, "name")?).await?;
    // The owner a first start made, named for the first time: the workspace
    // made with them, still called what it was made as, is called after
    // them (`Workspaces::follow_owner`).
    if account.role == "owner" && was == kernel::UNNAMED {
        app.studio.workspaces.follow_owner(&account.name, &who).await?;
    }
    Ok(Json(json!({ "account": account })))
}
