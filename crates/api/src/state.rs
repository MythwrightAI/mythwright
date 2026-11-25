//! What every request handler can reach, and who is asking.

use std::net::SocketAddr;
use std::sync::Arc;

use agents::Agents;
use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;
use conductor::Engine;
use gateway::Gateway;
use hosting::Hosting;
use journal::Journal;
use kernel::{Error, Principal};
use studio::Studio;

use crate::http::ApiError;
use served::client::Client;

/// The name of the cookie a browser holds its session in, for a factory
/// with no profile.
///
/// A browser keeps cookies by host and not by port, so two factories on one
/// device would each overwrite the other's session if both called it this:
/// signing in to one would sign the page out of the other. A factory with
/// a profile therefore calls its cookie after itself ([`cookie_for`]).
pub const SESSION_COOKIE: &str = "session";

/// What a factory calls its session cookie: the usual name, and for one
/// with a profile the factory's own id after it. Not the profile's
/// name, which two factories may share.
pub(crate) fn cookie_for(stamp: Option<&crate::profile::Stamp>) -> String {
    match stamp {
        Some(stamp) => format!("{SESSION_COOKIE}-{}", stamp.id),
        None => SESSION_COOKIE.to_string(),
    }
}

#[derive(Clone)]
pub struct App {
    inner: Arc<Inner>,
}

pub struct Inner {
    pub journal: Journal,
    pub engine: Engine,
    /// What is asked of comparisons, which the engine asks too.
    pub comparisons: comparisons::Comparisons,
    pub hosting: Hosting,
    pub studio: Studio,
    pub agents: Agents,
    pub gateway: Gateway,
    pub client: Client,
    pub addr: SocketAddr,
    /// The names this daemon answers to besides its own addresses.
    pub names: Arc<served::Names>,
    /// The ways in to mockups that are open now (`crate::mockups`).
    pub ways: served::mockups::Ways,
    /// The calls taken lately from callers that sign (`crate::callers`).
    /// Held while one is taken, so that the same call sent twice at once
    /// is said once.
    pub seen: tokio::sync::Mutex<crate::callers::Seen>,
    /// Told when the owner asks the daemon to stop.
    pub stop: tokio::sync::Notify,
    /// Its drain, once it was asked to stop when what is in hand is done.
    pub drain: crate::drain::Drain,
    /// What kind of factory this is, and how it is known on this device.
    pub place: Place,
    /// The sandbox tests under way (`crate::sandboxes`).
    pub sandboxes: crate::sandboxes::Sandboxes,
    /// The projects with where each keeps its own blueprints
    /// (`crate::library`).
    pub own: crate::library::Own,
}

/// Where a factory stands among the factories on its device.
pub struct Place {
    /// What its profile is called, when it has one.
    pub profile: Option<String>,
    /// Its data folder, which is what tells it from every other factory.
    pub data: std::path::PathBuf,
    /// The list of the factories on this device, when one is kept.
    pub registry: Option<crate::registry::Registry>,
    /// What this factory calls a browser's session cookie.
    pub cookie: String,
}

impl std::ops::Deref for App {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.inner
    }
}

impl App {
    pub(crate) fn new(
        journal: Journal,
        engine: Engine,
        comparisons: comparisons::Comparisons,
        hosting: Hosting,
        studio: Studio,
        agents: Agents,
        gateway: Gateway,
        client: Client,
        addr: SocketAddr,
        names: Arc<served::Names>,
        place: Place,
        sandboxes: crate::sandboxes::Sandboxes,
        own: crate::library::Own,
    ) -> App {
        let stop = tokio::sync::Notify::new();
        let inner = Inner {
            journal,
            engine,
            comparisons,
            hosting,
            studio,
            agents,
            gateway,
            client,
            addr,
            names,
            ways: served::mockups::Ways::default(),
            seen: Default::default(),
            stop,
            drain: Default::default(),
            place,
            sandboxes,
            own,
        };
        App { inner: Arc::new(inner) }
    }

    /// What where a text stands among those that ship is worked out from
    /// (`crate::forked`).
    pub(crate) fn knowing(&self) -> crate::forked::Knowing<'_> {
        crate::forked::Knowing { engine: &self.engine, projects: &self.studio.projects, own: &self.own }
    }

    /// The app for as long as something else keeps it: for what the app
    /// itself holds, which must not keep the app in turn.
    pub(crate) fn weak(&self) -> std::sync::Weak<Inner> {
        Arc::downgrade(&self.inner)
    }

    /// The app, if it is still there.
    pub(crate) fn kept(weak: &std::sync::Weak<Inner>) -> Option<App> {
        weak.upgrade().map(|inner| App { inner })
    }
}

impl Inner {
    /// Whether `who` may be told of an event of this topic: whether it
    /// holds what reading the thing the event is about needs. A credential
    /// says what its bearer may do, and being told is one of those things:
    /// what is said in a channel is in the event that says so. The
    /// conductor asks the same of what a blueprint's handlers hear.
    pub fn may_hear(&self, who: &Principal, event: &kernel::Event) -> bool {
        // And what the event itself says hearing it takes: it was said of
        // something kept from those without that.
        self.journal.may_tell(who, event) && self.engine.may_hear(who, &event.topic)
    }
}

/// The header a page of this daemon's own sends its key in, beside the
/// browser's cookie (`studio::Came`).
pub const PAGE_KEY: &str = "x-page-key";

