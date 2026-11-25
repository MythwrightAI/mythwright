//! The hub's door: who has been invited, who is in, and the session each
//! device that is linked proved its way into.
//!
//! A device is a member by a record the hub keeps: its name, the public key
//! it proved when it was let in, and what it may do here (its scopes). That
//! record is its credential: there is nothing a device can present but a
//! handshake, and the handshake proves a key or it does not ([`crate::sealed`]).
//!
//! A device is let in by an **invitation**: one string, made for one name,
//! that works once and for a while ([`Invitation`]). The hub keeps a hash of
//! the invitation's secret and never the secret. The device hands the secret
//! over inside its first message, where only this hub can read it, and the
//! key it proved in that same message is the key its record is given.
//!
//! Records and invitations are in the journal's `kv`, under
//! [`DEVICES`] and [`INVITATIONS`], and are left out of a copy of a factory
//! (`crate::copy`): a copy is another installation and has no members.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use journal::Journal;
use kernel::{Error, Result, Source};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::own::{KEY, Keys, fingerprint};
use crate::sealed::{self, Session};

/// Where the hub keeps each member: its name, and under it its key and scopes.
pub(crate) const DEVICES: &str = "gateway.devices";
/// Where the hub keeps each invitation, by the hash of its secret.
pub(crate) const INVITATIONS: &str = "gateway.invitations";

/// What an invitation begins with, so that one is known for what it is.
pub const INVITATION: &str = "inv_";
/// How long an invitation works unless said, and the longest it may.
pub const USUAL: Duration = Duration::from_secs(24 * 3600);
pub const LONGEST: Duration = Duration::from_secs(14 * 24 * 3600);

/// How long a session's name is: random, and no secret.
pub const SESSION: usize = 16;

/// Handshakes refused from one place before it is not answered for a while,
/// how long a while, and how many places are remembered.
const TRIES: u32 = 5;
const WINDOW: Duration = Duration::from_secs(15 * 60);
const PLACES: usize = 1024;
/// How often a refusal is said when there are more places than are
/// remembered.
const UNPLACED: Duration = Duration::from_secs(60);

pub fn encode(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(text.trim()).ok()
}

fn key_of(text: &str) -> Option<[u8; KEY]> {
    decode(text)?.try_into().ok()
}

/// What an invitation says, as the device that takes it reads it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Invitation {
    /// What the hub calls itself.
    pub hub: String,
    /// The hub's public key: what must be proved by whatever answers.
    pub key: String,
    /// Where the hub can be reached, each tried in turn.
    pub at: Vec<String>,
    /// The name the device is let in under.
    pub device: String,
    /// What works once.
    pub secret: String,
    /// When it stops working, in milliseconds since 1970.
    pub expires: i64,
}

/// Printed without what works once.
impl std::fmt::Debug for Invitation {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Invitation")
            .field("hub", &self.hub)
            .field("device", &self.device)
            .field("at", &self.at)
            .field("expires", &self.expires)
            .finish_non_exhaustive()
    }
}

