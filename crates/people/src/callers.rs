//! A caller: someone outside who proves each thing they say by signing it.
//!
//! A key in an address (the daemon's `routes::outside`) says who is
//! calling by what the address holds, so whoever reads the address can call. A service that
//! signs what it sends proves more: that it holds a secret the address
//! never carries, and that what arrived is what it sent. A caller is the
//! description of one such service (which header its signature is in, what
//! it signed, with which of the factory's secrets, and the one topic it may
//! say) and the check of a call against that description.
//!
//! The proof is HMAC-SHA256, which is what such services sign with, and is
//! the `hmac` crate's: nothing here works a proof out for itself. What
//! differs from one service to the next is only where things are put, so
//! that is all a description says:
//!
//! - **where the signature is**: a header, and what comes before the
//!   signature in it (`sha256=`, `v1=`, `v1,`). A header may hold several,
//!   as it does while a service changes its secret; any one may match.
//! - **what was signed**: a line with `{body}` in it, and `{time}` or
//!   `{header:<name>}` where the service signs those too.
//! - **when it was signed**, where the service says: a call older than
//!   `within` is not taken, and one that was taken is not taken again while
//!   it could still be sent again ([`Seen`]).
//!
//! A service that signs no time can be sent again by anyone who saw the
//! call, for as long as its secret stands. That is the service's choice
//! and not something that can be made up for here; `docs/blueprints.md`
//! says so where it describes callers.
//!
//! It came out of the daemon (`api`), which offers it under the name it
//! always had (`api::callers`): a caller is a kind of credential, which is
//! this crate's, and what it was handed of a request (its headers) it is
//! handed as a lookup by name, which the route answers from the request.

use std::collections::{HashMap, VecDeque};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use blueprint::RESERVED_TOPICS;
use blueprint::validate::is_topic;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Where callers are kept in the journal's small durable state. Not in a
/// copy of the factory: a copy must not answer for the one it was taken of,
/// and has none of its secrets to check with.
pub const KEPT_AS: &str = "callers";

/// How many signatures one header may offer. A service that is changing its
/// secret sends two; more than a few is somebody making work.
const OFFERED: usize = 8;

/// How many calls one caller's taken calls are remembered for, at most.
const REMEMBERED: usize = 10_000;

/// How something is written down as text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Written {
    /// As it is.
    Text,
    #[default]
    Hex,
    Base64,
}

fn as_text() -> Written {
    Written::Text
}

fn the_body() -> String {
    "{body}".to_string()
}

