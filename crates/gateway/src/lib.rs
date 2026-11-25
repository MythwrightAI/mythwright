//! Joins devices into one fleet.
//!
//! One device is the **hub**: it runs the blueprints. Others are
//! **members**: each keeps a link open to the hub and does the steps the
//! hub sends it (`device: <name>` on a step). A member is let in by an
//! invitation the hub made for it; that is all a member needs, because it
//! is always the member that calls.
//!
//! Nothing here chooses a network, and nothing here trusts one. The hub's
//! address only has to be reachable from the member. What crosses is
//! sealed between the two ([`sealed`]): each proves a key to the other in a
//! handshake, the member holding the hub to the key its invitation named
//! and the hub finding the member by the key it was let in with, and what
//! is between them can read nothing and change nothing. A member still
//! decides for itself what its hub may do on it.
//!
//! - [`own`]: this installation's secret, and the keys made from it.
//! - [`sealed`]: the handshake, and what is said after it as sealed frames.
//! - [`door`]: the hub's side of letting devices in: invitations, members,
//!   sessions.
//! - [`wire`]: what the two sides say to each other.
//! - [`hub`]: the hub's side; steps waiting on members.
//! - [`member`]: the member's side; the link and its retries.
//! - [`http`]: the member's link, sealed, and carried over HTTP.
//! - [`place`]: choosing which device a piece of work is given to.

mod actions;
pub mod copy;
pub mod door;
pub mod events;
pub mod http;
pub mod hub;
pub mod member;
pub mod own;
pub mod place;
pub mod sealed;
pub mod wire;

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use conductor::{Engine, Registry};
use hosting::Hosting;
use journal::Journal;
use kernel::{Cancel, Error, Principal, PrincipalKind, Result, Source};
use serde::{Deserialize, Serialize};
use serde_json::json;
use studio::{CredentialKind, Identity};

pub use door::{Invitation, Linked, Shut};
pub use http::{Carrier, HttpCarrier, SealedUplink};
pub use hub::{Hub, LinkStream};
pub use member::LinkStatus;
pub use wire::{Ack, Command, DeviceInfo, LinkError, Report, Uplink};

use crate::door::Door;
use crate::member::Member;
use crate::own::{Keys, Own, Said, fingerprint, said_file, said_to};
use crate::sealed::Kind;

#[derive(Clone)]
pub struct Gateway {
    inner: Arc<Inner>,
}

struct Inner {
    journal: Journal,
    identity: Identity,
    hosting: Hosting,
    data_dir: PathBuf,
    /// The keys this installation proves itself with on a link.
    keys: Keys,
    /// The number each first message of a handshake of this device's
    /// carries, for each hub it has shaken hands with since it started, by
    /// the hub's key: one count for one hub, however many links are made.
    said: Mutex<HashMap<[u8; own::KEY], Arc<Said>>>,
    hub: Hub,
    door: Door,
    /// Held weakly: what the engine holds (the gateway's actions, the way
    /// it reaches other devices) holds the gateway, and an engine held
    /// back by them was never given back, a node's whole in memory for
    /// every node a test started.
    engine: OnceLock<conductor::Downgraded>,
    joined: Mutex<Option<Joined>>,
    /// What carries a link to a hub at a given address.
    carry: Carry,
    /// What was picked for which device lately ([`place`]).
    placed: place::Placed,
}

/// Makes what carries a link to the hub at an address.
pub type Carry = Box<dyn Fn(&str) -> Arc<dyn Carrier> + Send + Sync>;

/// This device's membership of another's fleet.
struct Joined {
    url: String,
    /// The short name of the key the hub must prove.
    hub_id: String,
    member: Member,
    stop: Cancel,
}

/// What is kept on disk about the hub this device has joined. There is no
/// secret in it: what this device proves itself with is its own
/// installation's ([`own`]). The file is the owner's alone all the same.
#[derive(Serialize, Deserialize)]
struct Membership {
    url: String,
    /// The hub's public key, as its invitation gave it. Every handshake is
    /// held to it.
    hub_key: String,
    allow: Vec<String>,
}