/// Whether `place` is somewhere a link can be carried to: plain HTTP, and
/// nothing else. What crosses is sealed by the two installations and needs
/// nothing of what carries it; a place written any other way is not one
/// this goes to.
pub fn is_place(place: &str) -> bool {
    place.strip_prefix("http://").is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

impl Invitation {
    pub fn written(&self) -> String {
        format!("{INVITATION}{}", encode(json!(self).to_string().as_bytes()))
    }

    pub fn read(text: &str) -> Result<Invitation> {
        let not_one = || {
            Error::invalid(format!(
                "that is not an invitation. One begins `{INVITATION}` and is made on the hub with `{} device invite <name>`; give all of it, as it was printed",
                brand::CLI
            ))
        };
        let body = text.trim().strip_prefix(INVITATION).ok_or_else(not_one)?;
        let invitation: Invitation =
            decode(body).and_then(|bytes| serde_json::from_slice(&bytes).ok()).ok_or_else(not_one)?;
        if key_of(&invitation.key).is_none() || invitation.at.is_empty() {
            return Err(not_one());
        }
        if let Some(wrong) = invitation.at.iter().find(|place| !is_place(place)) {
            return Err(Error::invalid(format!(
                "that invitation says its hub is at `{}`, which is not somewhere a link goes: an address looks like `http://host:port`. \
                 Ask whoever runs the hub for another, made with `--at http://host:port`",
                wrong.chars().take(120).collect::<String>()
            )));
        }
        Ok(invitation)
    }

    pub fn hub_key(&self) -> [u8; KEY] {
        key_of(&self.key).unwrap_or([0; KEY])
    }
}

fn hashed(secret: &str) -> String {
    Sha256::digest(secret.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A member, as the hub keeps it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    /// The public key it proved when it was let in.
    pub key: String,
    /// What it may do here.
    pub scopes: Vec<String>,
    pub joined: i64,
    /// The highest number a first message of its has carried. One that is
    /// not above it was said before, by the device or by whoever saw it
    /// cross, and is not answered with a session.
    #[serde(default)]
    pub heard: u64,
}

impl Member {
    pub fn id(&self) -> String {
        decode(&self.key).map(|key| fingerprint(&key)).unwrap_or_default()
    }
}

/// An invitation, as the hub keeps it: never its secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Invited {
    device: String,
    expires: i64,
    /// When it was used, once it has been. Kept until it would have stopped
    /// working anyway, so that using it again is told apart from one that
    /// was never made.
    #[serde(default)]
    spent: Option<i64>,
    /// The key it was used by. That device is let in by it again, for the
    /// case that the hub's first answer never reached it.
    #[serde(default)]
    spent_by: Option<String>,
}

/// What a device says in its first message.
#[derive(Default, Serialize, Deserialize)]
pub struct Hello {
    /// The secret of the invitation it is redeeming, when it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invitation: Option<String>,
    /// A number that only goes up for this device ([`crate::own::Said`]).
    #[serde(default)]
    pub number: u64,
}

/// What the hub says in its answer, sealed for the device.
#[derive(Default, Serialize, Deserialize)]
pub struct Welcome {
    pub ok: bool,
    #[serde(default)]
    pub hub: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    /// Why not, when not.
    #[serde(default)]
    pub error: Option<String>,
    /// When the first message's number was not above what the hub has
    /// taken from this device: what it has taken, for the device to go on
    /// from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub above: Option<u64>,
    /// That the key the device proved is none this hub knows, and it
    /// brought no invitation: one that was in the fleet has been taken out
    /// of it. Said sealed, so that only the hub can say it; what the
    /// device was doing for the hub it stops.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unknown: bool,
}

/// The hub's answer to a first message, and whose links it ended.
pub struct Answered {
    /// What to send back: a session's name, then the hub's own message.
    pub answer: Vec<u8>,
    /// Names the device that was let in has left: it came in under another.
    pub left: Vec<String>,
}

/// A device that proved its key, and the session it did it in.
pub struct Linked {
    pub device: String,
    pub scopes: Vec<String>,
    pub session: Session,
    /// Which handshake of this hub's it came of, in order.
    made: u64,
    /// When that handshake was answered.
    at: i64,
    /// Raised when the device is no longer one, or the hub stops: nothing
    /// more is taken or said in it, by whoever still holds it.
    ended: AtomicBool,
    /// Whether the device has said anything in it yet. A first message of
    /// the handshake can be sent again by anyone who saw it cross, and gets
    /// an answer and a session; but only the device that made it has the
    /// keys to say anything in that session. So a session counts for
    /// nothing until something sealed in it has opened ([`Door::proved`]).
    spoken: AtomicBool,
}

impl Linked {
    /// Whether nothing more is to be taken or said in this session.
    pub fn ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }

    /// Whether nothing was said in it within the time a device is given.
    fn stale(&self, now: i64) -> bool {
        !self.spoken.load(Ordering::SeqCst) && now - self.at > UNSPOKEN_FOR.as_millis() as i64
    }
}

