//! A member's way to its hub: sealed, and carried over HTTP.
//!
//! [`SealedUplink`] is the link as the member knows it. It shakes hands
//! (`crate::sealed`), and from then on everything it sends is a sealed frame
//! and everything it takes must open as one. It holds the hub to the public
//! key this device joined: whatever answers without proving that key is not
//! the hub, whatever address it answers at, and nothing it says is acted on.
//!
//! [`Carrier`] is what takes the bytes there and back, and knows nothing of
//! what they are. [`HttpCarrier`] does it over plain HTTP: one exchange to
//! shake hands, one long request the hub answers with frames as they come,
//! and one exchange for each report. Plain HTTP is enough because nothing
//! that crosses depends on what carries it: what is between the two can
//! read nothing and change nothing, and can only keep the two apart.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use kernel::{BoxFut, Cancel};
use reqwest::StatusCode;
use reqwest::header::{ACCEPT, CONTENT_TYPE, RETRY_AFTER};
use tokio::sync::mpsc;

use crate::door::{Hello, SESSION, Welcome, decode};
use crate::own::{KEY, Keys, Said};
use crate::sealed::{self, Kind, MAX_SEALED, Session};
use crate::wire::{Ack, Command, HELLO_PATH, LINK_PATH, LinkError, REPORT_PATH, Report, Uplink};

/// A link that says nothing for this long is taken to be dead. The hub
/// sends a keep-alive well inside it. Asking for the link is given no
/// longer either: whatever takes the request and never answers it has
/// kept nothing waiting past this.
pub(crate) const SILENCE: Duration = Duration::from_secs(60);

/// The most that is read of one answer: a session's name and the longest
/// frame there is. What answers has proved nothing when it is read, so
/// what it sends is not let grow past what a hub would send.
pub const MAX_ANSWER: usize = SESSION + MAX_SEALED;
/// The most that is read of the stream between one frame and the next: a
/// frame in letters, and the line it is on.
const MAX_LINE: usize = MAX_SEALED / 3 * 4 + 1024;
/// The most that is read of words where something sealed was wanted, and
/// the most of them that is kept: a line, for whoever has to find out what
/// is wrong. They are not the hub's until proved, and steer nothing.
const MAX_WORDS_READ: usize = 4096;
const MAX_WORDS: usize = 200;
/// The longest wait that is taken from an answer that is not sealed.
const MAX_ASKED: Duration = Duration::from_secs(60);

/// What takes bytes to the hub and brings its answer back.
pub trait Carrier: Send + Sync + 'static {
    /// Where the hub is, as a person would say it.
    fn place(&self) -> String;

    /// One exchange: `body` to `path`, and what came back.
    fn exchange(&self, path: &'static str, body: Vec<u8>) -> BoxFut<'static, Result<Vec<u8>, LinkError>>;

    /// The long exchange: `body` to `path`, and what comes back a piece at a
    /// time for as long as the hub keeps it open.
    fn stream(
        &self,
        path: &'static str,
        body: Vec<u8>,
    ) -> BoxFut<'static, Result<mpsc::Receiver<Vec<u8>>, LinkError>>;
}

pub struct HttpCarrier {
    /// What sends, or why there is nothing to send with.
    client: Result<reqwest::Client, String>,
    url: String,
}

impl HttpCarrier {
    pub fn new(url: &str) -> HttpCarrier {
        // Sent nowhere but where it was told: what answers cannot send
        // this device's requests on to another address. Where a client
        // cannot be made as asked there is none, and nothing is sent: not
        // one made some other way, which would go where it is sent on to.
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| err.to_string());
        HttpCarrier { client, url: url.trim_end_matches('/').to_string() }
    }

    fn client(&self) -> Result<reqwest::Client, LinkError> {
        self.client.clone().map_err(|why| {
            LinkError::new(format!(
                "this device could not make what carries its link to {} ({why}); nothing was sent",
                self.url
            ))
        })
    }
}

/// Reads what was answered, and no more of it than `most`: past that it is
/// not what a hub sends, and reading stops.
async fn read_no_more(mut response: reqwest::Response, most: usize, url: &str) -> Result<Vec<u8>, LinkError> {
    let mut read = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|err| unreachable(url, &err))? {
        if read.len() + chunk.len() > most {
            return Err(LinkError::new(format!(
                "what answers at {url} sent more than a hub sends at once; it was not read"
            )));
        }
        read.extend_from_slice(&chunk);
    }
    Ok(read)
}

/// One line of what was said, cut to a line's length, with nothing in it
/// that is not a letter someone could read.
fn a_line(words: &str) -> String {
    let line: String =
        words.chars().map(|letter| if letter.is_control() { ' ' } else { letter }).take(MAX_WORDS).collect();
    line.trim().to_string()
}

