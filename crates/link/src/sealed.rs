//! What two installations say to each other, sealed: nobody between the two
//! can read it or change it, and each knows it is the other. It was written
//! for a device and its hub, and the words below speak of those two; it
//! serves any two where one holds the other's public key beforehand.
//!
//! The handshake and the ciphers are Noise's, through the `snow` crate:
//! `Noise_IK_25519_ChaChaPoly_BLAKE2s`. Nothing here is a cipher, a way of
//! making keys or a way of proving one of its own. A device (the initiator)
//! must hold its hub's public key before it says a word, and is refused by
//! the mathematics if what answers does not hold the private half; the hub
//! learns the device's public key from the first message, where only the
//! hub can read it, and looks the device up by it.
//!
//! What is ours is how the sealed messages are laid out as **frames**, and
//! the rules for taking one. A frame is one thing said (a report, a
//! command), which may be longer than a Noise message can be, so it is one
//! or more **records** end to end:
//!
//! ```text
//! record = number (8 bytes, big-endian)
//!          length (2 bytes, big-endian)
//!          a Noise transport message of that length, sealed under that number
//! ```
//!
//! What is sealed in every record begins with the number its frame began at
//! (8 bytes), so that a record is known for the first of its frame or for
//! one that continues it by what was sealed, and not by what follows. After
//! that the first record says how many records the frame has (2 bytes) and
//! what kind of thing it is (1 byte); the rest of it, and all of the
//! others, is the thing said. On taking a frame:
//!
//! - every record must open under the session's key for that direction
//!   (Noise's authentication: a byte changed anywhere, or a frame of another
//!   session, fails here);
//! - every record must say it is of the frame that begins at the first
//!   one's number, the records' numbers must follow one another, and there
//!   must be as many as the first says, so none can be dropped, added,
//!   moved or taken for the start of a frame of its own;
//! - the kind must be the one asked for where it arrived, so a report
//!   cannot be handed in where a link is opened;
//! - and its numbers must be ones not taken before ([`Session::open_next`]),
//!   or it must say which of our own frames it answers ([`Session::open_any`]
//!   and the caller), so that nothing said once can be said again.
//!
//! **A number is never used twice.** Each side seals under a counter of its
//! own for the session that only goes forward, taken in one step for all the
//! records of a frame, and the two directions have different keys. A session
//! is one handshake's: when a link is made again there is a new handshake,
//! new keys, and the counters start again under them.

use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use snow::{Builder, HandshakeState, StatelessTransportState};

use crate::own::{KEY, Keys};

const PATTERN: &str = "Noise_IK_25519_ChaChaPoly_BLAKE2s";
/// Said by both sides before anything else, and part of what the handshake
/// proves: a message made for some other use of these keys is not one here.
/// This is what a device and its hub say; any other pair that shakes hands
/// with these keys says something else ([`offer_for`], [`hear_for`]).
pub const DEVICE_AND_HUB: &[u8] = b"a device and its hub, version 1";

/// The longest a Noise message may be, and what it adds to its contents.
const MESSAGE: usize = 65535;
const TAG: usize = 16;
/// What is sealed in every record begins with: the number its frame began at.
const OF: usize = 8;
/// The most of a frame that one record carries.
const PART: usize = MESSAGE - TAG - OF;
/// What a record has before its sealed message: its number and its length.
const HEAD: usize = 8 + 2;
/// What the first record's contents begin with: how many, and what kind.
const FIRST: usize = 2 + 1;
/// The longest thing that can be said in one frame.
pub const MAX_FRAME: usize = 1_900_000;
/// The longest a frame is once sealed: what whoever reads one from a link
/// reads no more than.
pub const MAX_SEALED: usize = MAX_FRAME + FIRST + ((MAX_FRAME + FIRST) / PART + 1) * (HEAD + OF + TAG);
/// No counter gets near this; one that did would seal nothing more.
const LAST_NUMBER: u64 = u64::MAX / 2;

/// What kind of thing a frame is. Each is taken in one place only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Device to hub: open the link.
    Open = 1,
    /// Device to hub: a report.
    Report = 2,
    /// Hub to device: a command, down the link.
    Command = 3,
    /// Hub to device: the answer to one report.
    Ack = 4,
}

/// Why something sealed was not taken. Said in words where a person is
/// told; nothing of what was sealed is in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsealed(pub String);