/// How many sessions a device may have that nothing has been said in yet,
/// and how long one is kept waiting for its device to speak. A device
/// speaks at once; an answer nobody has used in this long was not the
/// device's to use, or was held back by whatever carries it.
const UNSPOKEN: usize = 4;
const UNSPOKEN_FOR: Duration = Duration::from_secs(60);

/// Why a first message got no sealed answer.
#[derive(Debug)]
pub enum Shut {
    /// It was not made for this installation's key: there is nobody to seal
    /// an answer for.
    Unread(String),
    /// Too many refused from there: not now.
    Later {
        wait: Duration,
        message: String,
    },
    Failed(Error),
}

impl From<Error> for Shut {
    fn from(err: Error) -> Shut {
        Shut::Failed(err)
    }
}

#[derive(Default)]
struct Tries {
    /// How many were refused from a place, and when the first of them was.
    places: HashMap<IpAddr, (u32, i64)>,
    /// When it was last said that something from a place was refused.
    said: HashMap<IpAddr, i64>,
    /// When a refusal was last said while there was no room to remember
    /// another place.
    said_unplaced: i64,
}

pub struct Door {
    journal: Journal,
    keys: Keys,
    sessions: Mutex<HashMap<[u8; SESSION], Arc<Linked>>>,
    /// How many handshakes have been answered with a session.
    made: AtomicU64,
    tries: Mutex<Tries>,
    /// One device is let in, looked up, given a session or taken out at a
    /// time: an invitation that two present at once is spent by one of
    /// them, and a device that is taken out as it shakes hands is left
    /// with no session.
    one_at_a_time: tokio::sync::Mutex<()>,
}

impl Door {
    pub fn new(journal: Journal, keys: Keys) -> Door {
        Door {
            journal,
            keys,
            sessions: Mutex::default(),
            made: AtomicU64::new(0),
            tries: Mutex::default(),
            one_at_a_time: tokio::sync::Mutex::new(()),
        }
    }

    /// The hub's own key's short name.
    pub fn id(&self) -> String {
        fingerprint(&self.keys.public)
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    // ---- who is in, and who is invited ---------------------------------------

    pub async fn members(&self) -> Result<Vec<(String, Member)>> {
        let kept = self.journal.kv_list(DEVICES).await?;
        Ok(kept
            .into_iter()
            .filter_map(|(name, member)| Some((name, serde_json::from_value(member).ok()?)))
            .collect())
    }

    /// The names invited and not yet in, each with when its invitation stops
    /// working (the latest, where a name has several).
    pub async fn invited(&self) -> Result<Vec<(String, i64)>> {
        let now = self.journal.now_ms();
        let mut waiting: HashMap<String, i64> = HashMap::new();
        for (_, kept) in self.journal.kv_list(INVITATIONS).await? {
            let Ok(invited) = serde_json::from_value::<Invited>(kept) else { continue };
            if invited.spent.is_none() && invited.expires > now {
                let until = waiting.entry(invited.device).or_insert(invited.expires);
                *until = (*until).max(invited.expires);
            }
        }
        let mut waiting: Vec<(String, i64)> = waiting.into_iter().collect();
        waiting.sort();
        Ok(waiting)
    }

    /// Makes an invitation for `device`, kept by the hash of its secret.
    pub async fn invite(&self, device: &str, expires: i64, at: Vec<String>) -> Result<Invitation> {
        let now = self.journal.now_ms();
        // What stopped working is of no more use, spent or not.
        for (hash, kept) in self.journal.kv_list(INVITATIONS).await? {
            let gone = serde_json::from_value::<Invited>(kept).map_or(true, |invited| invited.expires <= now);
            if gone {
                self.journal.kv_set(INVITATIONS, &hash, &serde_json::Value::Null).await?;
            }
        }
        let secret = kernel::id::secret();
        let kept = Invited { device: device.to_string(), expires, spent: None, spent_by: None };
        self.journal.kv_set(INVITATIONS, &hashed(&secret), &json!(kept)).await?;
        Ok(Invitation {
            hub: self.journal.device().to_string(),
            key: encode(&self.keys.public),
            at,
            device: device.to_string(),
            secret,
            expires,
        })
    }

    /// Takes a device out, and what was waiting to let it in. Says whether
    /// there was anything of that name.
    pub async fn remove(&self, device: &str) -> Result<bool> {
        let _one = self.one_at_a_time.lock().await;
        let mut found = self.journal.kv_get(DEVICES, device).await?.is_some();
        self.journal.kv_set(DEVICES, device, &serde_json::Value::Null).await?;
        for (hash, kept) in self.journal.kv_list(INVITATIONS).await? {
            if serde_json::from_value::<Invited>(kept).is_ok_and(|invited| invited.device == device) {
                found = true;
                self.journal.kv_set(INVITATIONS, &hash, &serde_json::Value::Null).await?;
            }
        }
        self.forget(device);
        Ok(found)
    }

    // ---- sessions -------------------------------------------------------------

    /// The session a sealed body begins by naming, and the rest of the body.
    pub fn session<'a>(&self, body: &'a [u8]) -> Result<(Arc<Linked>, &'a [u8])> {
        let unknown = || {
            Error::unauthenticated(
                "this hub holds no such session: it was ended, or the hub has been started again since. Shake hands again",
            )
        };
        if body.len() < SESSION {
            return Err(unknown());
        }
        let (name, rest) = body.split_at(SESSION);
        let name: [u8; SESSION] = name.try_into().expect("a session's name");
        let now = self.journal.now_ms();
        let mut sessions = self.sessions.lock().expect("door lock");
        let linked = sessions.get(&name).cloned().ok_or_else(unknown)?;
        if linked.stale(now) {
            // Answered, and never used in the time a device takes to
            // speak: it is over, whoever comes with it now.
            linked.ended.store(true, Ordering::SeqCst);
            sessions.remove(&name);
        }
        if linked.ended() {
            return Err(unknown());
        }
        Ok((linked, rest))
    }