fn five_minutes() -> String {
    "5m".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Caller {
    /// The one topic a call that checks is said as.
    pub topic: String,
    /// Which of the factory's secrets the caller signs with, by name.
    pub secret: String,
    /// The header its signature is sent in.
    pub header: String,
    /// What comes before the signature in that header.
    #[serde(default)]
    pub prefix: String,
    /// How the signature is written.
    #[serde(default)]
    pub encoding: Written,
    /// How the secret is written: most are the key as they are, and some
    /// are the key in base64.
    #[serde(default = "as_text")]
    pub key: Written,
    /// What was signed.
    #[serde(default = "the_body")]
    pub signed: String,
    /// The header that says when it was signed, when that is not the header
    /// the signature is in.
    #[serde(default)]
    pub time_header: String,
    /// What comes before the time in its header.
    #[serde(default)]
    pub time_prefix: String,
    /// How old a call may be.
    #[serde(default = "five_minutes")]
    pub within: String,
}

/// Why `name` may not be a caller's name, if it may not. A name is the end
/// of an address, and is said in events.
pub fn misnamed(name: &str) -> Option<String> {
    let mut letters = name.chars();
    let right = letters.next().is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
        && name.len() <= 100;
    (!right).then(|| {
        format!(
            "`{name}` cannot be a caller's name: use lowercase letters, digits, `-` and `_`, beginning with a \
             letter or a digit, such as `payments`"
        )
    })
}

/// A part of what was signed.
enum Part<'a> {
    Said(&'a str),
    Body,
    Time,
    Header(&'a str),
}

/// A call that checked.
#[derive(Debug, PartialEq, Eq)]
pub struct Proved {
    /// The signature that matched: the bytes it stood for, as hex.
    pub signature: String,
    /// When the caller says it signed, in milliseconds since 1970, if it
    /// says.
    pub at: Option<i64>,
}

impl Caller {
    /// Whether the caller says when it signed.
    pub fn timed(&self) -> bool {
        !self.time_header.is_empty() || !self.time_prefix.is_empty()
    }

    fn time_header(&self) -> &str {
        if self.time_header.is_empty() { &self.header } else { &self.time_header }
    }

    pub fn within_ms(&self) -> Result<i64, String> {
        blueprint::duration::parse(&self.within).map(|ms| ms as i64).map_err(|err| format!("`within`: {err}"))
    }

    fn parts(&self) -> Result<Vec<Part<'_>>, String> {
        let (mut parts, mut rest) = (Vec::new(), self.signed.as_str());
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let (before, name) = (&rest[..open], &rest[open + 1..open + close]);
            if !before.is_empty() {
                parts.push(Part::Said(before));
            }
            parts.push(match name {
                "body" => Part::Body,
                "time" => Part::Time,
                _ => match name.strip_prefix("header:").map(str::trim).filter(|name| !name.is_empty()) {
                    Some(header) => Part::Header(header),
                    None => {
                        return Err(format!(
                            "`signed` has `{{{name}}}` in it, and what can stand there is `{{body}}`, `{{time}}` or \
                             `{{header:<name>}}`"
                        ));
                    }
                },
            });
            rest = &rest[open + close + 1..];
        }
        if !rest.is_empty() {
            parts.push(Part::Said(rest));
        }
        Ok(parts)
    }

    /// Why this is not a description of a caller, if it is not.
    pub fn refused(&self) -> Option<String> {
        if !is_topic(&self.topic) {
            return Some(format!("`{}` is not a topic; use lowercase words joined by dots", self.topic));
        }
        if RESERVED_TOPICS.contains(&self.topic.split('.').next().unwrap_or_default()) {
            return Some(format!(
                "`{}` is in a namespace only the system may publish under; a caller says a topic of your \
                 blueprint's own",
                self.topic
            ));
        }
        if self.header.trim().is_empty() || self.header.contains(char::is_whitespace) {
            return Some("`header` is the name of the header the caller's signature is sent in".to_string());
        }
        if self.encoding == Written::Text {
            return Some("a signature is written as `hex` or as `base64`".to_string());
        }
        let parts = match self.parts() {
            Ok(parts) => parts,
            Err(why) => return Some(why),
        };
        let has = |wanted: fn(&Part<'_>) -> bool| parts.iter().any(wanted);
        if !has(|part| matches!(part, Part::Body)) {
            return Some(
                "`signed` has no `{body}` in it: a signature that does not cover what was sent would let anything \
                 be sent under it"
                    .to_string(),
            );
        }
        match (self.timed(), has(|part| matches!(part, Part::Time))) {
            (true, false) => {
                return Some(
                    "the caller says when it signed, and `signed` has no `{time}` in it: a time that was not \
                     signed can be written by anyone, and proves nothing"
                        .to_string(),
                );
            }
            (false, true) => {
                return Some(
                    "`signed` has `{time}` in it, and nothing says where the time is: give `time_header`, or \
                     `time_prefix` if it is in the header the signature is in"
                        .to_string(),
                );
            }
            _ => {}
        }
        self.within_ms().err()
    }

    /// Whether `body`, sent with the headers `headers` finds by name (each
    /// as text, or nothing where there is none that is text), was signed
    /// by a holder of `secret`; and if it was not, a sentence for whoever
    /// is setting the caller up. Never says anything of the secret.
    pub fn proves<'h>(
        &self,
        secret: &str,
        headers: impl Fn(&str) -> Option<&'h str>,
        body: &[u8],
        now: i64,
    ) -> Result<Proved, String> {
        let header = |name: &str| -> Result<&'h str, String> {
            headers(name).ok_or_else(|| format!("this call has no `{name}` header"))
        };
        let offered = after(header(&self.header)?, &self.prefix);
        if offered.is_empty() {
            return Err(format!(
                "the `{}` header has no signature after `{}` in it",
                self.header, self.prefix
            ));
        }
        let (time, at) = match self.timed() {
            false => ("", None),
            true => {
                let name = self.time_header();
                let said = after(header(name)?, &self.time_prefix).first().copied().unwrap_or_default();
                let number = said.parse::<i64>().ok().filter(|number| *number > 0).ok_or_else(|| {
                    format!("the `{name}` header does not say when this was signed, as a number")
                })?;
                // Seconds since 1970, as nearly every service says it, or
                // milliseconds: a number this large in seconds is thousands
                // of years off.
                let at = if number < 100_000_000_000 { number.saturating_mul(1000) } else { number };
                if (now - at).abs() > self.within_ms()? {
                    return Err(format!(
                        "this call says it was signed more than {} from now, and one that old is not taken",
                        self.within
                    ));
                }
                (said, Some(at))
            }
        };
        let key = match self.key {
            Written::Text => secret.as_bytes().to_vec(),
            other => read(secret.trim(), other).ok_or_else(|| {
                format!("the secret `{}` is not written the way this caller says", self.secret)
            })?,
        };
        let mut signing = Hmac::<Sha256>::new_from_slice(&key)
            .map_err(|_| format!("the secret `{}` cannot be signed with", self.secret))?;
        for part in self.parts()? {
            match part {
                Part::Said(text) => signing.update(text.as_bytes()),
                Part::Body => signing.update(body),
                Part::Time => signing.update(time.as_bytes()),
                Part::Header(name) => signing.update(header(name)?.as_bytes()),
            }
        }
        for signature in offered.into_iter().take(OFFERED) {
            // Compared in the same time whatever it is, which is the
            // crate's doing.
            // Remembered as the bytes it stands for, written one way: the
            // same signature in other letters is the same call sent again.
            let sent = read(signature, self.encoding);
            if let Some(sent) = sent.filter(|sent| signing.clone().verify_slice(sent).is_ok()) {
                let signature = sent.iter().map(|byte| format!("{byte:02x}")).collect();
                return Ok(Proved { signature, at });
            }
        }
        Err("the signature does not match what was sent: it was signed with another secret, or what arrived is not \
             what was signed"
            .to_string())
    }
}

