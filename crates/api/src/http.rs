//! Errors as HTTP, and the measurement taken of every request.

use std::time::Instant;

use axum::Json;
use axum::extract::{MatchedPath, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use kernel::{Error, ErrorKind};
use serde_json::json;

use crate::state::App;

pub struct ApiError(pub Error);

pub type ApiResult<T> = Result<T, ApiError>;

impl<E: Into<Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        ApiError(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.kind {
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Invalid => StatusCode::BAD_REQUEST,
            ErrorKind::Denied => StatusCode::FORBIDDEN,
            ErrorKind::Unauthenticated => StatusCode::UNAUTHORIZED,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = json!({ "error": { "kind": self.0.kind.name(), "message": self.0.message } });
        (status, Json(body)).into_response()
    }
}

/// What kind of caller a token belongs to, from its prefix alone. Used only
/// to label measurements, never to decide anything.
fn caller(request: &Request) -> &'static str {
    match request.headers().get(axum::http::header::AUTHORIZATION).and_then(|value| value.to_str().ok()) {
        Some(value) if value.contains("agt_") => "agent",
        Some(value) if value.contains("dev_") => "device",
        Some(value) if value.contains("key_") => "script",
        Some(_) => "cli",
        None if request.headers().contains_key(axum::http::header::COOKIE) => "browser",
        None => "anonymous",
    }
}

/// Measures every request: which route, who by, how long, how it ended.
///
/// This is how "the daemon is busy, who is doing it?" gets answered from
/// data instead of from guesswork. On the previous system the answer, when
/// it was finally measured, was a forgotten browser tab.
pub async fn measure(State(app): State<App>, request: Request, next: Next) -> Response {
    let started = Instant::now();
    let route =
        request.extensions().get::<MatchedPath>().map_or("(unmatched)", MatchedPath::as_str).to_string();
    let method = request.method().as_str().to_string();
    let who = caller(&request);
    let response = next.run(request).await;
    let status = response.status();
    let class = match status.as_u16() {
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    };
    // The event stream is open for as long as someone is watching; its
    // "duration" is not a response time.
    if route != "/api/events" {
        let labels =
            [("route", route.as_str()), ("method", method.as_str()), ("status", class), ("caller", who)];
        app.journal.measure(
            "http",
            &labels,
            started.elapsed().as_secs_f64() * 1000.0,
            !status.is_server_error(),
        );
    }
    response
}