/// A member, as its hub knows it.
#[derive(Clone, Debug, Serialize)]
pub struct Device {
    pub name: String,
    pub connected: bool,
    /// When the current link opened.
    pub since: Option<i64>,
    pub last_seen: Option<i64>,
    pub info: Option<DeviceInfo>,
    /// The short name of the key it proved when it was let in; none while
    /// it is only invited.
    pub id: Option<String>,
    /// When its invitation stops working, while it has not yet used it.
    pub invited: Option<i64>,
}

/// This device's link to its hub.
#[derive(Clone, Debug, Serialize)]
pub struct HubLink {
    pub url: String,
    /// The short name of the key the hub proves: what `id` is there.
    pub id: String,
    #[serde(flatten)]
    pub status: LinkStatus,
}

/// An invitation that was just made: shown once.
#[derive(Clone, Serialize)]
pub struct Invited {
    pub name: String,
    pub invitation: String,
    pub expires: i64,
}

/// Printed without the invitation, which lets a device in.
impl std::fmt::Debug for Invited {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Invited")
            .field("name", &self.name)
            .field("expires", &self.expires)
            .finish_non_exhaustive()
    }
}

/// The stream of commands for one member, each sealed for it as it is taken.
pub struct SealedStream {
    stream: LinkStream,
    linked: Arc<Linked>,
    hub: Hub,
}

impl SealedStream {
    /// The next command as a line of the link: a sealed frame, in letters.
    /// `None` once the hub has closed the link, or the device is no longer
    /// one of the fleet.
    pub async fn next(&mut self) -> Option<String> {
        loop {
            let command = self.stream.next().await?;
            if self.linked.ended() {
                return None;
            }
            let content = serde_json::to_vec(&command).ok()?;
            if content.len() > sealed::MAX_FRAME {
                // More than is said at once over a link. Sent again it
                // would be too much again, for ever: the step that asked
                // for it fails, saying so, and the link goes on.
                if let Command::Run(run) = &command {
                    self.hub.refuse(
                        &run.id,
                        &format!(
                            "what this step sends to `{}` is {} bytes, and the most that is said at once over a link is {}. \
                             Send less: keep what is large in a file there, or fetch it there",
                            self.linked.device,
                            content.len(),
                            sealed::MAX_FRAME
                        ),
                    );
                }
                continue;
            }
            let (_, frame) = self.linked.session.seal(Kind::Command, &content).ok()?;
            return Some(door::encode(&frame));
        }
    }
}

fn is_device_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 40
        && name.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !name.starts_with('-')
        && name != "local"
}

impl Gateway {
    /// Opens the gateway of the installation whose data is in `data_dir`,
    /// making the installation's secret if this is its first start. Links
    /// to a hub are carried over HTTP.
    pub fn new(journal: Journal, identity: Identity, hosting: Hosting, data_dir: PathBuf) -> Result<Gateway> {
        Self::carried_by(
            journal,
            identity,
            hosting,
            data_dir,
            Box::new(|url| Arc::new(HttpCarrier::new(url))),
        )
    }

    /// The same, with links to a hub carried by whatever `carry` makes for
    /// the hub's address.
    pub fn carried_by(
        journal: Journal,
        identity: Identity,
        hosting: Hosting,
        data_dir: PathBuf,
        carry: Carry,
    ) -> Result<Gateway> {
        let own = |err: link::Error| Error::internal(err.message);
        let keys = Own::open(&data_dir).map_err(own)?.link().map_err(own)?;
        let hub = Hub::new(journal.clone());
        let door = Door::new(journal.clone(), keys.clone());
        Ok(Gateway {
            inner: Arc::new(Inner {
                journal,
                identity,
                hosting,
                data_dir,
                keys,
                said: Mutex::default(),
                hub,
                door,
                engine: OnceLock::new(),
                joined: Mutex::new(None),
                carry,
                placed: Default::default(),
            }),
        })
    }

    pub fn register(&self, registry: &mut Registry) {
        actions::register(registry, self.clone());
        registry.emits(events::events());
    }

    /// Makes this the way the engine reaches other devices.
    pub fn attach(&self, engine: &Engine) {
        let _ = self.inner.engine.set(engine.downgrade());
        let gateway = self.clone();
        engine.set_remote(Arc::new(hub::Dispatch {
            hub: self.inner.hub.clone(),
            known: Arc::new(move || {
                let gateway = gateway.clone();
                Box::pin(async move { gateway.names().await.unwrap_or_default() })
            }),
        }));
    }