/// What goes before words that came unsealed, wherever they are shown.
pub const UNPROVEN: &str = "it said, unproven: ";

/// Words that came unsealed, said as that: a line of them, after what marks
/// them as anybody's. Whoever reads them, in a status, an event or an
/// error, is not left to take them for the hub's.
pub fn marked(words: &str) -> String {
    match words.strip_prefix(UNPROVEN) {
        Some(_) => a_line(words),
        None => a_line(&format!("{UNPROVEN}{words}")),
    }
}

/// What was said where a sealed answer was wanted. Not sealed, so not to be
/// believed: kept for the person who has to find out what is wrong.
fn said(status: StatusCode, retry_after: Option<Duration>, body: &str) -> LinkError {
    let words = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|body| body["error"]["message"].as_str().map(str::to_string))
        .unwrap_or_else(|| format!("what is there answered {status}"));
    // "Not now" is not "no": a hub on its way down, or one that has heard
    // too much from here, is asked again later.
    let later = matches!(status, StatusCode::SERVICE_UNAVAILABLE | StatusCode::TOO_MANY_REQUESTS);
    LinkError { retry_after, said: !later, ..LinkError::new(marked(&words)) }
}

/// A failure of what carries the link, with any words in it that came
/// unsealed said as that.
fn plainly(err: LinkError) -> LinkError {
    if err.said { LinkError { message: marked(&err.message), ..err } } else { err }
}

fn unreachable(url: &str, err: &reqwest::Error) -> LinkError {
    let why = if err.is_connect() {
        "nothing is answering there".to_string()
    } else if err.is_timeout() {
        "it did not answer in time".to_string()
    } else {
        err.to_string()
    };
    LinkError::new(format!("could not reach the hub at {url}: {why}"))
}

/// What is said of a hub that took the asking for a link and never answered.
pub(crate) fn unanswered(place: &str) -> String {
    format!("what is at {place} took this device's asking for its link and did not answer in time")
}

fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    let seconds: u64 = response.headers().get(RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()?;
    // Asked for in the clear, by whatever answered: obeyed for a minute at
    // most, so that nothing between the two can keep a device away.
    Some(Duration::from_secs(seconds).min(MAX_ASKED))
}

/// Pulls complete events out of what has arrived so far, leaving any
/// unfinished one in `buffer`. Only `data:` lines matter; the rest (ids,
/// keep-alive comments) is the stream's own business.
pub fn take_events(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();
    while let Some(end) = buffer.find("\n\n") {
        let frame: String = buffer.drain(..end + 2).collect();
        let data: Vec<&str> = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(|line| line.strip_prefix(' ').unwrap_or(line))
            .collect();
        if !data.is_empty() {
            events.push(data.join("\n"));
        }
    }
    events
}

impl Carrier for HttpCarrier {
    fn place(&self) -> String {
        self.url.clone()
    }

    fn exchange(&self, path: &'static str, body: Vec<u8>) -> BoxFut<'static, Result<Vec<u8>, LinkError>> {
        let client = self.client();
        let url = self.url.clone();
        Box::pin(async move {
            let request = client?
                .post(format!("{url}{path}"))
                .header(CONTENT_TYPE, "application/octet-stream")
                .timeout(Duration::from_secs(30))
                .body(body);
            let response = request.send().await.map_err(|err| unreachable(&url, &err))?;
            if !response.status().is_success() {
                let (status, wait) = (response.status(), retry_after(&response));
                let words = read_no_more(response, MAX_WORDS_READ, &url).await.unwrap_or_default();
                return Err(said(status, wait, &String::from_utf8_lossy(&words)));
            }
            read_no_more(response, MAX_ANSWER, &url).await
        })
    }

    fn stream(
        &self,
        path: &'static str,
        body: Vec<u8>,
    ) -> BoxFut<'static, Result<mpsc::Receiver<Vec<u8>>, LinkError>> {
        let client = self.client();
        let url = self.url.clone();
        Box::pin(async move {
            let request = client?
                .post(format!("{url}{path}"))
                .header(CONTENT_TYPE, "application/octet-stream")
                .header(ACCEPT, "text/event-stream")
                .body(body);
            // Given the silence a link is allowed and no longer: what takes
            // the request and never answers it is not waited on for ever.
            let response = match tokio::time::timeout(SILENCE, request.send()).await {
                Ok(response) => response.map_err(|err| unreachable(&url, &err))?,
                Err(_) => return Err(LinkError::new(unanswered(&url))),
            };
            if !response.status().is_success() {
                let (status, wait) = (response.status(), retry_after(&response));
                let words = read_no_more(response, MAX_WORDS_READ, &url).await.unwrap_or_default();
                return Err(said(status, wait, &String::from_utf8_lossy(&words)));
            }
            let (pieces, stream) = mpsc::channel(256);
            tokio::spawn(async move {
                let mut body = response.bytes_stream();
                let mut buffer = String::new();
                // Ends, closing the channel, when the hub hangs up, goes
                // quiet, sends a line that is not a frame's, or sends more
                // without ending a frame than the longest frame is.
                loop {
                    let chunk = tokio::select! {
                        chunk = tokio::time::timeout(SILENCE, body.next()) => chunk,
                        // Nobody is taking what comes any more: the link
                        // is let go now, not at whatever comes next.
                        () = pieces.closed() => return,
                    };
                    let Ok(Some(Ok(chunk))) = chunk else { return };
                    if buffer.len() + chunk.len() > MAX_LINE {
                        return;
                    }
                    buffer.push_str(&String::from_utf8_lossy(&chunk).replace("\r\n", "\n"));
                    for event in take_events(&mut buffer) {
                        let Some(piece) = decode(&event) else { return };
                        if pieces.send(piece).await.is_err() {
                            return;
                        }
                    }
                }
            });
            Ok(stream)
        })
    }
}

