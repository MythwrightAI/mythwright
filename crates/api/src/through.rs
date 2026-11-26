//! A page a browser opens at its address: a library's, a mockup's.
//!
//! A browser that opens one hands it the cookie and nothing more, and the
//! cookie, which a browser hands to every port of the machine, reads
//! nothing by itself (`state::shown`): such a page is answered for what
//! the client's page says beside it in the address (`by`), and its links
//! say the same. An address of one that says nothing beside it (typed,
//! kept, copied before) is sent through the client, where whoever is
//! signed in has what to say: the page there adds it and sends the browser
//! on (`ui/src/senton.ts`), and somebody not signed in signs in there
//! first. What asks for it without a browser (a script, a test) is told
//! to sign in, as anywhere.

use axum::extract::FromRequestParts;
use axum::http::Method;
use axum::http::header::ACCEPT;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Redirect, Response};
use kernel::addressed::written;
use kernel::{Error, ErrorKind, Principal};

use crate::http::ApiError;
use crate::state::{App, Bearing, SHOWN};

/// What the client's page is told, in its own address, to send the
/// browser on to.
pub const THROUGH: &str = "to";

/// Who opened a page, and what each of its links that leads somewhere
/// here is to say as well (`by=...`, or nothing for one asked with a
/// token that is said).
pub struct Opened {
    pub who: Principal,
    pub said: String,
}

impl FromRequestParts<App> for Opened {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, Response> {
        let bearing = Bearing::of(parts, app);
        let who = match &bearing {
            Some(bearing) => bearing.who(app).await,
            None => Err(Error::unauthenticated("sign in first")),
        };
        match who {
            Ok(who) => {
                // Only what can be nothing but the digest it was compared
                // as is written into a page again.
                let shown = bearing.as_ref().and_then(Bearing::shown);
                let said = shown
                    .filter(|by| !by.is_empty() && by.bytes().all(|byte| byte.is_ascii_hexdigit()))
                    .map(|by| format!("{SHOWN}={by}"))
                    .unwrap_or_default();
                Ok(Opened { who, said })
            }
            Err(err) if err.kind == ErrorKind::Unauthenticated && opened_by_a_browser(parts) => {
                Err(Redirect::to(&through(parts)).into_response())
            }
            Err(err) => Err(ApiError(err).into_response()),
        }
    }
}

/// Whether a browser is opening this as a page, as opposed to fetching it
/// for one (a picture) or something else asking.
fn opened_by_a_browser(parts: &Parts) -> bool {
    let wants = parts.headers.get(ACCEPT).and_then(|accept| accept.to_str().ok()).unwrap_or_default();
    parts.method == Method::GET && wants.contains("text/html")
}

/// The client's page, told to send the browser on to what was asked,
/// without what it said beside the cookie (which did not count, and which
/// the page says again as things now stand).
fn through(parts: &Parts) -> String {
    let kept: Vec<&str> = parts
        .uri
        .query()
        .unwrap_or_default()
        .split('&')
        .filter(|pair| !pair.is_empty() && pair.split_once('=').map_or(*pair, |(name, _)| name) != SHOWN)
        .collect();
    let asked = match kept.is_empty() {
        true => parts.uri.path().to_string(),
        false => format!("{}?{}", parts.uri.path(), kept.join("&")),
    };
    format!("/?{THROUGH}={}", written(&asked))
}
