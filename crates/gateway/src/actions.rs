//! Actions for starting and stopping whole blueprints, and for choosing
//! the device one is started on.
//!
//! A step sent to another device does one piece of work there and returns.
//! Something that must stay running on that device (a service, a router)
//! needs an instance *there* to own it, so the way to host on another
//! device is to apply a blueprint on it: `do: blueprint.apply` with
//! `device:`. Used without `device:` it starts an independent instance on
//! this one. Which device, when it is to be wherever there is room, is
//! `device.pick` ([`crate::place`]).

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Apply, InstanceFilter, Registry, RunStatus};
use serde_json::{Value, json};

use crate::Gateway;
use crate::place::Asked;

pub(crate) fn register(registry: &mut Registry, gateway: Gateway) {
    let fleet = gateway.clone();
    registry.add_fn(
        ActionSpec::new(
            "device.list",
            "How this device and each device of its fleet stands: what it has free, and how busy it is.",
        )
        .example(LIST_ASKED)
        .capability("fleet:read")
        .output(
            "devices",
            "each with `device`, `here`, `connected`, `os`, `arch`, `cpus`, `load1`, `free_mem_mb`, \
             `free_disk_mb`, `instances`, and `picked`, how many pieces of work were picked for it lately",
        )
        .idempotent(),
        move |_, _| {
            let fleet = fleet.clone();
            Box::pin(async move { Ok(json!({ "devices": fleet.standing().await? })) })
        },
    );
    registry.add_fn(
        ActionSpec::new(
            "device.pick",
            "Choose a device with room for a piece of work: of those that have what is asked, the least busy \
             for its size. What it chooses is counted against that device for a short while, so that work \
             asked for together is spread.",
        )
        .example(PICK_ASKED)
        .arg("mem_mb", T::Int, "how much memory it must have free")
        .arg("disk_mb", T::Int, "how much disk it must have free")
        .arg("cpus", T::Int, "how many processors it must have")
        .arg("os", T::String, "the system it must run, as `device.list` says it")
        .arg("arch", T::String, "the processor it must have, such as `x86_64` or `aarch64`")
        .arg("among", T::List, "only these devices, by name")
        .arg("here", T::Bool, "whether this device itself will do (default: true)")
        .capability("fleet:read")
        .output("device", "its name, to give to `device:`")
        .output("here", "whether it is this device")
        .output("free_mem_mb", "how much memory it said it had free")
        .output("cpus", "how many processors it has")
        .output("load1", "its load over the last minute")
        .output("considered", "how many devices were asked about")
        .idempotent(),
        move |_, args| {
            let gateway = gateway.clone();
            Box::pin(async move {
                let number = |key: &str| args.get(key).and_then(Value::as_u64).unwrap_or(0);
                let word = |key: &str| {
                    args.get(key).and_then(Value::as_str).filter(|word| !word.is_empty()).map(str::to_string)
                };
                let asked = Asked {
                    mem_mb: number("mem_mb"),
                    disk_mb: number("disk_mb"),
                    cpus: number("cpus") as usize,
                    os: word("os"),
                    arch: word("arch"),
                    among: args.get("among").and_then(Value::as_array).map(|among| {
                        among.iter().filter_map(|name| name.as_str().map(str::to_string)).collect()
                    }),
                    here: args.get("here").and_then(Value::as_bool).unwrap_or(true),
                };
                let (chosen, considered) = gateway.pick(&asked).await?.map_err(ActionError::new)?;
                Ok(json!({
                    "device": chosen.device,
                    "here": chosen.here,
                    "free_mem_mb": chosen.free_mem_mb,
                    "cpus": chosen.cpus,
                    "load1": chosen.load1,
                    "considered": considered,
                }))
            })
        },
    );
    registry.add_fn(
        ActionSpec::new(
            "blueprint.apply",
            "Start a blueprint as an instance of its own, or update the one it started before.",
        )
        .example(APPLY_ASKED)
        .making("an instance of its own of the blueprint", "blueprint")
        .arg("blueprint", T::String, "its name, as known on the device this runs on")
        .arg("source", T::String, "its text, when it is not known there by name")
        .arg("params", T::Object, "its parameters")
        .arg("key", T::String, "which instance this is; applying again with the same key updates it")
        .arg("wait", T::Bool, "wait for its steps to finish (default: true)")
        .capability("blueprints:run")
        .output("instance", "the instance's id")
        .output("status", "how its run ended, when waited for")
        .output("outputs", "what its run produced")
        .idempotent(),
        |cx, args| {
            Box::pin(async move {
                let text = |key: &str| args.get(key).and_then(Value::as_str).filter(|text| !text.is_empty());
                let source = text("source").map(str::to_string);
                let name = match (text("blueprint"), &source) {
                    (Some(name), _) => name.to_string(),
                    (None, Some(source)) => blueprint::check(source, &blueprint::NoCatalog)
                        .blueprint
                        .map(|blueprint| blueprint.name)
                        .ok_or_else(|| ActionError::new("`source` does not read as a blueprint"))?,
                    (None, None) => return Err(ActionError::new("give `blueprint` or `source`")),
                };
                let engine = cx.engine().clone();
                // A step asks, and nobody: a text of the project's own is
                // one the instance this step is of may run.
                let request = Apply {
                    blueprint: name,
                    source,
                    params: args.get("params").and_then(Value::as_object).cloned().unwrap_or_default(),
                    key: text("key").map(str::to_string),
                    parent: None,
                    target: None,
                    project: cx.project().map(str::to_string),
                    principal: cx.principal().clone(),
                    force: false,
                    take: false,
                    // What this step is measured with, what it starts is.
                    labels: cx.labels().clone(),
                    // Under the environment of the instance that asks.
                    environment: None,
                };
                let applied = engine.apply_within(request, cx.instance()).await?;
                let mut out = json!({
                    "instance": applied.instance,
                    "created": applied.created,
                    "changed": applied.changed,
                    "device": engine.journal().device(),
                });
                let wait = args.get("wait").and_then(Value::as_bool).unwrap_or(true);
                if let (true, Some(run)) = (wait, &applied.run) {
                    let finished = tokio::select! {
                        finished = engine.wait_run(run) => finished?,
                        () = cx.cancel().cancelled() => return Err(ActionError::cancelled()),
                    };
                    out["status"] = json!(finished.status);
                    out["outputs"] = finished.outputs.clone();
                    if finished.status != RunStatus::Ok {
                        let why = finished.error.unwrap_or_else(|| "it did not finish".to_string());
                        return Err(
                            ActionError::new(format!("`{}` failed: {why}", applied.instance)).with_data(out)
                        );
                    }
                }
                Ok(out)
            })
        },
    );

    registry.add_fn(
        ActionSpec::new(
            "blueprint.stop",
            "Stop an instance started with `blueprint.apply`, and what it holds.",
        )
        .example(UNAPPLY_ASKED)
        .arg("instance", T::String, "its id")
        .arg("key", T::String, "or the key it was applied with")
        .capability("blueprints:run")
        .output("stopped", "whether there was anything to stop")
        .idempotent(),
        |cx, args| {
            Box::pin(async move {
                let text = |key: &str| args.get(key).and_then(Value::as_str).filter(|text| !text.is_empty());
                let engine = cx.engine().clone();
                let id = match (text("instance"), text("key")) {
                    (Some(id), _) => Some(id.to_string()),
                    (None, Some(key)) => engine
                        .instances(InstanceFilter {
                            live_only: true,
                            roots_only: true,
                            limit: 10_000,
                            ..InstanceFilter::default()
                        })
                        .await?
                        .into_iter()
                        .find(|instance| instance.key == key)
                        .map(|instance| instance.id),
                    (None, None) => return Err(ActionError::new("give `instance` or `key`")),
                };
                match id {
                    Some(id) => {
                        engine.stop(&id).await?;
                        Ok(json!({ "stopped": true, "instance": id }))
                    }
                    None => Ok(json!({ "stopped": false })),
                }
            })
        },
    );
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const LIST_ASKED: &str = "\
blueprint: devices
permissions: [fleet:read]

steps:
  listed:
    do: device.list

  said:
    do: log
    with: { message: \"Devices: ${steps.listed.out.devices}\" }
";
const PICK_ASKED: &str = "\
blueprint: build-where-there-is-room
permissions: [fleet:read, shell]

steps:
  pick:
    do: device.pick
    with: { mem_mb: 4096, cpus: 4 }

  build:
    device: ${steps.pick.out.device}
    do: shell
    with: { cmd: [make] }
";
const APPLY_ASKED: &str = "\
blueprint: news-there
permissions: [blueprints:run]

steps:
  applied:
    do: blueprint.apply
    with: { blueprint: std/watch, key: board-news, params: { channel: general, collection: board } }
";
const UNAPPLY_ASKED: &str = "\
blueprint: news-gone
permissions: [blueprints:run]

steps:
  stopped:
    do: blueprint.stop
    with: { key: board-news }
";