    /// The engine it was attached to, while that is there.
    fn engine(&self) -> Option<Engine> {
        self.inner.engine.get().and_then(conductor::Downgraded::upgrade)
    }

    pub fn hub(&self) -> &Hub {
        &self.inner.hub
    }

    /// The short name of this installation's key for links: what another
    /// installation sees it prove.
    pub fn id(&self) -> String {
        fingerprint(&self.inner.keys.public)
    }

    /// This device's sealed way to the hub at `url` whose key is `hub_key`.
    fn uplink_to(&self, url: &str, hub_key: [u8; own::KEY]) -> SealedUplink {
        let journal = self.inner.journal.clone();
        let said = self
            .inner
            .said
            .lock()
            .expect("gateway lock")
            .entry(hub_key)
            .or_insert_with(|| Arc::new(said_to(&self.inner.data_dir, &hub_key)))
            .clone();
        SealedUplink::new(
            (self.inner.carry)(url),
            self.inner.keys.clone(),
            hub_key,
            said,
            Arc::new(move || journal.now_ms()),
        )
    }

    fn membership_file(&self) -> PathBuf {
        self.inner.data_dir.join("gateway").join("hub.json")
    }

    /// Links to the hub this device joined, if it has joined one, and puts
    /// away what is left of the way devices were joined before.
    pub async fn start(&self) -> Result<()> {
        // A device used to be known by a credential it sent with every
        // request, in the clear. None is taken any more: each is ended, and
        // the device is let in again by an invitation.
        for credential in self.inner.identity.credentials().await? {
            if credential.kind == CredentialKind::Device && credential.revoked.is_none() {
                self.inner.identity.revoke_subject(CredentialKind::Device, &credential.subject).await?;
                let _ = self
                    .inner
                    .journal
                    .emit("device.removed", Source::actor("system"), json!({ "device": credential.subject }))
                    .await;
            }
        }
        // There was one count for every hub; there is one for each now.
        let _ = std::fs::remove_file(self.inner.data_dir.join("gateway").join("hub.said"));
        let file = self.membership_file();
        let Ok(saved) = std::fs::read_to_string(&file) else { return Ok(()) };
        let membership = match serde_json::from_str::<Membership>(&saved) {
            Ok(membership) => membership,
            Err(_) => {
                // Joined the old way, or damaged: either way there is no key
                // to hold a hub to. Said once; the file is put aside.
                let hub = serde_json::from_str::<serde_json::Value>(&saved)
                    .ok()
                    .and_then(|kept| kept["url"].as_str().map(str::to_string))
                    .unwrap_or_default();
                let _ = std::fs::rename(&file, file.with_extension("json.before"));
                let why = format!(
                    "this device was joined to its hub by a credential sent in the clear, which is no longer taken.                      On the hub, run `{0} device invite <name>`; here, `{0} device join <invitation>`",
                    brand::CLI
                );
                let _ = self
                    .inner
                    .journal
                    .emit("device.unjoined", Source::actor("system"), json!({ "hub": hub, "why": why }))
                    .await;
                return Ok(());
            }
        };
        let Some(hub_key) = door::decode(&membership.hub_key).and_then(|key| key.try_into().ok()) else {
            return Err(Error::internal(
                "this device's record of its hub is damaged: it holds no key for the hub",
            ));
        };
        if !door::is_place(&membership.url) {
            return Err(Error::internal(format!(
                "this device's record of its hub says the hub is at `{}`, which is not somewhere a link goes. \
                 Leave and join again with an invitation (`{1} device leave`, then `{1} device join <invitation>`)",
                membership.url.chars().take(120).collect::<String>(),
                brand::CLI
            )));
        }
        let uplink = Arc::new(self.uplink_to(&membership.url, hub_key));
        self.link_to(uplink, membership.allow, &membership.url, &fingerprint(&hub_key));
        Ok(())
    }