    /// The device has said something in this session, which only the device
    /// that shook hands in it could: it is the device's session now, and
    /// the one it had before is ended. Not if the device was taken out
    /// meanwhile: then the session is nobody's.
    pub fn proved(&self, linked: &Arc<Linked>) -> Result<()> {
        let mut sessions = self.sessions.lock().expect("door lock");
        if linked.ended() {
            return Err(Error::unauthenticated(
                "this hub holds no such session: the device it was for is no longer one of its fleet",
            ));
        }
        if linked.spoken.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        // The device's now: the one it was using is over, and so is every
        // one answered before this that nothing was said in. Such an answer
        // turning up later, from wherever it was held, opens nothing and
        // takes nothing from the device.
        sessions.retain(|_, other| {
            let stays = other.device != linked.device
                || Arc::ptr_eq(other, linked)
                || (!other.spoken.load(Ordering::SeqCst) && other.made > linked.made);
            if !stays {
                other.ended.store(true, Ordering::SeqCst);
            }
            stays
        });
        Ok(())
    }

    /// Ends every session `device` has.
    pub fn forget(&self, device: &str) {
        self.sessions.lock().expect("door lock").retain(|_, linked| {
            if linked.device == device {
                linked.ended.store(true, Ordering::SeqCst);
            }
            linked.device != device
        });
    }

    /// Ends every session, as when the daemon stops.
    pub fn halt(&self) {
        let mut sessions = self.sessions.lock().expect("door lock");
        for linked in sessions.values() {
            linked.ended.store(true, Ordering::SeqCst);
        }
        sessions.clear();
    }

    // ---- the handshake ----------------------------------------------------------

    /// How long `from` is not answered for, if too many were refused from there.
    fn put_off(&self, from: IpAddr, now: i64) -> Option<Duration> {
        let mut tries = self.tries.lock().expect("door lock");
        let window = WINDOW.as_millis() as i64;
        tries.places.retain(|_, (_, first)| now - *first < window);
        let (refused, first) = *tries.places.get(&from)?;
        (refused >= TRIES).then(|| Duration::from_millis((first + window - now).max(1000) as u64))
    }

