//! What a hub and a member say to each other, and the one seam between
//! them: [`Uplink`], a member's way of reaching its hub.
//!
//! The member always calls the hub, never the other way round, so a member
//! needs no address of its own and no open port. How the hub's address is
//! reached (a LAN, a VPN, a tunnel, a reverse proxy doing TLS) is decided
//! outside this product; nothing here knows or cares.

use std::collections::BTreeMap;
use std::time::Duration;

use conductor::{ActionError, ActionResult, FailureKind};
use kernel::{BoxFut, Fields, Principal, Source};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

/// Path a member shakes hands at, relative to the hub's address: its first
/// message of the handshake goes there, and the hub's comes back.
pub const HELLO_PATH: &str = "/api/gateway/hello";
/// Path of the stream of commands. Asked for with a sealed frame, answered
/// with sealed frames.
pub const LINK_PATH: &str = "/api/gateway/link";
/// Path a member posts its reports to, each a sealed frame answered by one.
pub const REPORT_PATH: &str = "/api/gateway/report";

/// Hub to member.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// First on every link: what each side is called.
    Welcome {
        hub: String,
        device: String,
    },
    Run(Box<RunCommand>),
    Cancel {
        id: String,
    },
}

/// One action to run on the member.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunCommand {
    /// The same for every delivery of the same attempt of the same step, so
    /// a command that arrives twice is run once.
    pub id: String,
    pub action: String,
    pub args: Fields,
    pub principal: Principal,
    pub project: Option<String>,
    pub origin: Source,
    pub attempt: u32,
    pub env: BTreeMap<String, String>,
    /// The secrets the step's program is to be handed, each by the
    /// variable it is to be in: names, and never what one is. The device
    /// that runs the program hands over what it keeps under each itself.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secrets: BTreeMap<String, String>,
}

/// Member to hub: how things stand, and what has happened since last time.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    /// Sent when the link comes up and with each heartbeat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<DeviceInfo>,
    /// Commands in progress, so the hub can tell what never arrived.
    #[serde(default)]
    pub running: Vec<String>,
    #[serde(default)]
    pub results: Vec<Outcome>,
    #[serde(default)]
    pub logs: Vec<LogEntry>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.info.is_none() && self.results.is_empty() && self.logs.is_empty()
    }
}

/// What the hub took. The member keeps a result until it is listed here.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ack {
    pub ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub id: String,
    pub ok: bool,
    #[serde(default)]
    pub output: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Failure>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Failure {
    pub message: String,
    #[serde(default)]
    pub data: Value,
    /// `failed`, `timed_out` or `cancelled`.
    pub kind: String,
}

impl Outcome {
    pub fn of(id: &str, result: &ActionResult) -> Outcome {
        match result {
            Ok(output) => Outcome { id: id.to_string(), ok: true, output: output.clone(), error: None },
            Err(err) => Outcome {
                id: id.to_string(),
                ok: false,
                output: Value::Null,
                error: Some(Failure {
                    message: err.message.clone(),
                    data: err.data.clone(),
                    kind: match err.kind {
                        FailureKind::Failed => "failed",
                        FailureKind::TimedOut => "timed_out",
                        FailureKind::Cancelled => "cancelled",
                    }
                    .to_string(),
                }),
            },
        }
    }

    pub fn into_result(self) -> ActionResult {
        if self.ok {
            return Ok(self.output);
        }
        let failure = self.error.unwrap_or(Failure {
            message: "failed on the other device".into(),
            data: Value::Null,
            kind: "failed".into(),
        });
        Err(ActionError {
            message: failure.message,
            data: failure.data,
            kind: match failure.kind.as_str() {
                "timed_out" => FailureKind::TimedOut,
                "cancelled" => FailureKind::Cancelled,
                _ => FailureKind::Failed,
            },
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: String,
    pub stream: char,
    pub text: String,
}

/// A member's account of itself.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub cpus: usize,
    pub load1: Option<f64>,
    pub free_mem_mb: u64,
    pub free_disk_mb: Option<u64>,
    /// Instances alive on the member.
    pub instances: usize,
    pub services: Vec<ServiceBrief>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServiceBrief {
    pub name: String,
    pub status: String,
    pub port: u16,
}

/// Why the hub could not be reached, or would not talk.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkError {
    pub message: String,
    /// The hub asked for this long before the next try.
    pub retry_after: Option<Duration>,
    /// The hub, having proved it is the hub, said no: this device is not
    /// one it lets in.
    pub refused: bool,
    /// The hub, having proved it is the hub, said it does not know the key
    /// this device proved: a device that was in its fleet has been taken
    /// out of it.
    pub out: bool,
    /// What answered did not prove it is the hub this device joined.
    pub unproven: bool,
    /// Something answered in words that were not sealed. Anyone between the
    /// two could have written them, so nothing is taken from them but that
    /// the exchange did not work.
    pub said: bool,
    /// What was to be sent is more than is said at once over a link.
    /// Sending it again would be no better: whoever sent it must send less.
    pub too_much: bool,
}

impl LinkError {
    pub fn new(message: impl Into<String>) -> LinkError {
        LinkError {
            message: message.into(),
            retry_after: None,
            refused: false,
            out: false,
            unproven: false,
            said: false,
            too_much: false,
        }
    }
}

/// A member's way of reaching its hub.
pub trait Uplink: Send + Sync + 'static {
    /// Opens the stream of commands. The stream ending means the link is
    /// lost; the member opens another.
    fn open(&self) -> BoxFut<'static, Result<mpsc::Receiver<Command>, LinkError>>;

    fn report(&self, report: Report) -> BoxFut<'static, Result<Ack, LinkError>>;
}
