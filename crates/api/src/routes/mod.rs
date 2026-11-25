//! The routes.
//!
//! Grouped by what they are about: `work` (projects, blueprints, instances,
//! runs), `collab` (the board, records, agents), `talk` (channels),
//! `inbox` (what was addressed to whoever asks), `themes` (the client's
//! themes whoever asks made, and those offered to everybody), `arranged`
//! (how whoever asks arranged the side panel for themselves), `clocked`
//! (how a time is written for whoever asks), `appearance` (how what they
//! read looks for them),
//! `signins` (what agents' programs sign in with), `fleet` (this device,
//! its services, search, measurements, accounts), `devices` (the other
//! devices of the fleet, and the links to them) and `profiles` (the
//! other factories on this device), `compared` (one blueprint started
//! several ways, side by side). This file holds what is common
//! to all of them: signing in, and the event stream.

mod about;
mod across;
mod appearance;
mod archived;
mod arranged;
mod asked;
mod begun;
pub(crate) mod blueprinted;
mod bot;
mod channeldocs;
mod clocked;
mod collab;
mod compared;
mod conversed;
mod deploys;
mod devices;
mod environments;
mod examples;
mod experiments;
mod external;
mod files;
mod flags;
mod fleet;
pub(crate) mod follows;
pub mod git;
mod grouped;
mod inbox;
mod kept;
mod outlined;
mod outside;
mod posted;
mod profiles;
mod recordfiles;
mod releases;
mod renamed;
mod reviewed;
mod rows;
mod sandboxes;
mod secrets;
mod setups;
mod signins;
mod summary;
mod talk;
mod texts;
mod themes;
pub(crate) mod topics;
mod work;
mod workspaces;

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use futures_util::Stream;
use journal::EventQuery;
use kernel::Error;
use serde::Deserialize;
use serde_json::{Value, json};
use studio::SignedIn;

use crate::http::{ApiResult, measure};
use crate::state::{App, Auth};

pub fn router(app: App) -> Router {
    let api = Router::new()
        .route("/meta", get(meta))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/shows", get(shows))
        .route("/me", get(me))
        .route("/events", get(events))
        .route("/events/recent", get(recent_events))
        .merge(work::routes())
        .merge(files::routes())
        .merge(collab::routes())
        .merge(rows::routes())
        .merge(talk::routes())
        .merge(posted::routes())
        .merge(recordfiles::routes())
        .merge(channeldocs::routes())
        .merge(inbox::routes())
        .merge(follows::routes())
        .merge(reviewed::routes())
        .merge(asked::routes())
        .merge(begun::routes())
        .merge(themes::routes())
        .merge(arranged::routes())
        .merge(across::routes())
        .merge(clocked::routes())
        .merge(appearance::routes())
        .merge(conversed::routes())
        .merge(signins::routes())
        .merge(setups::routes())
        .merge(fleet::routes())
        .merge(grouped::routes())
        .merge(bot::routes())
        .merge(devices::routes())
        .merge(profiles::routes())
        .merge(environments::routes())
        .merge(flags::routes())
        .merge(experiments::routes())
        .merge(deploys::routes())
        .merge(git::routes())
        .merge(outside::routes())
        .merge(secrets::routes())
        .merge(compared::routes())
        .merge(texts::routes())
        .merge(blueprinted::routes())
        .merge(outlined::routes())
        .merge(topics::routes())
        .merge(crate::letgo::routes())
        .merge(crate::trash::routes())
        .merge(crate::opened::routes())
        .merge(summary::routes())
        .merge(sandboxes::routes())
        .merge(examples::routes())
        .merge(external::routes())
        .merge(releases::routes())
        .merge(about::routes())
        .merge(workspaces::routes())
        .merge(crate::mockups::api())
        .fallback(no_such_route);
    Router::new()
        .nest("/api", api)
        .merge(crate::documents::routes())
        .merge(crate::mockups::routes())
        .merge(outside::ways_in())
        .fallback(client)
        // Before any route, and measured like any other: whether this was
        // asked of this daemon at all (`guard`).
        .layer(middleware::from_fn_with_state(app.names.clone(), served::guard::guard))
        .layer(middleware::from_fn_with_state(app.clone(), measure))
        // Outside the measuring, which is of the route: what a request
        // sent beyond what its route takes is read to its end before it
        // is told so (`sent`).
        .layer(middleware::from_fn(served::sent::read_through))
        .with_state(app)
}

