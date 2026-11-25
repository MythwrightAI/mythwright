//! Choosing a device for a piece of work.
//!
//! `device:` says where a step runs, and takes an expression. What was
//! missing is something to put there when the answer is "wherever there is
//! room": `device.pick` chooses among this device and those linked to it,
//! and `device.list` hands a blueprint what is known of each, to choose by
//! a rule of its own.
//!
//! What is known of another device is what it last said of itself, which
//! it says again every heartbeat. Between two of those nothing here sees
//! what was just sent there, so ten pieces of work picked for in one moment
//! would all be given the one device that looked emptiest. Each pick is
//! therefore counted against the device it chose for a short while
//! ([`FRESH`]): as one more thing running there, and as the memory it
//! asked for. That spreads work that is asked for together.
//!
//! It is not a reservation. Nothing here holds a device to a number; what
//! does is a blueprint that counts what it placed (`one_at_a_time`, and
//! its `state`), and a device that is full says so of itself soon enough.

use std::sync::Mutex;

use conductor::InstanceFilter;
use serde::Serialize;

use crate::Gateway;

/// How long a pick is counted against the device it chose: two of a
/// member's heartbeats, by when what was started there is in what the
/// device says of itself.
pub const FRESH: i64 = 2 * crate::member::HEARTBEAT.as_millis() as i64;

/// How a device stands, as far as is known here.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Standing {
    pub device: String,
    /// Whether it is this device.
    pub here: bool,
    pub connected: bool,
    pub os: String,
    pub arch: String,
    pub cpus: usize,
    pub load1: Option<f64>,
    pub free_mem_mb: u64,
    pub free_disk_mb: Option<u64>,
    /// Instances alive there.
    pub instances: usize,
    /// How many pieces of work were picked for it lately ([`FRESH`]).
    pub picked: usize,
    /// How much memory those asked for between them.
    pub picked_mem_mb: u64,
}

/// What a piece of work asks of the device it is given.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    pub mem_mb: u64,
    pub disk_mb: u64,
    pub cpus: usize,
    pub os: Option<String>,
    pub arch: Option<String>,
    /// Only these, when said.
    pub among: Option<Vec<String>>,
    /// Whether this device itself will do.
    pub here: bool,
}

impl Standing {
    /// Whether it is one of those asked about at all.
    pub(crate) fn considered(&self, asked: &Asked) -> bool {
        let named = asked.among.as_ref().is_none_or(|among| among.contains(&self.device));
        named && (asked.here || !self.here)
    }

    /// Why this device will not do for what is asked, if it will not.
    fn lacks(&self, asked: &Asked) -> Option<String> {
        let other = |wanted: &Option<String>, has: &str| {
            wanted.as_ref().is_some_and(|wanted| !wanted.eq_ignore_ascii_case(has))
        };
        let free = self.free_mem_mb.saturating_sub(self.picked_mem_mb);
        if !self.connected {
            Some("is not connected".into())
        } else if other(&asked.os, &self.os) {
            Some(format!("runs {}", self.os))
        } else if other(&asked.arch, &self.arch) {
            Some(format!("is {}", self.arch))
        } else if self.cpus < asked.cpus {
            Some(format!("has {} processors", self.cpus))
        } else if free < asked.mem_mb {
            let placed = match self.picked_mem_mb {
                0 => String::new(),
                placed => format!(", less the {placed} MB just placed there"),
            };
            Some(format!("has {} MB of memory free{placed}", self.free_mem_mb))
        } else if asked.disk_mb > 0 {
            match self.free_disk_mb {
                None => Some("does not say how much disk it has free".into()),
                Some(free) if free < asked.disk_mb => Some(format!("has {free} MB of disk free")),
                Some(_) => None,
            }
        } else {
            None
        }
    }

    /// How busy it is for its size: what it is doing and what was just
    /// given it, for each processor it has.
    fn busy(&self) -> f64 {
        (self.load1.unwrap_or(0.0) + self.picked as f64) / self.cpus.max(1) as f64
    }
}

