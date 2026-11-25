//! What a request sent beyond what its route takes.
//!
//! A route reads so much of what it is sent and no more (a file to
//! attach, a record, a caller's words), and one sent more is refused.
//! Refused as soon as it was known, the refusal was answered while
//! whoever sent it was still sending, and the connection closed on what
//! they had yet to send: a system that is told so in the middle of
//! sending gives up the connection and whatever had come back on it, so
//! the client was told its connection was cut and never read why. It
//! showed on one system, once, with a megabyte over; it is so on any,
//! given enough left to send.
//!
//! So what is sent past what a route takes is read to its end and thrown
//! away before the refusal is answered: whoever sent it has finished
//! sending, reads the refusal, and may go on asking on the same
//! connection. It is read through only so far: [`READ_THROUGH`] times as
//! much again as the route had read when it refused. One that says it is
//! longer than that is not read on at all, and one that does not say is
//! given up at that length: the refusal is then answered with the
//! connection cut, as it was before, and may not be read. Nobody is held
//! to reading whatever they are sent for as long as it is sent.
//!
//! It is done in one place for every route there is, by what the answer
//! is (too large), and no route says anything for it: what a new route
//! takes is still its own limit.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::{Body, BodyDataStream, Bytes};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::CONTENT_LENGTH;
use axum::middleware::Next;
use axum::response::Response;
use futures_util::StreamExt;

/// How many times what a route had read when it refused is read on and
/// thrown away, at most, so that the refusal can be read: with the most
/// a route takes at thirty-two megabytes (a file attached), a request of
/// up to a hundred and sixty is read to its end.
pub const READ_THROUGH: u64 = 4;

/// What a request sends, read by its route and then, where the route
/// refused it as too large, by what throws the rest away.
struct Sent {
    /// What is still to come; nothing once it has ended or broken off.
    rest: tokio::sync::Mutex<Option<BodyDataStream>>,
    /// How many bytes of it have been read.
    read: AtomicU64,
}

impl Sent {
    async fn next(&self) -> Option<Result<Bytes, axum::Error>> {
        let mut rest = self.rest.lock().await;
        let piece = rest.as_mut()?.next().await;
        match &piece {
            Some(Ok(bytes)) => {
                self.read.fetch_add(bytes.len() as u64, Ordering::Relaxed);
            }
            _ => *rest = None,
        }
        piece
    }
}

/// Hands a request on to its route, and where the route answers that it
/// was sent too much, reads what is still being sent to its end before
/// that answer goes back.
pub async fn read_through(request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    let said = parts
        .headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let sent = Arc::new(Sent {
        rest: tokio::sync::Mutex::new(Some(body.into_data_stream())),
        read: AtomicU64::new(0),
    });
    let handed = futures_util::stream::unfold(sent.clone(), |sent| async move {
        let piece = sent.next().await?;
        Some((piece, sent))
    });
    let response = next.run(Request::from_parts(parts, Body::from_stream(handed))).await;
    if response.status() != StatusCode::PAYLOAD_TOO_LARGE {
        return response;
    }
    let read = sent.read.load(Ordering::Relaxed);
    let most = read.saturating_mul(READ_THROUGH);
    // One that says it is longer than is read through is not read on: it
    // would be read so far for nothing.
    if said.is_some_and(|said| said.saturating_sub(read) > most) {
        return response;
    }
    let mut thrown = 0u64;
    while thrown <= most {
        match sent.next().await {
            Some(Ok(bytes)) => thrown += bytes.len() as u64,
            // It has all been sent, or whoever was sending has gone.
            _ => break,
        }
    }
    response
}