    /// Counts against `from` something that was refused: what could not be
    /// opened, a key this hub does not know, an invitation that does not
    /// work, a first message that was said before. A device that proves a
    /// key that is a member's, in a message it has not said before, is not
    /// counted and is answered whatever else has come from where it is,
    /// and so is one that brings an invitation this hub holds: many
    /// devices can be behind one address (a proxy in front of the hub, a
    /// tunnel, one router).
    fn refused(&self, from: IpAddr, now: i64) {
        let mut tries = self.tries.lock().expect("door lock");
        if tries.places.len() >= PLACES && !tries.places.contains_key(&from) {
            // Room is made by forgetting whoever has been remembered longest.
            if let Some(oldest) =
                tries.places.iter().min_by_key(|(_, (_, first))| *first).map(|(place, _)| *place)
            {
                tries.places.remove(&oldest);
            }
        }
        tries.places.entry(from).or_insert((0, now)).0 += 1;
    }

    fn shut(&self, wait: Duration) -> Shut {
        Shut::Later {
            wait,
            message: "too much from there was refused; this hub does not answer it again for a while".into(),
        }
    }

    /// Whether a refusal from `from` is to be said now: once for a place
    /// within the time refusals are counted over, so that being refused
    /// again and again writes nothing down again and again.
    fn says_refused(&self, from: IpAddr, now: i64) -> bool {
        let mut tries = self.tries.lock().expect("door lock");
        let window = WINDOW.as_millis() as i64;
        tries.said.retain(|_, at| now - *at < window);
        if tries.said.contains_key(&from) {
            return false;
        }
        if tries.said.len() >= PLACES {
            // No room to remember another place. Not said of every one of
            // them for ever, and not of none: once in a while, so that
            // coming from many places at once does not make refusals
            // unseen.
            if now - tries.said_unplaced < UNPLACED.as_millis() as i64 {
                return false;
            }
            tries.said_unplaced = now;
            return true;
        }
        tries.said.insert(from, now);
        true
    }

    /// Whether `secret` is of an invitation this hub made that can still
    /// let someone in: not used and not over, or used by the device that
    /// proved `proved` (whose first answer may never have reached it).
    async fn holds(&self, secret: &str, proved: &str, now: i64) -> Result<bool> {
        let kept = self.journal.kv_get(INVITATIONS, &hashed(secret)).await?;
        let Some(invited) = kept.and_then(|kept| serde_json::from_value::<Invited>(kept).ok()) else {
            return Ok(false);
        };
        Ok(match invited.spent {
            None => invited.expires > now,
            Some(_) => invited.spent_by.as_deref() == Some(proved),
        })
    }

