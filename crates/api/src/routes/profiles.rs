//! The factories on this device, for a page that offers the way to another.
//!
//! A page is in one factory, and is told of the others by name and by an
//! address it can open. The address has to be one that works from where
//! the page is, and that is not always knowable: another factory listening
//! on this device only cannot be opened from a phone, and a page that came
//! through a proxy came through one that leads here and nowhere else. Where
//! no address can honestly be given, none is: the factory is listed with
//! the reason instead, and nothing is guessed.

use std::net::{IpAddr, SocketAddr};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::HOST;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::http::ApiResult;
use crate::state::{App, Auth};

pub fn routes() -> Router<App> {
    // `/environments` is what a page from before 0.1.0, left open as the
    // daemon is replaced, asks: it is answered the same.
    Router::new().route("/profiles", get(profiles)).route("/environments", get(profiles))
}

/// How a page was reached, as its request says.
pub(crate) struct Reached<'a> {
    /// The request's `Host`: the name and port the page was asked for by.
    pub host: Option<&'a str>,
    /// Something between the page and the daemon said it passed the request on.
    pub forwarded: bool,
}

/// Headers a proxy adds to what it passes on.
const FORWARDING: &[&str] = &["forwarded", "x-forwarded-for", "x-forwarded-host", "x-forwarded-proto", "via"];

/// A `Host` as its name and its port, the port being 80 when it has none:
/// the daemon is asked over plain HTTP.
fn split(host: &str) -> (&str, u16) {
    // An IPv6 address is in brackets, and has colons of its own.
    let colon = match host.rfind(']') {
        Some(bracket) => host[bracket..].find(':').map(|at| bracket + at),
        None => host.rfind(':'),
    };
    match colon.and_then(|at| host[at + 1..].parse().ok().map(|port| (&host[..at], port))) {
        Some(parts) => parts,
        None => (host, 80),
    }
}

/// The address a name stands for when the name is one: `localhost` and
/// addresses written out. Any other name is somewhere this cannot know.
fn literal(name: &str) -> Option<IpAddr> {
    if name.eq_ignore_ascii_case("localhost") {
        return Some(IpAddr::from([127, 0, 0, 1]));
    }
    name.trim_start_matches('[').trim_end_matches(']').parse().ok()
}

/// Where a page that was reached as `reached`, in the factory listening on
/// `own`, can open the factory called `name` that listens on `other`: an
/// address, or the reason there is none to give.
pub(crate) fn way_to(
    own: SocketAddr,
    reached: &Reached<'_>,
    name: &str,
    other: SocketAddr,
) -> Result<String, String> {
    let Some(host) = reached.host else {
        return Err(format!(
            "The request did not say where this page was reached, so there is no saying where `{name}` \
             would be from there. On the device itself it answers at {other}."
        ));
    };
    let (page, port) = split(host);
    // A page that came straight to this daemon came to the port it listens
    // on, and nothing in between said otherwise.
    if reached.forwarded || port != own.port() {
        return Err(format!(
            "This page came through {host}, which leads to this factory and to no other. `{name}` answers \
             at {other} on the device itself; to open it from here, give it an address of its own in the \
             same way."
        ));
    }
    let page_is = literal(page);
    let on_device = page_is.is_some_and(|address| address.is_loopback());
    let ip = other.ip();
    if ip.is_loopback() {
        if on_device {
            return Ok(format!("http://{other}"));
        }
        return Err(format!(
            "`{name}` listens on the device itself only ({other}), and this page was reached from elsewhere \
             ({host}). Open it on the device, or start it listening on an address that can be reached."
        ));
    }
    if ip.is_unspecified() {
        // It listens on every address of the device, of its own kind: the
        // one this page came to is one of them.
        if page_is.is_none_or(|address| address.is_ipv4() == ip.is_ipv4()) {
            return Ok(format!("http://{page}:{}", other.port()));
        }
        return Err(format!(
            "`{name}` listens on {other}, which is not the kind of address this page was reached at ({host})."
        ));
    }
    // One address of the device: the page is there too, or on the device,
    // which reaches all of its own.
    if on_device || page_is == Some(ip) {
        return Ok(format!("http://{other}"));
    }
    Err(format!(
        "`{name}` listens on {other}, and this page was reached at {host}: whether one can be reached from \
         where the other was is not something this factory knows."
    ))
}

/// The named factories on this device: what each is called, whether it is
/// this one, and where the page that asked can open it, or why it cannot.
async fn profiles(State(app): State<App>, Auth(who): Auth, headers: HeaderMap) -> ApiResult<Json<Value>> {
    // Which other factories are on this device, and where, is the fleet's to tell.
    who.require("fleet:read")?;
    let reached = Reached {
        host: headers.get(HOST).and_then(|host| host.to_str().ok()),
        forwarded: FORWARDING.iter().any(|name| headers.contains_key(*name)),
    };
    let registry = app.place.registry.clone();
    let mut factories = match registry {
        Some(registry) => {
            tokio::task::spawn_blocking(move || registry.factories()).await.map_err(|err| {
                kernel::Error::internal(format!("reading the factories on this device crashed: {err}"))
            })??
        }
        None => Vec::new(),
    };
    // This one is listed whether or not the list could be written to.
    if let Some(name) = &app.place.profile
        && !factories.iter().any(|factory| factory.data == app.place.data)
    {
        factories.push(crate::registry::Factory {
            name: name.clone(),
            data: app.place.data.clone(),
            address: app.addr,
        });
        factories.sort_by(|a, b| a.name.cmp(&b.name));
    }
    let listed: Vec<Value> = factories
        .iter()
        .map(|factory| {
            if factory.data == app.place.data {
                return json!({ "name": factory.name, "this": true, "url": null, "why": null });
            }
            match way_to(app.addr, &reached, &factory.name, factory.address) {
                Ok(url) => json!({ "name": factory.name, "this": false, "url": url, "why": null }),
                Err(why) => json!({ "name": factory.name, "this": false, "url": null, "why": why }),
            }
        })
        .collect();
    // And said under the names it had before 0.1.0, for a page from then.
    Ok(Json(json!({
        "profile": app.place.profile, "profiles": listed.clone(),
        "environment": app.place.profile, "environments": listed,
    })))
}
