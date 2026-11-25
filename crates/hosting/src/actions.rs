//! The `service.*` and `router.*` actions.

use std::time::Duration;

use blueprint::{ActionSpec, ValueType as T, duration};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

use crate::Hosting;
use crate::service::{ServiceSpec, Status};
use crate::shares::shares;

fn add<F, Fut>(registry: &mut Registry, hosting: &Hosting, spec: ActionSpec, run: F)
where
    F: Fn(Hosting, ActionCx, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let hosting = hosting.clone();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(run(hosting.clone(), cx, args))
    });
}

fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn millis(args: &Fields, key: &str, default: u64) -> Result<u64, ActionError> {
    match args.get(key).and_then(Value::as_str) {
        Some(text) => duration::parse(text).map_err(|err| ActionError::new(format!("`{key}`: {err}"))),
        None => Ok(default),
    }
}

/// What a service's `health` takes: the words `ServiceSpec::read` reads
/// there, each with its sentence, which a test holds to that reading.
pub const HEALTH: &[blueprint::inside::Row] = &[
    ("tcp", "bool", "`true`: it is healthy while its port takes a connection"),
    (
        "http",
        "string",
        "a path, such as `/health`: it is healthy while asking for that on its port is answered well",
    ),
    ("cmd", "string", "a command your shell reads: it is healthy while the command ends well"),
    ("interval", "duration", "how often it is asked (default 2s, and never more often than every 100ms)"),
    (
        "timeout",
        "duration",
        "how long an answer is waited for before the asking counts as failed (default 2s)",
    ),
    (
        "unhealthy_after",
        "int",
        "how many askings in a row fail before a healthy service is said to be unhealthy (default 3)",
    ),
];