    /// Stops the link to the hub and every member's link to this device.
    /// Nothing is forgotten but the sessions: the next start picks both up
    /// again, each member with a new handshake.
    pub fn halt(&self) {
        if let Some(joined) = self.inner.joined.lock().expect("gateway lock").take() {
            // Silenced first, so that work called off by the stop is not
            // reported to the hub as the work's own outcome.
            joined.member.halt();
            joined.stop.cancel();
        }
        self.inner.door.halt();
        self.inner.hub.halt();
    }

    // ---- as a hub -------------------------------------------------------------

    /// Every name a step may be sent to: the members, and those invited
    /// that have yet to come.
    async fn names(&self) -> Result<Vec<String>> {
        let mut names: Vec<String> =
            self.inner.door.members().await?.into_iter().map(|(name, _)| name).collect();
        names.extend(self.inner.door.invited().await?.into_iter().map(|(name, _)| name));
        names.sort();
        names.dedup();
        Ok(names)
    }

    /// Makes an invitation for a device to join this fleet as `name`. It is
    /// shown once, works once, and stops working at `expires` from now (a
    /// day, unless said; never more than fourteen). `at` is where this hub
    /// can be reached from the device.
    pub async fn invite(
        &self,
        name: &str,
        expires: Option<Duration>,
        at: Vec<String>,
        by: &Principal,
    ) -> Result<Invited> {
        by.require("admin")?;
        if !is_device_name(name) {
            return Err(Error::invalid(
                "a device's name is lowercase letters, digits and dashes, e.g. `builder-1`",
            ));
        }
        if name == self.inner.journal.device() {
            return Err(Error::invalid(format!("`{name}` is this device's own name")));
        }
        if self.inner.door.members().await?.iter().any(|(member, _)| member == name) {
            return Err(Error::conflict(format!(
                "a device called `{name}` is in this fleet already. Remove it first (`{} device remove {name}`) to let another in under that name, or invite this one under another",
                brand::CLI
            )));
        }
        let expires = expires.unwrap_or(door::USUAL);
        if expires > door::LONGEST || expires.is_zero() {
            return Err(Error::invalid(
                "an invitation works for a day unless said, and for no more than fourteen: say `--expires` as something like 2h or 3d",
            ));
        }
        if at.is_empty() {
            return Err(Error::invalid(
                "an invitation says where this hub can be reached, and there is nowhere to say: give `--at http://host:port`",
            ));
        }
        if let Some(wrong) = at.iter().find(|place| !place.starts_with("http://")) {
            return Err(Error::invalid(format!(
                "`{wrong}` is not somewhere a device can link to: an address looks like `http://host:port`.                  It need not be a network anyone trusts: what crosses is sealed between the two"
            )));
        }
        let until = self.inner.journal.now_ms() + expires.as_millis() as i64;
        let invitation = self.inner.door.invite(name, until, at).await?;
        let _ = self
            .inner
            .journal
            .emit("device.invited", Source::actor(by.actor()), json!({ "device": name, "expires": until }))
            .await;
        Ok(Invited { name: name.to_string(), invitation: invitation.written(), expires: until })
    }

    /// Removes a device: it is no longer let in, an invitation waiting for
    /// it no longer works, its link is closed, and the steps that were
    /// waiting on it end as failed. What it was doing for this hub it
    /// stops when the hub next tells it that it is out (`Member::run`).
    pub async fn remove(&self, name: &str, by: &Principal) -> Result<()> {
        by.require("admin")?;
        if !self.inner.door.remove(name).await? {
            return Err(Error::not_found(format!("there is no device called `{name}` in this fleet")));
        }
        self.inner.hub.taken_out(name);
        let _ = self
            .inner
            .journal
            .emit("device.removed", Source::actor(by.actor()), json!({ "device": name }))
            .await;
        Ok(())
    }