impl fmt::Display for Unsealed {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

fn unsealed(why: impl Into<String>) -> Unsealed {
    Unsealed(why.into())
}

fn noise(what: &str, err: snow::Error) -> Unsealed {
    unsealed(format!("{what}: {err:?}"))
}

fn builder<'a>(own: &'a Keys, purpose: &'a [u8]) -> Result<Builder<'a>, Unsealed> {
    let params = PATTERN.parse().map_err(|err| noise("this build does not have the handshake", err))?;
    Builder::new(params)
        .local_private_key(own.private())
        .and_then(|builder| builder.prologue(purpose))
        .map_err(|err| noise("the handshake could not be begun", err))
}

/// A device's first message to its hub, and what it needs to take the answer.
pub struct Offer {
    state: HandshakeState,
    pub message: Vec<u8>,
}

/// Begins a handshake with the hub whose public key is `hub`. `payload` is
/// sealed in the first message, where only that hub can read it.
pub fn offer(own: &Keys, hub: &[u8; KEY], payload: &[u8]) -> Result<Offer, Unsealed> {
    offer_for(DEVICE_AND_HUB, own, hub, payload)
}

/// The same between any two: `purpose` is what both say before anything
/// else, so that a first message made for one use of these keys is not a
/// first message for another.
pub fn offer_for(purpose: &[u8], own: &Keys, hub: &[u8; KEY], payload: &[u8]) -> Result<Offer, Unsealed> {
    let mut state = builder(own, purpose)?
        .remote_public_key(hub)
        .and_then(Builder::build_initiator)
        .map_err(|err| noise("the handshake could not be begun", err))?;
    let mut message = vec![0u8; MESSAGE];
    let length = state.write_message(payload, &mut message).map_err(|err| noise("the first message", err))?;
    message.truncate(length);
    Ok(Offer { state, message })
}

impl Offer {
    /// Takes the hub's answer. It opens only if whoever answered holds the
    /// private half of the key this device expected; then both sides have
    /// the session, and what the hub said in its answer.
    pub fn accept(mut self, answer: &[u8]) -> Result<(Session, Vec<u8>), Unsealed> {
        let mut payload = vec![0u8; MESSAGE];
        let length = self
            .state
            .read_message(answer, &mut payload)
            .map_err(|err| noise("the answer did not open", err))?;
        payload.truncate(length);
        let transport = self
            .state
            .into_stateless_transport_mode()
            .map_err(|err| noise("the handshake did not finish", err))?;
        Ok((Session::new(transport), payload))
    }
}

/// The most that can be said in a handshake's first message, sealed where
/// only the other side can read it: what a Noise message holds, less what
/// the handshake itself puts there.
pub const MAX_FIRST: usize = MESSAGE - 32 - (32 + TAG) - TAG;
/// The most that can be said in the answer to it.
pub const MAX_ANSWER: usize = MESSAGE - 32 - TAG;

/// A first message a hub has read: who sent it, by the key they proved,
/// and what they said in it.
pub struct Heard {
    state: HandshakeState,
    pub from: [u8; KEY],
    pub payload: Vec<u8>,
}

/// Reads a device's first message. It opens only if it was made for this
/// installation's key.
pub fn hear(own: &Keys, message: &[u8]) -> Result<Heard, Unsealed> {
    hear_for(DEVICE_AND_HUB, own, message)
}

/// The same between any two, as [`offer_for`].
pub fn hear_for(purpose: &[u8], own: &Keys, message: &[u8]) -> Result<Heard, Unsealed> {
    let mut state = builder(own, purpose)?
        .build_responder()
        .map_err(|err| noise("the handshake could not be begun", err))?;
    let mut payload = vec![0u8; MESSAGE];
    let length = state.read_message(message, &mut payload).map_err(|err| noise("the first message", err))?;
    payload.truncate(length);
    let from: [u8; KEY] = state
        .get_remote_static()
        .and_then(|key| key.try_into().ok())
        .ok_or_else(|| unsealed("the first message proved no key"))?;
    Ok(Heard { state, from, payload })
}

impl Heard {
    /// Answers, with `payload` sealed for the device that sent the first
    /// message, and gives the session the two now share.
    pub fn answer(mut self, payload: &[u8]) -> Result<(Session, Vec<u8>), Unsealed> {
        let mut message = vec![0u8; MESSAGE];
        let length =
            self.state.write_message(payload, &mut message).map_err(|err| noise("the answer", err))?;
        message.truncate(length);
        let transport = self
            .state
            .into_stateless_transport_mode()
            .map_err(|err| noise("the handshake did not finish", err))?;
        Ok((Session::new(transport), message))
    }
}

/// One handshake's keys, and where each side has got to under them.
pub struct Session {
    transport: StatelessTransportState,
    /// The number the next record this side seals will have.
    next: AtomicU64,
    /// The number after the last one taken in order from the other side.
    expect: Mutex<u64>,
}

impl Session {
    fn new(transport: StatelessTransportState) -> Session {
        Session { transport, next: AtomicU64::new(0), expect: Mutex::new(0) }
    }