/// One handshake's session, as the member holds it.
struct Up {
    name: [u8; SESSION],
    session: Session,
    /// Raised when the session is found to be no good, so that the link it
    /// carries ends and another is made.
    broken: Cancel,
}

/// The link to a hub, sealed, over whatever carries it.
/// What a handshake is made with: what carries it, this device's keys, the
/// key its hub must prove, and the number its first message carries.
#[derive(Clone)]
struct Hand {
    carrier: Arc<dyn Carrier>,
    keys: Keys,
    hub: [u8; KEY],
    said: Arc<Said>,
    /// The time, in milliseconds: no number is below it.
    now: Arc<dyn Fn() -> i64 + Send + Sync>,
}

pub struct SealedUplink {
    hand: Hand,
    up: Arc<Mutex<Option<Arc<Up>>>>,
}

impl SealedUplink {
    /// A link to the hub whose public key is `hub`, by `carrier`, proving
    /// this device with `keys`. Each first message of a handshake carries
    /// the next number of `said`, which is never below `now`.
    pub fn new(
        carrier: Arc<dyn Carrier>,
        keys: Keys,
        hub: [u8; KEY],
        said: Arc<Said>,
        now: Arc<dyn Fn() -> i64 + Send + Sync>,
    ) -> SealedUplink {
        SealedUplink { hand: Hand { carrier, keys, hub, said, now }, up: Arc::default() }
    }

    /// Shakes hands, handing over `invitation` when there is one to redeem,
    /// and says what the hub called itself and this device.
    pub fn hello(&self, invitation: Option<String>) -> BoxFut<'static, Result<Welcome, LinkError>> {
        let (hand, kept) = (self.hand.clone(), self.up.clone());
        Box::pin(async move {
            let (up, welcome) = shake(&hand, invitation).await?;
            *kept.lock().expect("uplink lock") = Some(Arc::new(up));
            Ok(welcome)
        })
    }
}

/// What a member says when what answers is not its hub.
fn not_the_hub(place: &str, how: &str) -> LinkError {
    LinkError {
        unproven: true,
        ..LinkError::new(format!(
            "what answers at {place} did not prove it is the hub this device joined ({how}). Nothing it asks is done. \
             If the hub was made again, or this is another installation at its address, ask for an invitation there and join again"
        ))
    }
}

async fn shake(hand: &Hand, invitation: Option<String>) -> Result<(Up, Welcome), LinkError> {
    let place = hand.carrier.place();
    let unkept = |err: std::io::Error| {
        LinkError::new(format!("could not keep the number this device's handshakes are counted by: {err}"))
    };
    let mut number = hand.said.next((hand.now)()).map_err(unkept)?;
    // Twice at most: once more if the hub, having proved itself, says it
    // has taken a higher number from this device than this device knew.
    for again in [true, false] {
        let hello = serde_json::to_vec(&Hello { invitation: invitation.clone(), number }).unwrap_or_default();
        let offer =
            sealed::offer(&hand.keys, &hand.hub, &hello).map_err(|err| LinkError::new(err.to_string()))?;
        let answer = match hand.carrier.exchange(HELLO_PATH, offer.message.clone()).await {
            Ok(answer) => answer,
            // Words that were not sealed, where the hub's half of the
            // handshake was wanted: the hub would have answered with that.
            Err(err) if err.said => return Err(not_the_hub(&place, &marked(&err.message))),
            Err(err) => return Err(err),
        };
        if answer.len() < SESSION {
            return Err(not_the_hub(&place, "its answer was no handshake"));
        }
        let (name, message) = answer.split_at(SESSION);
        let (session, payload) = offer
            .accept(message)
            .map_err(|_| not_the_hub(&place, "its answer was not made with that hub's key"))?;
        let welcome: Welcome = serde_json::from_slice(&payload).unwrap_or_default();
        if welcome.ok {
            let up = Up { name: name.try_into().expect("a session's name"), session, broken: Cancel::new() };
            return Ok((up, welcome));
        }
        // Sealed by the hub itself from here on: this is the hub speaking.
        if let (Some(above), true) = (welcome.above, again) {
            let now = (hand.now)();
            if !Said::sensible(above, now) {
                // Counted up to by nobody: taken, it would leave this
                // device with no number left to say.
                return Err(not_the_hub(
                    &place,
                    "it told this device to count its handshakes from a number no clock will reach",
                ));
            }
            number = hand.said.above(above, now).map_err(unkept)?;
            continue;
        }
        let why = welcome.error.unwrap_or_else(|| "the hub would not let this device in".to_string());
        return Err(LinkError { refused: true, out: welcome.unknown, ..LinkError::new(why) });
    }
    Err(LinkError::new("the hub and this device could not agree which handshake this is"))
}