    /// Why the device that proved `key`, saying `hello`, is not let in; or
    /// its name, its record, and the names it has left by coming in under
    /// this one.
    async fn admit(
        &self,
        key: &[u8; KEY],
        hello: &Hello,
        members: Vec<(String, Member)>,
        now: i64,
    ) -> Result<std::result::Result<(String, Member, Vec<String>), String>> {
        let proved = encode(key);
        let Some(secret) = &hello.invitation else {
            return Ok(match members.into_iter().find(|(_, member)| member.key == proved) {
                Some((name, member)) => {
                    let member = Member { heard: hello.number, ..member };
                    self.journal.kv_set(DEVICES, &name, &json!(member)).await?;
                    Ok((name, member, Vec::new()))
                }
                None => Err(format!(
                    "this hub does not know the key that device proved: it was never let in, or it has been removed. \
                     Ask whoever runs the hub for an invitation (`{0} device invite <name>` there) and join with it (`{0} device join <invitation>`)",
                    brand::CLI
                )),
            });
        };
        let hash = hashed(secret);
        let another = format!("Ask for another: `{} device invite <name>` on the hub", brand::CLI);
        let Some(invited) = self
            .journal
            .kv_get(INVITATIONS, &hash)
            .await?
            .and_then(|kept| serde_json::from_value::<Invited>(kept).ok())
        else {
            return Ok(Err(format!(
                "that invitation is not one this hub made, or it stopped working long ago. {another}"
            )));
        };
        if invited.spent.is_some() {
            // Used, and by this device, which is still the member it made:
            // the answer to its first try was lost on the way, and it is
            // let in as it would have been.
            let again = members
                .iter()
                .find(|(name, member)| *name == invited.device && member.key == proved)
                .filter(|_| invited.spent_by.as_deref() == Some(proved.as_str()));
            if let Some((name, member)) = again {
                let member = Member { heard: hello.number, ..member.clone() };
                self.journal.kv_set(DEVICES, name, &json!(member)).await?;
                return Ok(Ok((name.clone(), member, Vec::new())));
            }
            return Ok(Err(format!(
                "that invitation has been used: an invitation lets one device in, once. {another}"
            )));
        }
        if invited.expires <= now {
            return Ok(Err(format!(
                "that invitation has stopped working: it was good for a while and the while is over. {another}"
            )));
        }
        if members.iter().any(|(name, member)| *name == invited.device && member.key != proved) {
            return Ok(Err(format!(
                "a device called `{0}` is in this fleet already, and it is not this one. Remove it on the hub first (`{1} device remove {0}`), or invite this device under another name",
                invited.device,
                brand::CLI
            )));
        }
        // A device is one member: let in under a new name, it leaves the old.
        let mut left = Vec::new();
        for (name, member) in &members {
            if member.key == proved && *name != invited.device {
                self.journal.kv_set(DEVICES, name, &serde_json::Value::Null).await?;
                self.forget(name);
                left.push(name.clone());
            }
        }
        let member = Member {
            key: proved.clone(),
            scopes: vec!["fleet".to_string()],
            joined: now,
            heard: hello.number,
        };
        self.journal.kv_set(DEVICES, &invited.device, &json!(member)).await?;
        let spent = Invited { spent: Some(now), spent_by: Some(proved), ..invited.clone() };
        self.journal.kv_set(INVITATIONS, &hash, &json!(spent)).await?;
        let _ = self
            .journal
            .emit(
                "device.added",
                Source::actor("system"),
                json!({ "device": invited.device, "id": member.id() }),
            )
            .await;
        Ok(Ok((invited.device, member, left)))
    }