    /// The members of this fleet and how each stands, and after them the
    /// devices invited that have yet to come.
    pub async fn devices(&self) -> Result<Vec<Device>> {
        let mut devices = Vec::new();
        for (name, member) in self.inner.door.members().await? {
            let since = self.inner.hub.linked_since(&name);
            let seen = match self.inner.hub.seen(&name) {
                Some(seen) => Some(seen),
                // Not seen since this daemon started: what the last one knew.
                None => self
                    .inner
                    .journal
                    .kv_get(copy::SEEN, &name)
                    .await?
                    .and_then(|saved| serde_json::from_value::<hub::Seen>(saved).ok()),
            };
            devices.push(Device {
                connected: since.is_some(),
                since,
                last_seen: seen.as_ref().map(|seen| seen.last_seen).filter(|at| *at > 0),
                info: seen.and_then(|seen| seen.info),
                id: Some(member.id()),
                invited: None,
                name,
            });
        }
        for (name, until) in self.inner.door.invited().await? {
            if devices.iter().all(|device| device.name != name) {
                devices.push(Device {
                    name,
                    connected: false,
                    since: None,
                    last_seen: None,
                    info: None,
                    id: None,
                    invited: Some(until),
                });
            }
        }
        devices.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(devices)
    }

    fn device_of(who: &Principal) -> Result<&str> {
        if who.kind != PrincipalKind::Device {
            return Err(Error::denied("only a device, by the key it proved, can open its link"));
        }
        Ok(&who.id)
    }

    /// Opens the link for the device `who` is. Whoever calls this has
    /// already held the device to its key ([`Gateway::sealed_link`]): it
    /// is not something anything outside this crate can ask.
    async fn open_link(&self, who: &Principal) -> Result<LinkStream> {
        Ok(self.inner.hub.open_link(Self::device_of(who)?).await)
    }

    async fn report(&self, who: &Principal, report: Report) -> Result<Ack> {
        Ok(self.inner.hub.report(Self::device_of(who)?, report).await)
    }

    /// The device a session is for, as what may act here: its record's
    /// name and scopes.
    fn device(linked: &Linked) -> Principal {
        Principal {
            kind: PrincipalKind::Device,
            id: linked.device.clone(),
            name: linked.device.clone(),
            scopes: linked.scopes.clone(),
            key: None,
        }
    }

    /// A device's first message of the handshake, from `from`; gives the
    /// answer to send back ([`door::Door::hello`]).
    pub async fn hello(&self, message: &[u8], from: IpAddr) -> std::result::Result<Vec<u8>, Shut> {
        let answered = self.inner.door.hello(message, from).await?;
        // Let in under a new name, a device's link under the old is over.
        for name in &answered.left {
            self.inner.hub.unlink(name);
        }
        Ok(answered.answer)
    }

    /// Opens a device's link, asked for with a sealed frame of the session
    /// it shook hands in.
    pub async fn sealed_link(&self, body: &[u8]) -> Result<SealedStream> {
        let (linked, frame) = self.inner.door.session(body)?;
        linked.session.open_next(Kind::Open, frame).map_err(|err| Error::unauthenticated(err.to_string()))?;
        self.inner.door.proved(&linked)?;
        let stream = self.open_link(&Self::device(&linked)).await?;
        Ok(SealedStream { stream, linked, hub: self.inner.hub.clone() })
    }

    /// Takes a device's report as a sealed frame and gives the answer as one.
    pub async fn sealed_report(&self, body: &[u8]) -> Result<Vec<u8>> {
        let (linked, frame) = self.inner.door.session(body)?;
        // Where the report's frame began is what the answer names, so that
        // it is the answer to this report and to no other.
        let (number, content) = linked
            .session
            .open_next(Kind::Report, frame)
            .map_err(|err| Error::unauthenticated(err.to_string()))?;
        self.inner.door.proved(&linked)?;
        // In words of our own: what would not read was sealed, and an
        // account of why could quote it where nothing is sealed.
        let report: Report = serde_json::from_slice(&content)
            .map_err(|_| Error::invalid("what was sealed there is not a report this hub can read"))?;
        let ack = self.report(&Self::device(&linked), report).await?;
        let mut answer = number.to_be_bytes().to_vec();
        answer.extend(serde_json::to_vec(&ack).unwrap_or_default());
        let (_, sealed) =
            linked.session.seal(Kind::Ack, &answer).map_err(|err| Error::internal(err.to_string()))?;
        Ok(sealed)
    }

    // ---- as a member ------------------------------------------------------------