impl Uplink for SealedUplink {
    fn open(&self) -> BoxFut<'static, Result<mpsc::Receiver<Command>, LinkError>> {
        let (hand, kept) = (self.hand.clone(), self.up.clone());
        let carrier = hand.carrier.clone();
        Box::pin(async move {
            // A new handshake for each link: whatever session there was is
            // over, on this side as on the hub's.
            if let Some(old) = kept.lock().expect("uplink lock").take() {
                old.broken.cancel();
            }
            let (up, _) = shake(&hand, None).await?;
            let up = Arc::new(up);
            let (_, opening) =
                up.session.seal(Kind::Open, &[]).map_err(|err| LinkError::new(err.to_string()))?;
            let mut pieces =
                carrier.stream(LINK_PATH, [up.name.as_slice(), &opening].concat()).await.map_err(plainly)?;
            *kept.lock().expect("uplink lock") = Some(up.clone());
            let (commands, stream) = mpsc::channel(256);
            tokio::spawn(async move {
                loop {
                    let piece = tokio::select! {
                        piece = pieces.recv() => piece,
                        () = up.broken.cancelled() => None,
                        // The member has stopped listening (it was halted,
                        // or left): the link ends now, so that the hub sees
                        // it go and not only when it next has work for it.
                        () = commands.closed() => None,
                    };
                    // Ends, and with it the link, at the first thing that is
                    // not the next frame of this session: changed, said
                    // before, or not a command.
                    let Some(piece) = piece else { return };
                    let Ok((_, content)) = up.session.open_next(Kind::Command, &piece) else { return };
                    let Ok(command) = serde_json::from_slice::<Command>(&content) else { return };
                    if commands.send(command).await.is_err() {
                        return;
                    }
                }
            });
            Ok(stream)
        })
    }

    fn report(&self, report: Report) -> BoxFut<'static, Result<Ack, LinkError>> {
        let carrier = self.hand.carrier.clone();
        let up = self.up.lock().expect("uplink lock").clone();
        Box::pin(async move {
            let Some(up) = up else {
                return Err(LinkError::new("there is no link to the hub to report over"));
            };
            let content = serde_json::to_vec(&report).unwrap_or_default();
            if content.len() > sealed::MAX_FRAME {
                return Err(LinkError {
                    too_much: true,
                    ..LinkError::new("more than is said at once over a link")
                });
            }
            let (number, frame) =
                up.session.seal(Kind::Report, &content).map_err(|err| LinkError::new(err.to_string()))?;
            let answer = match carrier.exchange(REPORT_PATH, [up.name.as_slice(), &frame].concat()).await {
                Ok(answer) => answer,
                Err(err) => {
                    if err.said {
                        // The hub does not have this session, or would not
                        // take the frame: the link is made again.
                        up.broken.cancel();
                    }
                    return Err(plainly(err));
                }
            };
            // The answer must be the hub's, of this session, and to this
            // report and no other: it begins with the report's own number.
            let taken = up.session.open_any(Kind::Ack, &answer).ok().and_then(|(_, _, content)| {
                let (to, ack) = content.split_at_checked(8)?;
                (u64::from_be_bytes(to.try_into().ok()?) == number).then(|| ack.to_vec())
            });
            let Some(ack) = taken else {
                up.broken.cancel();
                return Err(LinkError::new("what came back for a report was not the hub's answer to it"));
            };
            serde_json::from_slice::<Ack>(&ack)
                .map_err(|err| LinkError::new(format!("the hub's answer made no sense: {err}")))
        })
    }
}