    /// Takes a device's first message and gives the answer to send back: a
    /// session's name, then the hub's own message of the handshake. The
    /// device learns from the sealed answer whether it is in; a session is
    /// kept only when it is.
    ///
    /// The message is read before anything is counted or looked at. What
    /// follows is done for one device at a time, from finding the device to
    /// keeping its session, so that a device taken out meanwhile
    /// ([`Door::remove`]) is left with none.
    pub async fn hello(&self, message: &[u8], from: IpAddr) -> std::result::Result<Answered, Shut> {
        let now = self.journal.now_ms();
        let heard = match sealed::hear(&self.keys, message) {
            Ok(heard) => heard,
            Err(_) => {
                if let Some(wait) = self.put_off(from, now) {
                    return Err(self.shut(wait));
                }
                self.refused(from, now);
                return Err(Shut::Unread(
                    "that was not made for this installation's key. Whoever sent it holds another key for this hub than the one it has: \
                     it joined some other installation at this address, or this one has been made again since"
                        .into(),
                ));
            }
        };
        let hello: Hello = serde_json::from_slice(&heard.payload).unwrap_or_default();
        let proved = encode(&heard.from);
        let _one = self.one_at_a_time.lock().await;
        let members = self.members().await?;
        let known = members.iter().find(|(_, member)| member.key == proved);
        let refuse = |heard: sealed::Heard, welcome: Welcome| {
            let (_, answer) = heard.answer(json!(welcome).to_string().as_bytes()).map_err(|err| {
                Shut::Failed(Error::internal(format!("the handshake could not be answered: {err}")))
            })?;
            Ok::<_, Shut>(Answered {
                answer: [[0u8; SESSION].as_slice(), &answer].concat(),
                left: Vec::new(),
            })
        };
        // Said before: by this device, or by whoever saw it cross and sends
        // it again. Turned away before a session is made, anything is
        // counted or written down, or it is looked into further; the device
        // itself, if it is the device, learns where to go on from.
        if let Some((_, member)) = known
            && hello.number <= member.heard
        {
            // Counted against where it came from, and from a place too
            // much was refused from it is not answered at all: it costs
            // whoever sends it again nothing to send, and this hub a
            // handshake each time.
            if let Some(wait) = self.put_off(from, now) {
                return Err(self.shut(wait));
            }
            self.refused(from, now);
            let welcome = Welcome {
                ok: false,
                error: Some("that first message was said before; a new one is wanted".into()),
                above: Some(member.heard),
                ..Welcome::default()
            };
            return refuse(heard, welcome);
        }
        // A member linking is answered wherever it is, and so is whoever
        // brings an invitation this hub holds: its secret cannot be
        // guessed, so there is nothing to count, and the device it was made
        // for may be behind the same address as whatever was refused.
        // Anything else from a place too much was refused from is not
        // looked at for a while.
        let linking = known.is_some() && hello.invitation.is_none();
        let invited = match &hello.invitation {
            Some(secret) => self.holds(secret, &proved, now).await?,
            None => false,
        };
        if !linking
            && !invited
            && let Some(wait) = self.put_off(from, now)
        {
            return Err(self.shut(wait));
        }
        let (device, member, left) = match self.admit(&heard.from, &hello, members, now).await? {
            Ok(admitted) => admitted,
            Err(why) => {
                self.refused(from, now);
                if self.says_refused(from, now) {
                    let _ = self
                        .journal
                        .emit(
                            "device.refused",
                            Source::actor("system"),
                            json!({ "id": fingerprint(&heard.from), "from": from.to_string(), "why": why }),
                        )
                        .await;
                }
                // With no invitation, the one reason is that its key is
                // none of this fleet's: said so that the device can tell
                // being taken out from any other refusal.
                let unknown = hello.invitation.is_none();
                return refuse(heard, Welcome { ok: false, error: Some(why), unknown, ..Welcome::default() });
            }
        };
        let welcome = Welcome {
            ok: true,
            hub: Some(self.journal.device().to_string()),
            device: Some(device.clone()),
            ..Welcome::default()
        };
        let (session, answer) = heard.answer(json!(welcome).to_string().as_bytes()).map_err(|err| {
            Shut::Failed(Error::internal(format!("the handshake could not be answered: {err}")))
        })?;
        let mut name = [0u8; SESSION];
        getrandom::fill(&mut name).map_err(|err| {
            Shut::Failed(Error::internal(format!("nothing random to name a session with: {err}")))
        })?;
        let made = self.made.fetch_add(1, Ordering::SeqCst);
        let mut sessions = self.sessions.lock().expect("door lock");
        // The session the device is using stays until it has spoken in this
        // one ([`Door::proved`]). Of those nothing has been said in yet,
        // only the latest few are kept.
        let mut unspoken: Vec<u64> = sessions
            .values()
            .filter(|linked| linked.device == device && !linked.spoken.load(Ordering::SeqCst))
            .map(|linked| linked.made)
            .collect();
        unspoken.sort_unstable();
        let keep_from = unspoken.len().saturating_sub(UNSPOKEN - 1);
        let oldest_kept = unspoken.get(keep_from).copied().unwrap_or(made);
        sessions.retain(|_, linked| {
            let stays = !linked.stale(now)
                && (linked.device != device
                    || linked.spoken.load(Ordering::SeqCst)
                    || linked.made >= oldest_kept);
            if !stays {
                linked.ended.store(true, Ordering::SeqCst);
            }
            stays
        });
        let linked = Linked {
            device,
            scopes: member.scopes,
            session,
            made,
            at: now,
            ended: AtomicBool::new(false),
            spoken: AtomicBool::new(false),
        };
        sessions.insert(name, Arc::new(linked));
        Ok(Answered { answer: [name.as_slice(), &answer].concat(), left })
    }
}