pub(crate) fn register(registry: &mut Registry, hosting: &Hosting) {
    add(
        registry,
        hosting,
        ActionSpec::new("service.start", "Start a long-lived process and keep it running.")
.example(START_ASKED)
            .making("a service", "name")
            .runs("run")
            .runs("cmd")
            .runs("cli")
            .required("name", T::String, "what to call it; unique on this device")
            .arg("spec", T::Object, "everything below as one mapping, e.g. passed in as a param")
            .arg("run", T::String, crate::script::RUN_ARG)
            .arg(
                "cmd",
                T::List,
                "a program and its arguments, run without a shell; `{port}` in an argument is the port it is \
                 given and `{name}` what it is called, for a program that takes those as arguments",
            )
            .arg("cli", T::List, crate::shell::CLI_ARG)
            .arg("cwd", T::String, "directory to run in, relative to the project")
            .arg("env", T::Object, "extra environment variables")
            .arg("port", T::Int, "the port it listens on (default: a free one)")
            .arg("port_env", T::String, "the variable the port is passed in (default PORT)")
            .arg("health", T::Object, "how to tell it is healthy: one way of asking it, and how often")
            .within("health", HEALTH)
            .arg("restart", T::String, "always, on-failure (default) or never")
            .arg("max_restarts", T::Int, "restarts allowed per hour before giving up (default 6)")
            .arg(
                "restart_backoff",
                T::Duration,
                "wait before the first restart, doubling each time (default 1s)",
            )
            .arg(
                "stop_grace",
                T::Duration,
                "how long it gets to stop cleanly (default 5s); on Windows nothing is asked of it and it is ended \
                 at once",
            )
            .capability("fleet:write")
            .output("name", "the service's name")
            .output("port", "the port it was given")
            .output("pid", "its process id")
            .output("address", "where to reach it")
            .idempotent(),
        |hosting, cx, args| async move {
            // A service is a command that is run and kept running, whatever
            // it is called here: whoever this is done for must be allowed to
            // run commands, as well as to change what the fleet hosts.
            let by = cx.principal();
            if !by.can("shell") {
                return Err(ActionError::new(format!(
                    "{} may not use `shell`, which starting a service needs as well as `fleet:write`: a \
                     service is a command that is run. Have it started by someone who holds both",
                    by.name
                )));
            }
            let name = text(&args, "name").unwrap_or_default();
            // `spec:` and inline fields may be mixed; inline wins.
            let mut merged = args.get("spec").and_then(Value::as_object).cloned().unwrap_or_default();
            merged.extend(
                args.iter()
                    .filter(|(key, _)| *key != "spec" && *key != "name")
                    .map(|(k, v)| (k.clone(), v.clone())),
            );
            let mut spec = ServiceSpec::from_args(&merged).map_err(ActionError::new)?;
            // What it measures of its own is kept with what this step carries.
            spec.labels = cx.labels().clone();
            // The secrets it is to be handed, by name: read as it is started.
            spec.secrets = cx.secrets().clone();
            for (key, value) in cx.env() {
                spec.env.entry(key.clone()).or_insert_with(|| value.clone());
            }
            let cwd = match merged.get("cwd").and_then(Value::as_str) {
                Some(relative) => cx.workdir().join(relative),
                None => cx.workdir(),
            };
            let service = hosting.start_service(&name, cx.instance(), cx.project(), spec, cwd).await?;
            cx.hold(&format!("service:{name}")).await?;
            let info = service.info();
            Ok(
                json!({ "name": name, "port": info.port, "pid": info.pid, "address": format!("127.0.0.1:{}", info.port) }),
            )
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("service.wait_healthy", "Wait until a service passes its health check.")
            .example(WAIT_ASKED)
            .required("service", T::String, "which service")
            .arg("timeout", T::Duration, "how long to wait (default 60s)")
            .capability("fleet:read")
            .output("healthy", "true")
            .output("after_ms", "how long it took")
            .idempotent(),
        |hosting, cx, args| async move {
            let name = text(&args, "service").unwrap_or_default();
            let service = hosting
                .service(&name)
                .ok_or_else(|| ActionError::new(format!("there is no service called `{name}`")))?;
            let timeout = millis(&args, "timeout", 60_000)?;
            let started = cx.clock().now_ms();
            let mut state = service.state.subscribe();
            let deadline = cx.sleep(Duration::from_millis(timeout));
            tokio::pin!(deadline);
            loop {
                let now = state.borrow_and_update().clone();
                match now.status {
                    Status::Healthy => {
                        return Ok(json!({ "healthy": true, "after_ms": cx.clock().now_ms() - started }));
                    }
                    Status::Starting | Status::Unhealthy => {}
                    gone => {
                        let why = now.reason.unwrap_or_else(|| match now.last_exit {
                            Some(code) => format!("it exited with code {code}"),
                            None => "it is not running".into(),
                        });
                        return Err(ActionError::new(format!("`{name}` is {}: {why}", gone.as_str()))
                            .with_data(json!({ "status": gone.as_str(), "exit": now.last_exit })));
                    }
                }
                tokio::select! {
                    changed = state.changed() => {
                        if changed.is_err() {
                            return Err(ActionError::new(format!("`{name}` is no longer being looked after")));
                        }
                    }
                    waited = &mut deadline => {
                        waited?;
                        return Err(ActionError::timed_out(&duration::format(timeout))
                            .with_data(json!({ "status": now.status.as_str(), "restarts": now.restarts })));
                    }
                }
            }
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("service.stop", "Stop a service.")
            .example(STOP_ASKED)
            .required("service", T::String, "which service")
            .arg("missing_ok", T::Bool, "succeed if there is no such service (default true)")
            .capability("fleet:write")
            .output("stopped", "whether there was something to stop")
            .idempotent(),
        |hosting, cx, args| async move {
            let name = text(&args, "service").unwrap_or_default();
            let stopped = hosting.stop_service(&name).await;
            if !stopped && args.get("missing_ok").and_then(Value::as_bool) == Some(false) {
                return Err(ActionError::new(format!("there is no service called `{name}`")));
            }
            hosting.forget_service(&name).await;
            // The instance that started it may not be the one stopping it.
            let _ = cx.release(&format!("service:{name}")).await;
            Ok(json!({ "stopped": stopped }))
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("service.status", "Report on a service.")
            .example(STATUS_ASKED)
            .required("service", T::String, "which service")
            .capability("fleet:read")
            .output("name", "the service")
            .output("status", "starting, healthy, unhealthy, exited, stopped, failed or missing")
            .output("healthy", "whether it is healthy right now")
            .output("pid", "the process it is, where there is one")
            .output("port", "the port it was given, where it was given one")
            .output("restarts", "how many times it has been started again")
            .output("reason", "why it is as it is, when that needs saying")
            .output("log", "the file its output is written to")
            .idempotent(),
        |hosting, _cx, args| async move {
            let name = text(&args, "service").unwrap_or_default();
            Ok(match hosting.service(&name) {
                Some(service) => {
                    let info = service.info();
                    let mut value = serde_json::to_value(&info).unwrap_or(Value::Null);
                    value["healthy"] = json!(info.status == Status::Healthy);
                    value
                }
                None => json!({ "name": name, "status": "missing", "healthy": false }),
            })
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("router.ensure", "Make sure a router is listening.")
            .example(ROUTER_ASKED)
            .making("a router", "name")
            .required("name", T::String, "what to call it")
            .arg("listen", T::Int, "the port to listen on (default: a free one)")
            .arg("host", T::String, "the address to listen on (default 127.0.0.1)")
            .capability("fleet:write")
            .output("port", "the port it listens on")
            .output("address", "where to reach it")
            .idempotent(),
        |hosting, cx, args| async move {
            let name = text(&args, "name").unwrap_or_default();
            let host = text(&args, "host").unwrap_or_else(|| "127.0.0.1".into());
            let listen = args.get("listen").and_then(Value::as_u64).unwrap_or(0) as u16;
            let owner = (cx.instance(), cx.project());
            let router = hosting.ensure_router(&name, owner, &host, listen, Vec::new()).await?;
            cx.hold(&format!("router:{name}")).await?;
            Ok(json!({ "name": name, "port": router.listen, "address": format!("{host}:{}", router.listen) }))
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("router.route", "Set where a router sends new connections.")
.example(ROUTE_ASKED)
            .required("name", T::String, "which router")
            .required(
                "backends",
                T::List,
                "a list of `{ service: <name>, weight: <n> }` or `{ address: <host:port>, weight: <n> }`; one of \
                 them may say `weight: rest`, for what the others leave of a hundred",
            )
            .capability("fleet:write")
            .output("backends", "the weights now in effect")
            .idempotent(),
        |hosting, _cx, args| async move {
            let name = text(&args, "name").unwrap_or_default();
            let backends = shares(args.get("backends").unwrap_or(&Value::Null))
                .map_err(|why| ActionError::new(format!("`backends`: {why}")))?;
            let router = hosting.route(&name, backends).await?;
            Ok(json!({ "backends": router.backends() }))
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("router.observe", "Watch one backend for a while and fail if it misbehaves.")
            .example(OBSERVE_ASKED)
            .required("router", T::String, "which router")
            .required("backend", T::String, "the service (or address) to watch")
            .required("for", T::Duration, "how long to watch")
            .arg("max_fail_ratio", T::Number, "the share of its connections that may fail (default 0)")
            .arg("min_connections", T::Int, "fail unless it received at least this many (default 0)")
            .capability("fleet:read")
            .output("connections", "connections it was sent during the window")
            .output("failures", "how many of those it did not accept")
            .output("ratio", "failures / connections")
            .idempotent(),
        |hosting, cx, args| async move {
            let (name, backend) =
                (text(&args, "router").unwrap_or_default(), text(&args, "backend").unwrap_or_default());
            let router = hosting
                .router(&name)
                .ok_or_else(|| ActionError::new(format!("there is no router called `{name}`")))?;
            let window = millis(&args, "for", 0)?;
            // How often its process has ended, and how often it was started
            // again, as the watch begins.
            let lives = hosting.service(&backend).map(|service| {
                let state = service.state.borrow();
                (state.exits, state.restarts)
            });
            let before = router.stats(&backend);
            cx.sleep(Duration::from_millis(window)).await?;
            let after = router.stats(&backend);

            let connections = after.connections - before.connections;
            let failures = after.failures - before.failures;
            let ratio = if connections == 0 { 0.0 } else { failures as f64 / connections as f64 };
            let report = json!({ "connections": connections, "failures": failures, "ratio": ratio });
            let fail = |why: String| {
                Err(ActionError::new(format!("`{backend}` did not pass observation: {why}"))
                    .with_data(report.clone()))
            };

            // Only a service on this device has a health to check.
            if let Some(service) = hosting.service(&backend) {
                let info = service.info();
                if Some(info.restarts) != lives.map(|(_, restarts)| restarts) {
                    return fail("it restarted while being watched".into());
                }
                // One whose process ended in the last moments of the watch
                // is yet to be started again, and is said to be starting
                // until it is. With nobody sent to it since, it has refused
                // nothing either: what it did is what is said of it, before
                // how it stands.
                if Some(service.state.borrow().exits) != lives.map(|(exits, _)| exits) {
                    return fail("it exited while being watched".into());
                }
                if info.status != Status::Healthy {
                    return fail(format!("it is {}", info.status.as_str()));
                }
            }
            let limit = args.get("max_fail_ratio").and_then(Value::as_f64).unwrap_or(0.0);
            if ratio > limit {
                return fail(format!("{failures} of {connections} connections failed"));
            }
            let floor = args.get("min_connections").and_then(Value::as_u64).unwrap_or(0);
            if connections < floor {
                return fail(format!(
                    "it received {connections} connections; at least {floor} were expected"
                ));
            }
            Ok(report)
        },
    );

    add(
        registry,
        hosting,
        ActionSpec::new("router.remove", "Stop a router listening.")
            .example(REMOVE_ASKED)
            .required("name", T::String, "which router")
            .capability("fleet:write")
            .output("removed", "whether there was one")
            .idempotent(),
        |hosting, cx, args| async move {
            let name = text(&args, "name").unwrap_or_default();
            let removed = hosting.remove_router(&name).await;
            let _ = cx.release(&format!("router:{name}")).await;
            Ok(json!({ "removed": removed }))
        },
    );
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const START_ASKED: &str = "\
blueprint: web
permissions: [fleet:write]

steps:
  serve:
    do: service.start
    with: { name: web, cmd: [python3, -m, http.server, \"8080\"], port: 8080, health: { tcp: true } }
";
const WAIT_ASKED: &str = "\
blueprint: web-up
permissions: [fleet:read]

steps:
  up:
    do: service.wait_healthy
    with: { service: web, timeout: 1m }
";
const STOP_ASKED: &str = "\
blueprint: web-down
permissions: [fleet:write]

steps:
  down:
    do: service.stop
    with: { service: web, missing_ok: true }
";
const STATUS_ASKED: &str = "\
blueprint: web-status
permissions: [fleet:read]

steps:
  status:
    do: service.status
    with: { service: web }

  said:
    do: log
    with: { message: \"web is ${steps.status.out.status}\" }
";
const ROUTER_ASKED: &str = "\
blueprint: shop-front
permissions: [fleet:write]

steps:
  front:
    do: router.ensure
    with: { name: shop, listen: 8000 }
";
const ROUTE_ASKED: &str = "\
blueprint: shop-to-new
permissions: [fleet:write]

steps:
  route:
    do: router.route
    with:
      name: shop
      backends: [{ service: shop-v2, weight: 10 }, { service: shop-v1, weight: rest }]
";
const OBSERVE_ASKED: &str = "\
blueprint: shop-watched
permissions: [fleet:read]

steps:
  watched:
    do: router.observe
    with: { router: shop, backend: shop-v2, for: 1m, max_fail_ratio: 0.05 }
";
const REMOVE_ASKED: &str = "\
blueprint: shop-closed
permissions: [fleet:write]

steps:
  closed:
    do: router.remove
    with: { name: shop }
";