    /// Seals `content` as one frame of `kind`. Gives the number its first
    /// record has, which is what an answer to it names.
    pub fn seal(&self, kind: Kind, content: &[u8]) -> Result<(u64, Vec<u8>), Unsealed> {
        if content.len() > MAX_FRAME {
            return Err(unsealed(format!(
                "what was to be said is {} bytes, and the most that is said at once over a link is {MAX_FRAME}",
                content.len()
            )));
        }
        let count = (FIRST + content.len()).div_ceil(PART).max(1);
        // All of a frame's numbers are taken in one step, so that no two
        // frames sealed at once can come by the same one.
        let first = self.next.fetch_add(count as u64, Ordering::SeqCst);
        if first > LAST_NUMBER {
            return Err(unsealed("this session has said all it may; the link is made again"));
        }
        let mut plain = Vec::with_capacity(FIRST + content.len());
        plain.extend_from_slice(&(count as u16).to_be_bytes());
        plain.push(kind as u8);
        plain.extend_from_slice(content);
        let mut sealed = Vec::with_capacity(plain.len() + count * (HEAD + OF + TAG));
        let mut message = vec![0u8; MESSAGE];
        let mut record = Vec::with_capacity(OF + PART);
        for (index, part) in plain.chunks(PART).enumerate() {
            let number = first + index as u64;
            record.clear();
            record.extend_from_slice(&first.to_be_bytes());
            record.extend_from_slice(part);
            let length = self
                .transport
                .write_message(number, &record, &mut message)
                .map_err(|err| noise("a record could not be sealed", err))?;
            sealed.extend_from_slice(&number.to_be_bytes());
            sealed.extend_from_slice(&(length as u16).to_be_bytes());
            sealed.extend_from_slice(&message[..length]);
        }
        Ok((first, sealed))
    }

    /// Opens a frame of `kind` whatever its numbers. Gives the number of its
    /// first record and the number after its last, and what was said.
    ///
    /// By itself this does not stop a frame being taken twice. Whoever calls
    /// it must: see [`Session::open_next`], or hold what was said to a
    /// number of their own that it names.
    pub fn open_any(&self, kind: Kind, sealed: &[u8]) -> Result<(u64, u64, Vec<u8>), Unsealed> {
        let mut rest = sealed;
        let mut plain = Vec::new();
        let mut message = vec![0u8; MESSAGE];
        let mut first = None;
        let mut expected = 0u64;
        let mut records = 0usize;
        while !rest.is_empty() {
            if rest.len() < HEAD {
                return Err(unsealed("a record was cut short"));
            }
            let number = u64::from_be_bytes(rest[..8].try_into().expect("eight bytes"));
            let length = u16::from_be_bytes(rest[8..HEAD].try_into().expect("two bytes")) as usize;
            if rest.len() < HEAD + length {
                return Err(unsealed("a record was cut short"));
            }
            match first {
                None => first = Some(number),
                Some(_) if number != expected => {
                    return Err(unsealed("the records of a frame do not follow one another"));
                }
                Some(_) => {}
            }
            let read = self
                .transport
                .read_message(number, &rest[HEAD..HEAD + length], &mut message)
                .map_err(|_| {
                    unsealed("a record did not open: it was changed on the way, or is not of this session")
                })?;
            // Sealed with it: which frame it is a record of. One that
            // continues a frame says the frame's first number and not its
            // own, so it cannot be handed in as a frame by itself.
            let of = message[..read].first_chunk::<OF>().map(|of| u64::from_be_bytes(*of));
            if of != first {
                return Err(unsealed("a record is not of the frame it was handed in with"));
            }
            plain.extend_from_slice(&message[OF..read]);
            if plain.len() > MAX_FRAME + FIRST {
                return Err(unsealed("more was said at once than a link takes"));
            }
            expected = number.checked_add(1).ok_or_else(|| unsealed("a record's number is past the last"))?;
            records += 1;
            rest = &rest[HEAD + length..];
        }
        let first = first.ok_or_else(|| unsealed("nothing was sent where a frame was wanted"))?;
        if plain.len() < FIRST {
            return Err(unsealed("a frame did not say what it is"));
        }
        let count = u16::from_be_bytes([plain[0], plain[1]]) as usize;
        if count != records {
            return Err(unsealed("a frame does not have as many records as it says"));
        }
        if plain[2] != kind as u8 {
            return Err(unsealed("a frame was handed in where another kind is taken"));
        }
        Ok((first, expected, plain.split_off(FIRST)))
    }

    /// Opens a frame of `kind` that must come after every frame opened this
    /// way before: one sent again, or one that was overtaken, is refused.
    /// Gives the number its first record has, and what was said.
    pub fn open_next(&self, kind: Kind, sealed: &[u8]) -> Result<(u64, Vec<u8>), Unsealed> {
        // Held across the opening, so that of two that arrive at once one is
        // wholly before the other.
        let mut expect = self.expect.lock().expect("session lock");
        let stale = || unsealed("that was said before, or was overtaken by something said after it");
        // Refused by the number it begins with before any of it is opened:
        // what was said before costs nothing to turn away again.
        if sealed.first_chunk::<8>().is_some_and(|number| u64::from_be_bytes(*number) < *expect) {
            return Err(stale());
        }
        let (first, after, content) = self.open_any(kind, sealed)?;
        if first < *expect {
            return Err(stale());
        }
        *expect = after;
        Ok((first, content))
    }
}
