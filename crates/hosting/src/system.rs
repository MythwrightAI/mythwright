//! The `system.check` action: is there room on this device to do the work?

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, ActionResult, Registry};
use serde::Serialize;
use serde_json::{Value, json};
use sysinfo::{Disks, System};

/// What this device has, right now.
#[derive(Clone, Debug, Serialize)]
pub struct Resources {
    pub cpus: usize,
    /// One-minute load average. Absent where the platform has none.
    pub load1: Option<f64>,
    pub total_mem_mb: u64,
    pub free_mem_mb: u64,
    /// Space left on the volume holding the directory asked about.
    pub free_disk_mb: Option<u64>,
    /// Why that is not known, where it is not: a sentence that names the
    /// directory and what the system lists ([`unplaced`]).
    #[serde(skip)]
    pub unplaced: Option<String>,
    pub os: String,
    pub arch: String,
    pub host: String,
}

/// Reads this device's resources. `path` picks the volume to report on.
pub fn resources(path: &std::path::Path) -> Result<Resources, String> {
    let mut system = System::new().map_err(|err| format!("cannot read system information: {err}"))?;
    system.refresh_memory();
    let mb = |bytes: u64| bytes / (1024 * 1024);
    let path = kernel::whole::whole(path).unwrap_or_else(|_| path.to_path_buf());
    let path = path.to_string_lossy();
    let room = match Disks::new_with_refreshed_list() {
        Ok(disks) => {
            let mounts: Vec<String> =
                disks.iter().map(|disk| disk.mount_point().to_string_lossy().into_owned()).collect();
            let holds = volume(&path, &mounts, cfg!(windows)).and_then(|holds| disks.get(holds));
            holds.map(|disk| mb(disk.available_space())).ok_or_else(|| unplaced(&path, Some(&mounts)))
        }
        Err(_) => Err(unplaced(&path, None)),
    };
    let (free_disk_mb, unplaced) = (room.as_ref().ok().copied(), room.err());
    Ok(Resources {
        cpus: std::thread::available_parallelism().map_or(1, usize::from),
        load1: System::load_average().ok().map(|load| load.one).filter(|load| *load >= 0.0),
        total_mem_mb: mb(system.total_memory()),
        free_mem_mb: mb(system.available_memory()),
        free_disk_mb,
        unplaced,
        os: System::name().unwrap_or_else(|_| std::env::consts::OS.to_string()),
        arch: System::cpu_arch(),
        host: System::host_name().unwrap_or_default(),
    })
}

/// Which of `mounts` is the volume that holds `path`: the one mounted
/// deepest of those the path is under, name by name, or none where it is
/// under none of them (a path that does not say where it starts, a share
/// on another machine, a volume with no letter). The last of two mounted
/// in one place, which is the one that is seen there.
///
/// Both are text, and which system's is said: on `windows` a path is
/// under `C:\` whichever form it is written in (`kernel::whole`), with
/// either stroke, and in whatever case, as the system reads names there;
/// anywhere else names are parted by `/` alone and are as they are
/// written.
pub fn volume<M: AsRef<str>>(path: &str, mounts: &[M], windows: bool) -> Option<usize> {
    let names = |text: &str| -> Vec<String> {
        let strokes: &[char] = if windows { &['/', '\\'] } else { &['/'] };
        text.split(strokes).filter(|name| !name.is_empty()).map(str::to_string).collect()
    };
    let holds = |mount: &str| match windows {
        true => kernel::whole::beneath(&path.to_lowercase(), &mount.to_lowercase()).is_some(),
        false => {
            let from_the_top = path.starts_with('/') && mount.starts_with('/');
            let (path, mount) = (names(path), names(mount));
            from_the_top
                && path.len() >= mount.len()
                && path.iter().zip(&mount).all(|(ours, its)| ours == its)
        }
    };
    let mounts = mounts.iter().map(AsRef::as_ref).enumerate();
    mounts.filter(|(_, mount)| holds(mount)).max_by_key(|(_, mount)| names(mount).len()).map(|(at, _)| at)
}

/// The most volumes a sentence lists.
const LISTED: usize = 8;

/// How every sentence of [`unplaced`] begins: that the room is not
/// known, which is not that there is too little of it.
pub const UNPLACED: &str = "free disk space could not be determined, which is not the same as too little";