/// An API route this daemon does not have.
///
/// The client can be newer than the daemon it talks to, since replacing one
/// does not restart the other. This is what such a client is told when it
/// asks for something the daemon has not learned yet, in a form it can
/// recognise and pass on.
/// A file of the client, or its entry page for any path that is not a
/// file: the client does its own routing (`served::client::serve`).
async fn client(State(app): State<App>, uri: axum::http::Uri) -> Response {
    served::client::serve(&app.client, uri).await
}

async fn no_such_route(method: axum::http::Method, uri: axum::http::Uri) -> Response {
    // The path arrives without the prefix these routes are mounted under.
    let message = format!(
        "this daemon has no `{method} /api{}`; if what is asking is newer than the daemon, restart the daemon to bring it up to date",
        uri.path()
    );
    let body = json!({ "error": { "kind": "no_such_route", "message": message } });
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

/// A string field of a JSON body.
pub(crate) fn text<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(Value::as_str).filter(|text| !text.is_empty())
}

pub(crate) fn required<'a>(body: &'a Value, key: &str) -> Result<&'a str, Error> {
    text(body, key).ok_or_else(|| Error::invalid(format!("`{key}` is needed")))
}

/// What a client needs to know before anyone has signed in.
async fn meta(State(app): State<App>, parts: axum::http::request::Parts) -> ApiResult<Json<Value>> {
    // Whether this caller is signed in, so a client can show the sign-in
    // form straight away instead of asking who it is and being refused.
    let signed_in = crate::state::who(&parts, &app).await.is_ok();
    Ok(Json(json!({
        "signed_in": signed_in,
        "name": brand::DISPLAY,
        "id": brand::ID,
        "cli": brand::CLI,
        // Where support is asked for, as whoever built this said: the empty
        // text where they said nowhere, and then the client draws no way there.
        "support": brand::SUPPORT,
        "blueprint_ext": brand::BLUEPRINT_EXT,
        // Every ending a blueprint's file may have, the usual first.
        "blueprint_exts": brand::blueprint_endings(),
        // What a new project may start with, for whoever draws the choosing of it.
        "project_starts": crate::usual::listed(&crate::usual::the_usual(&app.engine).await.unwrap_or_default()),
        // What can be held, by area, and what an agent holds where nothing
        // is said of it: for whoever draws what somebody may do.
        "scopes": kernel::principal::areas(),
        "agent_scopes": kernel::principal::AGENT_SCOPES,
        // And what one this factory does not start is invited with,
        // where nothing is said: that and its own inbox.
        "external_scopes": agents::external::usual(),
        // What a nightly's package says it is (`0.1.0-nightly.20261124+abc1234`),
        // and otherwise the version it was built as.
        "version": crate::place::version(),
        // `installed` for a copy on its own, `checkout` for a program run
        // from where it was built, which reads that checkout as it is.
        "runs_from": crate::place::runs_from(),
        // Which build of the client is being served; it changes without the
        // daemon restarting.
        "client_build": app.client.build(&app.journal).await,
        "device": app.journal.device(),
        // What this factory's profile is called, for a client to say
        // where it is at all times; nothing for a factory with none.
        "profile": app.place.profile,
        // The same, under the name it had before 0.1.0, for a page from then.
        "environment": app.place.profile,
        "needs_setup": app.studio.identity.needs_setup().await?,
        // What every page says across its top to whoever is signed in: a
        // copy of another factory, and of what (`folders::across`).
        "across": if signed_in { json!(folders::across::read(&app.journal, app.place.profile.as_deref()).await) } else { Value::Null },
        // When it began draining to stop and the latest it stops, where
        // it was asked to (`crate::drain`): nothing where it was not.
        "draining": app.drain.said(),
        // Which of what is not yet whole this factory turned on (Settings, Experiments).
        "experiments": if signed_in { json!(experiments::on(&app).await?) } else { json!([]) },
    })))
}