/// The token a request carries and whether it came in a browser's cookie:
/// a bearer token, or the cookie called `cookie` and no other the browser
/// sent.
fn carried(parts: &Parts, cookie: &str) -> Option<(String, bool)> {
    if let Some(value) = parts.headers.get(AUTHORIZATION).and_then(|value| value.to_str().ok())
        && let Some(token) = value.strip_prefix("Bearer ")
    {
        return Some((token.trim().to_string(), false));
    }
    let cookies = parts.headers.get(COOKIE)?.to_str().ok()?;
    cookies.split(';').find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == cookie).then(|| (value.to_string(), true))
    })
}

/// The token a request carries, however it came.
pub fn token_of(parts: &Parts, cookie: &str) -> Option<String> {
    carried(parts, cookie).map(|(token, _)| token)
}

/// The address of a request, without where the API begins.
fn asked_at(parts: &Parts) -> &str {
    let path = parts.uri.path();
    path.strip_prefix("/api").unwrap_or(path)
}

/// Whether a request is a page a browser opened, whose own links it then
/// follows by itself: a page of a library, a mockup. Such a request can
/// carry nothing but the cookie, and is answered for it.
fn by_itself(parts: &Parts) -> bool {
    let path = asked_at(parts);
    parts.method == axum::http::Method::GET
        && (path.starts_with("/library/") || path.starts_with("/mockups/"))
}

/// What says, in the address, which page asks: `by`.
pub const SHOWN: &str = "by";

/// What a request the client's page has its browser make says in its
/// address beside the cookie (`by`), where it is one that only reads and
/// cannot carry the page's key: a file or a picture shown, something
/// attached, the stream of events. Said anywhere else, it is not heard:
/// an address is no way to ask for anything that changes something.
fn shown(parts: &Parts) -> Option<String> {
    // The three kinds of address, each exactly: a project's file as it
    // is, something attached, the stream. Not anything that begins or ends
    // as one of them does (a project that is called `raw`, an attachment
    // shown as a page, which the page asks for with its key).
    let parts_of: Vec<&str> = asked_at(parts).split('/').filter(|part| !part.is_empty()).collect();
    let fetched = matches!(parts_of.as_slice(), ["projects", _, "raw"] | ["attachments", _] | ["events"]);
    if parts.method != axum::http::Method::GET || !fetched {
        return None;
    }
    let query = parts.uri.query()?;
    query.split('&').find_map(|pair| {
        let (name, said) = pair.split_once('=')?;
        (name == SHOWN).then(|| said.to_string())
    })
}

/// How a request said who it is: its token, and how the token came. Kept
/// by what goes on answering after the request was let in (the stream of
/// events), so that it can ask again whether it would still be: a token
/// is the authority for as long as it is one, and no longer.
#[derive(Clone)]
pub struct Bearing {
    token: String,
    cookie: bool,
    page: Option<String>,
    shown: Option<String>,
    by_itself: bool,
}

impl Bearing {
    /// What `parts` carries, if it carries a token at all.
    pub fn of(parts: &Parts, app: &App) -> Option<Bearing> {
        let (token, cookie) = carried(parts, &app.place.cookie)?;
        let page = parts.headers.get(PAGE_KEY).and_then(|key| key.to_str().ok()).map(str::to_string);
        Some(Bearing { token, cookie, page, shown: shown(parts), by_itself: by_itself(parts) })
    }

    /// Who it is, as things stand now.
    pub async fn who(&self, app: &App) -> Result<Principal, Error> {
        let came = match self.cookie {
            false => studio::Came::Said,
            true => studio::Came::Cookie {
                page: self.page.as_deref(),
                shown: self.shown.as_deref(),
                by_itself: self.by_itself,
            },
        };
        app.studio.identity.authenticate_as(&self.token, came).await
    }
}

/// Who a request is from, if it says, held to how its token came.
pub async fn who(parts: &Parts, app: &App) -> Result<Principal, Error> {
    // Asked inside the daemon for somebody it had already let in, as they
    // were let in (`crate::commands`). Nothing sent to it can say this.
    if let Some(within) = parts.extensions.get::<crate::commands::Within>() {
        return Ok(within.0.clone());
    }
    let bearing = Bearing::of(parts, app).ok_or_else(|| Error::unauthenticated("sign in first"))?;
    bearing.who(app).await
}

/// Who is making the request. Every route that takes this refuses a request
/// without a valid credential; where the request came from proves nothing.
pub struct Auth(pub Principal);

/// Who is making the request, and what an event of what they do says did
/// it: the key, by its id, where they came with one somebody made, and
/// otherwise themselves. A key may hold less than the account it is of,
/// so what is done with one is said to be the key's doing wherever what
/// follows rests on what the doer holds.
pub struct Said {
    pub who: Principal,
    pub actor: String,
}

impl FromRequestParts<App> for Said {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, Self::Rejection> {
        // Whoever was let in says which key they came with, where it was
        // one: so does somebody the daemon asks for inside itself, as they
        // were let in when they said what it is doing for them
        // (`crate::commands`), who carries no token here.
        let who = who(parts, app).await?;
        let actor = who.key.as_ref().map(|id| format!("key:{id}")).unwrap_or_else(|| who.actor());
        Ok(Said { who, actor })
    }
}

impl FromRequestParts<App> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, Self::Rejection> {
        Ok(Auth(who(parts, app).await?))
    }
}