/// What follows `prefix` in `value` wherever a part of it begins (at the
/// start, or after a comma or a space), up to the next comma or space.
/// With no prefix, every part.
fn after<'a>(value: &'a str, prefix: &str) -> Vec<&'a str> {
    let ends = |c: char| c == ',' || c == ' ';
    if prefix.is_empty() {
        return value.split(ends).filter(|part| !part.is_empty()).collect();
    }
    let mut found = Vec::new();
    for (at, _) in value.char_indices() {
        let begins = at == 0 || value[..at].ends_with(ends);
        if let (true, Some(rest)) = (begins, value[at..].strip_prefix(prefix)) {
            let part = rest.split(ends).next().unwrap_or_default();
            if !part.is_empty() {
                found.push(part);
            }
        }
    }
    found
}

/// The bytes `text` stands for.
fn read(text: &str, written: Written) -> Option<Vec<u8>> {
    match written {
        Written::Text => Some(text.as_bytes().to_vec()),
        Written::Base64 => STANDARD.decode(text).ok(),
        Written::Hex => {
            let digits = text.as_bytes();
            if !digits.len().is_multiple_of(2) {
                return None;
            }
            digits
                .chunks(2)
                .map(|pair| {
                    let digit = |byte: u8| (byte as char).to_digit(16).map(|digit| digit as u8);
                    Some(digit(pair[0])? << 4 | digit(pair[1])?)
                })
                .collect()
        }
    }
}

/// The calls taken lately, by the signature each proved itself with.
///
/// A call that says when it was signed is good until it is `within` old,
/// and until then anyone who saw it could send it again. So one that was
/// taken is remembered that long, and sent again it is answered as it was
/// the first time and said no second time. This is kept in memory: a
/// daemon that restarts forgets it, and for what is left of `within` a call
/// it took before could be taken once more.
#[derive(Default)]
pub struct Seen {
    callers: HashMap<String, VecDeque<Taken>>,
}

struct Taken {
    signature: String,
    /// When it stops being good, on the journal's clock.
    until: i64,
    seq: u64,
}

impl Seen {
    /// The event a call with this signature was said as, if it was taken
    /// and is still good.
    pub fn before(&mut self, caller: &str, signature: &str, now: i64) -> Option<u64> {
        let taken = self.callers.get_mut(caller)?;
        while taken.front().is_some_and(|first| first.until < now) {
            taken.pop_front();
        }
        taken.iter().find(|one| one.signature == signature).map(|one| one.seq)
    }

    pub fn took(&mut self, caller: &str, signature: String, until: i64, seq: u64) {
        let taken = self.callers.entry(caller.to_string()).or_default();
        if taken.len() >= REMEMBERED {
            taken.pop_front();
        }
        taken.push_back(Taken { signature, until, seq });
    }

    pub fn forget(&mut self, caller: &str) {
        self.callers.remove(caller);
    }
}