/// A signed-in response: the token for clients that keep it themselves, and
/// a cookie for browsers, which is not readable by scripts and is not sent
/// with requests from other sites.
///
/// A browser sends that cookie to every port of the machine it came from,
/// and with any link followed there: so a page's session is good only in
/// the cookie, and for anything but what a browser fetches by itself only
/// with the key given here, which the page keeps where no other page on
/// the machine reads it (`studio::Came`). What says it is no page is given
/// the token and no cookie.
pub(crate) async fn signed_in(
    app: &App,
    signed: SignedIn,
    body: &Value,
    headers: &HeaderMap,
) -> ApiResult<Response> {
    if body.get("page").and_then(Value::as_bool) != Some(true) {
        return Ok(Json(json!({ "token": signed.token, "account": signed.account })).into_response());
    }
    // The cookie the browser sent as it signed in, if this is another page of the same browser.
    let cookies = headers.get(axum::http::header::COOKIE).and_then(|sent| sent.to_str().ok());
    let carried = cookies.into_iter().flat_map(|sent| sent.split(';')).find_map(|pair| {
        let (name, value) = pair.trim().split_once('=')?;
        (name == app.place.cookie).then_some(value)
    });
    let signed = app.studio.identity.for_page(signed, carried).await?;
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        app.place.cookie,
        signed.token,
        30 * 24 * 3600
    );
    // With its key, what the page says in the address of what its browser
    // fetches by itself (`state::SHOWN`).
    let shows = signed.page.as_deref().map(studio::Identity::shows);
    let body = json!({ "account": signed.account, "page": signed.page, "shows": shows });
    Ok((StatusCode::OK, [(SET_COOKIE, cookie)], Json(body)).into_response())
}

/// What a page says in the address of what its browser fetches by itself,
/// for a page that signed in before it was given that with its key. Asked
/// as the page asks anything, with its key, and so answered to nobody else.
async fn shows(Auth(_): Auth, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let key = headers.get(crate::state::PAGE_KEY).and_then(|key| key.to_str().ok());
    let key = key.ok_or_else(|| Error::invalid("only a page of this factory's own has anything to show"))?;
    Ok(Json(json!({ "shows": studio::Identity::shows(key) })))
}

async fn register(
    State(app): State<App>,
    ConnectInfo(from): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let signed = app
        .studio
        .identity
        .register(
            required(&body, "email")?,
            required(&body, "name")?,
            required(&body, "password")?,
            text(&body, "invite"),
            // Where the connection came from, which nothing in the request
            // can change: the first account is made on this machine.
            from.ip().is_loopback(),
        )
        .await?;
    // A factory has a workspace from when somebody owns it, called after
    // them: made here, where it is known that they just registered, and
    // by the next start where a stop falls between the two.
    if signed.account.role == "owner" {
        app.studio.settle().await?;
    }
    signed_in(&app, signed, &body, &headers).await
}