    /// Joins the fleet of the hub that made `invitation`, at the first of
    /// the places it names where that hub answers and proves its key.
    /// `allow` is what the hub may do here; everything, unless narrowed.
    pub async fn join(&self, invitation: &str, allow: Option<Vec<String>>, by: &Principal) -> Result<()> {
        by.require("admin")?;
        let invitation = Invitation::read(invitation)?;
        let hub_key = invitation.hub_key();
        if hub_key == self.inner.keys.public {
            return Err(Error::invalid(
                "that invitation was made by this installation: a hub does not join itself",
            ));
        }
        let allow = allow.unwrap_or_else(|| vec!["*".to_string()]);
        let mut unreached = Vec::new();
        for place in &invitation.at {
            let url = place.trim_end_matches('/');
            let uplink = Arc::new(self.uplink_to(url, hub_key));
            // Tried before it is kept: an invitation that does not work is
            // an error now, not a link that quietly never comes up.
            match uplink.hello(Some(invitation.secret.clone())).await {
                Ok(_) => {
                    let membership = Membership {
                        url: url.to_string(),
                        hub_key: invitation.key.clone(),
                        allow: allow.clone(),
                    };
                    own::write_private(&self.membership_file(), json!(membership).to_string().as_bytes())?;
                    let hub_id = fingerprint(&hub_key);
                    self.link_to(uplink, allow, url, &hub_id);
                    let _ = self.inner.journal.kv_set(copy::UNJOINED, "hub", &serde_json::Value::Null).await;
                    let _ = self
                        .inner
                        .journal
                        .emit("device.joined", Source::actor(by.actor()), json!({ "hub": url, "id": hub_id }))
                        .await;
                    return Ok(());
                }
                Err(err) if err.refused => {
                    return Err(Error::denied(format!(
                        "the hub would not let this device in: {}",
                        err.message
                    )));
                }
                Err(err) if err.unproven => return Err(Error::denied(err.message)),
                Err(err) => unreached.push(err.message),
            }
        }
        Err(Error::unavailable(unreached.join("; ")))
    }

    /// Leaves the hub's fleet. What the hub started here keeps running; it
    /// simply is no longer the hub's to direct.
    pub async fn leave(&self, by: &Principal) -> Result<()> {
        by.require("admin")?;
        let Some(joined) = self.inner.joined.lock().expect("gateway lock").take() else {
            return Err(Error::not_found("this device has not joined a hub"));
        };
        joined.member.halt();
        joined.stop.cancel();
        let _ = std::fs::remove_file(self.membership_file());
        // And the count of what was said to that hub: it was that hub's
        // business, and is nothing to the next.
        self.inner.said.lock().expect("gateway lock").retain(|hub, _| fingerprint(hub) != joined.hub_id);
        let _ = std::fs::remove_file(said_file(&self.inner.data_dir, &joined.hub_id));
        let _ = self
            .inner
            .journal
            .emit("device.left", Source::actor(by.actor()), json!({ "hub": joined.url }))
            .await;
        Ok(())
    }