/// The device to give a piece of work to: of those that have what is
/// asked, the least busy for its size, then the one with most memory free.
/// Or why there is none, saying of each what it lacks.
pub fn choose<'a>(standing: &'a [Standing], asked: &Asked) -> Result<&'a Standing, String> {
    let considered: Vec<&Standing> = standing.iter().filter(|device| device.considered(asked)).collect();
    let mut fit: Vec<&Standing> =
        considered.iter().copied().filter(|device| device.lacks(asked).is_none()).collect();
    fit.sort_by(|one, other| {
        one.busy()
            .total_cmp(&other.busy())
            .then(other.free_mem_mb.cmp(&one.free_mem_mb))
            .then(one.device.cmp(&other.device))
    });
    if let Some(chosen) = fit.first() {
        return Ok(chosen);
    }
    if considered.is_empty() {
        return Err(match &asked.among {
            Some(among) => format!("none of {} is a device of this fleet", among.join(", ")),
            None => "there is no other device in this fleet to give this to".to_string(),
        });
    }
    let mut wanted = Vec::new();
    for (asked, what) in [(asked.mem_mb, "MB of memory"), (asked.disk_mb, "MB of disk")] {
        if asked > 0 {
            wanted.push(format!("{asked} {what}"));
        }
    }
    if asked.cpus > 0 {
        wanted.push(format!("{} processors", asked.cpus));
    }
    wanted.extend(asked.os.iter().chain(&asked.arch).cloned());
    let why: Vec<String> = considered
        .iter()
        .take(8)
        .filter_map(|device| Some(format!("{} {}", device.device, device.lacks(asked)?)))
        .collect();
    Err(format!(
        "no device has room for this ({}): {}",
        if wanted.is_empty() { "nothing in particular was asked".to_string() } else { wanted.join(", ") },
        why.join("; ")
    ))
}

/// What was picked for lately, by device.
#[derive(Default)]
pub(crate) struct Placed {
    lately: Mutex<Vec<(String, i64, u64)>>,
}

impl Placed {
    /// Counts what was picked lately against each device it was picked
    /// for, and forgets what is no longer fresh.
    pub fn weigh(&self, standing: &mut [Standing], now: i64) {
        let mut lately = self.lately.lock().expect("placed lock");
        lately.retain(|(_, at, _)| now - *at < FRESH);
        for (device, _, mem_mb) in lately.iter() {
            if let Some(one) = standing.iter_mut().find(|one| &one.device == device) {
                one.picked += 1;
                one.picked_mem_mb += mem_mb;
            }
        }
    }

    pub fn add(&self, device: &str, now: i64, mem_mb: u64) {
        self.lately.lock().expect("placed lock").push((device.to_string(), now, mem_mb));
    }
}

impl Gateway {
    /// How every device this one could give work to stands: itself first,
    /// then each device of its fleet, with what was picked for each lately.
    pub async fn standing(&self) -> kernel::Result<Vec<Standing>> {
        let dir = self.inner.data_dir.clone();
        let own =
            tokio::task::spawn_blocking(move || hosting::resources(&dir)).await.ok().and_then(Result::ok);
        let instances = match self.engine() {
            Some(engine) => engine
                .instances(InstanceFilter { live_only: true, limit: 10_000, ..InstanceFilter::default() })
                .await
                .map_or(0, |instances| instances.len()),
            None => 0,
        };
        let mut standing = Vec::new();
        if let Some(own) = own {
            standing.push(Standing {
                device: self.inner.journal.device().to_string(),
                here: true,
                connected: true,
                os: own.os,
                arch: own.arch,
                cpus: own.cpus,
                load1: own.load1,
                free_mem_mb: own.free_mem_mb,
                free_disk_mb: own.free_disk_mb,
                instances,
                picked: 0,
                picked_mem_mb: 0,
            });
        }
        for device in self.devices().await? {
            // One that is only invited has said nothing of itself yet.
            let Some(info) = device.info else { continue };
            standing.push(Standing {
                device: device.name,
                here: false,
                connected: device.connected,
                os: info.os,
                arch: info.arch,
                cpus: info.cpus,
                load1: info.load1,
                free_mem_mb: info.free_mem_mb,
                free_disk_mb: info.free_disk_mb,
                instances: info.instances,
                picked: 0,
                picked_mem_mb: 0,
            });
        }
        self.inner.placed.weigh(&mut standing, self.inner.journal.now_ms());
        Ok(standing)
    }

    /// Chooses a device for a piece of work that asks `asked`, and counts
    /// it against that device for a while. The second of the pair is how
    /// many were asked about.
    pub async fn pick(&self, asked: &Asked) -> kernel::Result<Result<(Standing, usize), String>> {
        let standing = self.standing().await?;
        let considered = standing.iter().filter(|device| device.considered(asked)).count();
        Ok(choose(&standing, asked).cloned().map(|chosen| {
            self.inner.placed.add(&chosen.device, self.inner.journal.now_ms(), asked.mem_mb);
            (chosen, considered)
        }))
    }
}