async fn login(
    State(app): State<App>,
    ConnectInfo(from): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<Response> {
    let (email, password) = (required(&body, "email")?, required(&body, "password")?);
    // Where the connection came from, which nothing in the request can
    // change: tries from this machine are counted apart from all others.
    let signed = app.studio.identity.sign_in_from(email, password, from.ip().is_loopback()).await?;
    signed_in(&app, signed, &body, &headers).await
}

/// Ends the session the request carries; or, asked to (`everywhere`),
/// every session of whoever asks, wherever each was begun. The second is
/// for somebody who is signed in and says so as any request does: not for
/// a cookie by itself, and not for a key, which is no session and is
/// withdrawn by its own name.
async fn logout(State(app): State<App>, request: axum::extract::Request) -> ApiResult<Response> {
    let (parts, body) = request.into_parts();
    let asked: Value = axum::body::to_bytes(body, 4096)
        .await
        .ok()
        .and_then(|sent| serde_json::from_slice(&sent).ok())
        .unwrap_or_default();
    let mut answer = json!({ "ok": true });
    if asked.get("everywhere").and_then(Value::as_bool) == Some(true) {
        let who = crate::state::who(&parts, &app).await?;
        let token = crate::state::token_of(&parts, &app.place.cookie).unwrap_or_default();
        if app.studio.identity.known(&token).await.is_ok_and(|(_, key)| key.is_some()) {
            return Err(Error::denied(format!(
                "a key is no session, and signs nobody out: sign in and ask again, or withdraw the key with `{} key revoke <id>`",
                brand::CLI
            ))
            .into());
        }
        answer["sessions"] = json!(app.studio.identity.sign_out_everywhere(&who).await?);
        // What is not a session goes on: said, so that nobody takes
        // signing out for withdrawing them.
        let keys = app.studio.identity.credentials().await?;
        let theirs = keys.iter().filter(|one| {
            one.kind == studio::CredentialKind::Key && one.subject == who.id && one.revoked.is_none()
        });
        answer["keys"] = json!(theirs.count());
    } else if let Some(token) = crate::state::token_of(&parts, &app.place.cookie) {
        app.studio.identity.sign_out(&token).await?;
    }
    let expired = format!("{}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0", app.place.cookie);
    Ok((StatusCode::OK, [(SET_COOKIE, expired)], Json(answer)).into_response())
}

async fn me(State(app): State<App>, Auth(who): Auth) -> ApiResult<Json<Value>> {
    let account = match who.kind {
        kernel::PrincipalKind::User => app.studio.identity.account(&who.id).await.ok(),
        _ => None,
    };
    let tier = app.studio.identity.tier().await?;
    // Whether a person has a password to sign in with another way than a
    // link: an owner the factory made has none until they set one.
    let password = match &account {
        Some(account) => app.studio.identity.has_password(&account.id).await?,
        None => false,
    };
    Ok(Json(json!({
        "principal": who,
        "actor": who.actor(),
        "account": account,
        "password": password,
        "tier": tier,
        "can_invite": tier.allows_team() && who.can("admin"),
    })))
}

#[derive(Deserialize)]
struct EventsQuery {
    /// Go on from this place: a `cursor` or a frame's `id` handed back as
    /// it was, or an event's number (`crate::cursor`). Without it, only
    /// what happens from now on is sent.
    since: Option<String>,
    /// Comma-separated topic patterns; everything when absent.
    topics: Option<String>,
}

/// The event stream: the one place a client learns that anything changed.
///
/// Each stored event carries the place just after it as the stream's
/// event id, so a client that reconnects (the browser does this by itself)
/// sends the last one it saw and misses nothing. The id is text to hand
/// back and nothing to read: an event's number is its `seq`.
///
/// Each caller is told what its credential lets it read and nothing else:
/// what is said in a channel is in the event that says so, and a line a
/// step printed is in the event that passes it on.
///
/// It goes on only while what opened it would still open it: a session
/// that was ended, or a key that was withdrawn, is told nothing more.
async fn events(
    State(app): State<App>,
    Query(query): Query<EventsQuery>,
    parts: axum::http::request::Parts,
) -> ApiResult<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>> {
    // Listening for withdrawals before asking who this is. The other way
    // round, a session ended between the asking and the listening was
    // never heard of: what listens afterwards has seen it already, and the
    // stream went on for somebody who was nobody.
    let withdrawn = app.studio.identity.withdrawn();
    let bearing = crate::state::Bearing::of(&parts, &app);
    let who = crate::state::who(&parts, &app).await?;
    // What a browser sends by itself is taken where it reads and passed
    // over where it does not, as it always was; what somebody wrote in the
    // address is theirs to be told of.
    let resumed = parts
        .headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| crate::cursor::read(value).ok());
    let asked = crate::cursor::said(query.since.as_deref())?;
    let from = resumed.or(asked);
    let since = from.clone().unwrap_or_else(|| app.journal.now());
    // What every journal says: each one's events in its own order, and
    // the journals' as they come.
    let follow = app.journal.follow_every(since.clone(), query.topics.unwrap_or_default());
    // Gone on from a place handed back, what was passed on since it and
    // never kept is in nothing the stream can read back: whoever follows
    // is told first that it is to read again what it shows of those
    // (`behind`). Followed from now, nothing is missed.
    let missed = from.is_some()
        && app
            .journal
            .every()
            .iter()
            .any(|journal| journal.passed_after(app.journal.place_in(&since, journal)));
    // And followed from now, it says where now is before anything else, so
    // that a browser that goes on after the connection broke goes on from
    // there whether or not it has been handed anything kept since: a frame
    // with a place and nothing in it, which a browser keeps and hands
    // back, and dispatches to nobody.
    let first = match (missed, from.is_some()) {
        (true, _) => Some(behind()),
        (false, false) => Some(SseEvent::default().id(crate::cursor::at(&since))),
        (false, true) => None,
    };
    let held = (follow, app, who, bearing, withdrawn, first);
    let stream = futures_util::stream::unfold(
        held,
        |(mut follow, app, mut who, bearing, mut withdrawn, first)| async move {
            if let Some(frame) = first {
                return Some((Ok(frame), (follow, app, who, bearing, withdrawn, None)));
            }
            loop {
                // A withdrawal is looked at before the next event is handed
                // on: nothing is told to a bearer after it stopped being one.
                let event = tokio::select! {
                    biased;
                    changed = withdrawn.changed() => {
                        // Something that let somebody in was withdrawn. If it
                        // was what opened this, it is over; what they hold may
                        // also have changed, and is read again.
                        changed.ok()?;
                        if let Some(bearing) = &bearing {
                            who = bearing.who(&app).await.ok()?;
                        }
                        continue;
                    }
                    event = follow.next_or_behind() => match event.ok()? {
                        Some(event) => event,
                        // Fell behind what is said as it is said: what was
                        // passed on meanwhile is gone, and whoever follows
                        // is told so.
                        None => return Some((Ok(behind()), (follow, app, who, bearing, withdrawn, None))),
                    },
                };
                if !app.may_hear(&who, &event) {
                    continue;
                }
                let mut frame = SseEvent::default().json_data(&*event).unwrap_or_default();
                // Only stored events move the client's position; a passing log
                // line must not make it think it has seen everything up to zero.
                if event.is_durable() {
                    frame = frame.id(crate::cursor::at(&follow.after(&event)));
                }
                return Some((Ok(frame), (follow, app, who, bearing, withdrawn, None)));
            }
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20)).text("still here")))
}