/// Why the room left on disk for the folder `path` is not known, as a
/// sentence: the folder that was asked about, and that it is on none of
/// the volumes the system lists, with where those are mounted (`mounts`,
/// each said once; none where the system would not list them at all).
/// Whoever reads this has to be able to tell it from a full disk: a
/// folder on a share of another machine, or on a volume with no letter,
/// is counted as a device that is short by whatever only asks whether
/// there is enough.
pub fn unplaced(path: &str, mounts: Option<&[String]>) -> String {
    let Some(mounts) = mounts else {
        return format!(
            "{UNPLACED}: the system would not list its volumes, so which of them {path} is on is not known"
        );
    };
    let mut named: Vec<&str> = Vec::new();
    for mount in mounts {
        if !named.contains(&mount.as_str()) {
            named.push(mount);
        }
    }
    let more = match named.len().saturating_sub(LISTED) {
        0 => String::new(),
        more => format!(" and {more} more"),
    };
    match named.is_empty() {
        true => format!("{UNPLACED}: the system lists no volume, so {path} is on none"),
        false => format!(
            "{UNPLACED}: {path} is on none of the volumes the system lists ({}{more})",
            named[..named.len().min(LISTED)].join(", ")
        ),
    }
}

pub(crate) fn register(registry: &mut Registry) {
    let spec = ActionSpec::new("system.check", "Check that this device has the resources the work needs.")
        .example(CHECK_ASKED)
        .arg("min_free_mem_mb", T::Int, "memory that must be available")
        .arg("min_free_disk_mb", T::Int, "disk space that must be available where the project lives")
        .arg("max_load_per_cpu", T::Number, "the busiest the device may be, as load average per CPU")
        .arg("fail", T::Bool, "fail the step when a requirement is not met, instead of reporting `ok: false`")
        .capability("fleet:read")
        .output("ok", "whether every requirement is met")
        .output("reasons", "which requirements are not met, in words")
        .output("says", "those reasons as one line; nothing where every requirement is met")
        .output("free_mem_mb", "memory available")
        .output("free_disk_mb", "disk space available")
        .output("load1", "one-minute load average")
        .output("cpus", "CPUs")
        // Stood in, it looked at no device: every requirement is met.
        .stands(serde_json::json!({ "ok": true, "reasons": [], "says": "" }))
        .idempotent();
    registry.add_fn(spec, |cx, args| {
        Box::pin(async move {
            let workdir = cx.workdir();
            // Reading the disk list can block; keep it off the async threads.
            let found = tokio::task::spawn_blocking(move || resources(&workdir))
                .await
                .map_err(|err| ActionError::new(format!("the check crashed: {err}")))?
                .map_err(ActionError::new)?;

            let mut reasons: Vec<String> = Vec::new();
            if let Some(needed) = args.get("min_free_mem_mb").and_then(Value::as_u64)
                && found.free_mem_mb < needed
            {
                reasons.push(format!("{} MB of memory is free; {needed} MB is needed", found.free_mem_mb));
            }
            if let Some(needed) = args.get("min_free_disk_mb").and_then(Value::as_u64) {
                match found.free_disk_mb {
                    Some(free) if free < needed => {
                        reasons.push(format!("{free} MB of disk is free; {needed} MB is needed"));
                    }
                    // Not being able to tell is not the same as having enough.
                    None => reasons.push(found.unplaced.clone().unwrap_or_else(|| UNPLACED.to_string())),
                    _ => {}
                }
            }
            if let Some(limit) = args.get("max_load_per_cpu").and_then(Value::as_f64)
                && let Some(load) = found.load1
                && load / found.cpus as f64 > limit
            {
                reasons
                    .push(format!("load is {load:.1} on {} CPUs; the limit is {limit} per CPU", found.cpus));
            }

            let ok = reasons.is_empty();
            let mut output = serde_json::to_value(&found).unwrap_or(Value::Null);
            output["ok"] = json!(ok);
            output["says"] = json!(reasons.join("; "));
            output["reasons"] = json!(reasons);
            if !ok && args.get("fail").and_then(Value::as_bool).unwrap_or(false) {
                return Err(ActionError::new(reasons.join("; ")).with_data(output));
            }
            Ok::<Value, ActionError>(output)
        }) as kernel::BoxFut<'static, ActionResult>
    });
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const CHECK_ASKED: &str = "\
blueprint: room-to-build
permissions: [fleet:read]

steps:
  room:
    do: system.check
    with: { min_free_mem_mb: 2048, min_free_disk_mb: 4096 }
";
