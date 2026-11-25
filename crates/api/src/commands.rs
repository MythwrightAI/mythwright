//! What the factory's bot does when it is spoken to: `talk::bidden`
//! (which came out of here when this crate was at its ceiling), handed
//! this daemon's own routes to ask as whoever said the message
//! ([`Within`]). So the bot holds nothing: what a key may not do with the
//! command line it may not do by asking the bot to, and it is refused by
//! the same route in the same words.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, header};
use kernel::{BoxFut, Principal};
use serde_json::Value;
use talk::bidden::{Routes, Verb};
use tower::ServiceExt;

use crate::state::App;
pub(crate) use studio::commanded::REACHES;
pub use studio::commanded::TAKES;

/// Whoever a request made inside the daemon is made as: somebody already
/// let in, whose message is being done. It is put on a request here and
/// nowhere else, and nothing that arrives over the network can carry it.
#[derive(Clone)]
pub(crate) struct Within(pub Principal);

/// The most that is read of what a route answered.
const MOST: usize = 4 * 1024 * 1024;

/// Has the factory's bot hear what is said to it, from here on.
pub(crate) fn listen(app: &App) {
    // The bot is held by the app: holding the app back would keep both
    // for ever.
    let kept = app.weak();
    app.studio.channels.speaker.hear_with(Arc::new(move |who, message| {
        let app = App::kept(&kept);
        Box::pin(async move {
            if let Some(app) = app {
                talk::bidden::answer(&app.studio.channels, &Asked(app.clone()), &who, &message).await;
            }
        })
    }));
}

/// This daemon's routes, as the bot asks them.
struct Asked(App);

impl Routes for Asked {
    fn ask<'a>(
        &'a self,
        who: &'a Principal,
        verb: Verb,
        path: String,
        body: Option<Value>,
    ) -> BoxFut<'a, Result<Value, String>> {
        let method = match verb {
            Verb::Get => Method::GET,
            Verb::Post => Method::POST,
            Verb::Patch => Method::PATCH,
            Verb::Delete => Method::DELETE,
        };
        Box::pin(ask(&self.0, who, method, path, body))
    }
}

/// Asks one of this daemon's routes as `who`: what it answered, or the
/// words it refused in.
async fn ask(
    app: &App,
    who: &Principal,
    method: Method,
    path: String,
    body: Option<Value>,
) -> Result<Value, String> {
    let request = Request::builder().method(method).uri(path).header(header::HOST, "localhost");
    let request = match body {
        Some(body) => {
            request.header(header::CONTENT_TYPE, "application/json").body(Body::from(body.to_string()))
        }
        None => request.body(Body::empty()),
    };
    let mut request = request.map_err(|err| format!("That could not be asked: {err}."))?;
    request.extensions_mut().insert(Within(who.clone()));
    let Ok(response) = crate::routes::router(app.clone()).oneshot(request).await;
    let status = response.status();
    let read = axum::body::to_bytes(response.into_body(), MOST).await;
    let answer: Value = read.ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
    match status.is_success() {
        true => Ok(answer),
        false => Err(match answer["error"]["message"].as_str() {
            Some(why) => format!("That was refused: {why}."),
            None => format!("That was refused ({status})."),
        }),
    }
}