/// What the stream says where what it follows may have missed something
/// that was passed on as it was said and not kept (`journal::Writing::pass_on`):
/// a frame of its own kind, `behind`, which names the topics that may be
/// missing, moves no place and says nothing anybody may not hear. A page
/// reads again whatever it shows that it follows on one of those.
fn behind() -> SseEvent {
    SseEvent::default()
        .event("behind")
        .json_data(json!({ "topics": conductor::PASSED_ON }))
        .unwrap_or_default()
}

/// The most events handed at once by [`recent_events`], whatever is asked.
const MOST_RECENT: usize = 1000;

#[derive(Deserialize)]
struct RecentQuery {
    topics: Option<String>,
    instance: Option<String>,
    run: Option<String>,
    /// Only what was said after this place (`crate::cursor`).
    after: Option<String>,
    limit: Option<usize>,
}

/// Stored events, newest first: for a view that has just been opened.
async fn recent_events(
    State(app): State<App>,
    Auth(who): Auth,
    Query(query): Query<RecentQuery>,
) -> ApiResult<Json<Value>> {
    let after = crate::cursor::said(query.after.as_deref())?;
    // Where the journal has got to is read first, and nothing after it is
    // handed out here. Whoever asked follows the stream from that place
    // next (`cursor`, handed back as `since`): read the other way round,
    // an event written between the two reads was in neither, and whoever
    // waited for it waited for ever.
    let now = app.journal.now();
    // As many as are read at once, at most: a list cut there says so
    // (`most`, and `more` where it may be), and is not taken for all there
    // is (`events --limit 5000` handed 1000 with nothing said).
    let most = query.limit.unwrap_or(100).min(MOST_RECENT);
    let asked = EventQuery {
        topics: query.topics.unwrap_or_default(),
        instance: query.instance,
        run: query.run,
        limit: most,
        newest_first: true,
        ..Default::default()
    };
    // Of every journal, in the order of when each thing was said.
    let events = app.journal.read_within(&after.unwrap_or_default(), &now, asked).await?;
    let more = events.len() >= most;
    // Only what this caller may read, as on the stream.
    let events: Vec<_> = events.into_iter().filter(|event| app.may_hear(&who, event)).collect();
    // `last_seq` is the highest number an event has, as it always was;
    // `cursor` is the place to go on from, which is what to keep.
    let last_seq = app.journal.highest();
    Ok(Json(json!({
        "events": events,
        "last_seq": last_seq,
        "cursor": crate::cursor::at(&now),
        "most": most,
        "more": more,
    })))
}