    /// The hub this device had joined says, sealed, that the device is no
    /// longer one of its fleet. The membership is forgotten, as it is when
    /// the device leaves, so that nothing asks that hub again, now or
    /// after a restart; it is said once (`device.unjoined`), and kept for
    /// whoever next looks at this device's devices to be told, with how to
    /// join again ([`Gateway::unjoined`]). `member` is the link's own: if
    /// the device has joined something else meanwhile, that is left alone.
    async fn taken_out(&self, hub: &str, member: &Member, said: &str) {
        let ours = {
            let mut joined = self.inner.joined.lock().expect("gateway lock");
            match joined.as_ref() {
                Some(now) if now.member.is(member) => joined.take(),
                _ => None,
            }
        };
        let Some(joined) = ours else { return };
        joined.stop.cancel();
        let _ = std::fs::remove_file(self.membership_file());
        self.inner.said.lock().expect("gateway lock").retain(|hub, _| fingerprint(hub) != joined.hub_id);
        let _ = std::fs::remove_file(said_file(&self.inner.data_dir, &joined.hub_id));
        // What the hub's steps had started here earlier is not stopped:
        // that is this device's owner's to say. It is named, with how.
        let started = self.started_by_hub().await;
        let still = match started.as_slice() {
            [] => String::new(),
            running => format!(
                " What the hub had started here earlier is still running, as it was started, and is not stopped \
                 unasked: {}. Stop one with `{} stop <id>`.",
                running
                    .iter()
                    .map(|one| format!(
                        "{} ({})",
                        one["id"].as_str().unwrap_or_default(),
                        one["blueprint"].as_str().unwrap_or_default()
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                brand::CLI
            ),
        };
        let why = format!(
            "its hub says this device is no longer one of its fleet. The steps it was doing for the hub were \
             stopped, and it asks that hub nothing more.{still} {said}"
        );
        let kept = json!({ "hub": hub, "why": why, "started": started, "at": self.inner.journal.now_ms() });
        let _ = self.inner.journal.kv_set(copy::UNJOINED, "hub", &kept).await;
        let _ = self
            .inner
            .journal
            .emit("device.unjoined", Source::actor("system"), json!({ "hub": hub, "why": why }))
            .await;
    }

    /// The instances a hub's steps started on this device that are alive
    /// now (`copy::STARTED`), each by its id, its blueprint and its key;
    /// and nothing more is kept of them, since the hub is one this device
    /// is leaving.
    async fn started_by_hub(&self) -> Vec<serde_json::Value> {
        let kept = self.inner.journal.kv_list(copy::STARTED).await.unwrap_or_default();
        let alive = match self.engine() {
            Some(engine) => {
                let filter =
                    conductor::InstanceFilter { live_only: true, limit: 100_000, ..Default::default() };
                engine.instances(filter).await.unwrap_or_default()
            }
            None => Vec::new(),
        };
        let mut started = Vec::new();
        for (instance, _) in kept {
            if let Some(one) = alive.iter().find(|one| one.id == instance) {
                started.push(json!({ "id": one.id, "blueprint": one.blueprint, "key": one.key }));
            }
            let _ = self.inner.journal.kv_set(copy::STARTED, &instance, &serde_json::Value::Null).await;
        }
        started
    }

    /// That the hub this device had joined took it out, if it did and the
    /// device has joined nothing since: which hub, when, and why, with how
    /// to join again.
    pub async fn unjoined(&self) -> Option<serde_json::Value> {
        if self.inner.joined.lock().expect("gateway lock").is_some() {
            return None;
        }
        self.inner.journal.kv_get(copy::UNJOINED, "hub").await.ok().flatten().filter(|kept| !kept.is_null())
    }

    /// Starts keeping a link open through `uplink`, replacing any there was.
    /// This is the member's own side, given a way to a hub by whoever holds
    /// this gateway; it takes nobody's word for anything.
    pub fn link(&self, uplink: Arc<dyn Uplink>, allow: Vec<String>, url: &str) {
        self.link_to(uplink, allow, url, "");
    }

    fn link_to(&self, uplink: Arc<dyn Uplink>, allow: Vec<String>, url: &str, hub_id: &str) {
        let Some(engine) = self.engine() else { return };
        let member = Member::new(engine, self.inner.hosting.clone(), self.inner.data_dir.clone(), allow);
        let stop = Cancel::new();
        tokio::spawn({
            let (member, stop) = (member.clone(), stop.clone());
            // Not held: this goes on for as long as the gateway does, and
            // must not be what keeps it.
            let (gateway, hub) = (Arc::downgrade(&self.inner), url.to_string());
            async move {
                let Some(why) = member.run(uplink, stop).await else { return };
                if let Some(inner) = gateway.upgrade() {
                    Gateway { inner }.taken_out(&hub, &member, &why).await;
                }
            }
        });
        let joined = Joined { url: url.to_string(), hub_id: hub_id.to_string(), member, stop };
        if let Some(old) = self.inner.joined.lock().expect("gateway lock").replace(joined) {
            old.stop.cancel();
        }
    }

    /// How this device's link to its hub stands, if it has joined one.
    pub fn uplink(&self) -> Option<HubLink> {
        let joined = self.inner.joined.lock().expect("gateway lock");
        joined.as_ref().map(|joined| HubLink {
            url: joined.url.clone(),
            id: joined.hub_id.clone(),
            status: joined.member.status(),
        })
    }
}
