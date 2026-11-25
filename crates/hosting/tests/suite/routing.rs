use std::time::Duration;

use conductor::RunStatus;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpSocket, TcpStream};

use crate::{Rig, soon, step};

/// A backend that answers every connection with its name.
async fn backend(name: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let _ = stream.write_all(name.as_bytes()).await;
        }
    });
    address
}

/// An address nothing is listening on, for as long as what comes with it is
/// kept: a socket that has the port and does not listen. A port that was
/// only looked up and let go is anybody's, and where few are free the next
/// to ask for one is given it: the router of the same test, as often as not,
/// which then answered at the address that was to refuse.
fn dead_address() -> (String, TcpSocket) {
    let held = TcpSocket::new_v4().unwrap();
    held.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    (held.local_addr().unwrap().to_string(), held)
}

/// Connects through the router `count` times and returns who answered.
async fn ask(port: u64, count: usize) -> String {
    let mut answers = String::new();
    for _ in 0..count {
        let mut stream = TcpStream::connect(("127.0.0.1", port as u16)).await.unwrap();
        let mut answer = String::new();
        soon("an answer through the router", stream.read_to_string(&mut answer)).await.unwrap();
        answers.push_str(if answer.is_empty() { "-" } else { &answer });
    }
    answers
}

const ROUTE: &str = "\
blueprint: front
permissions: [fleet]
params: { backends: { type: list } }
steps:
  listen: { do: router.ensure, with: { name: front } }
  route: { do: router.route, with: { name: front, backends: ${params.backends} } }
outputs:
  port: ${ steps.listen.out.port }
";

async fn route(rig: &Rig, backends: Value) -> u64 {
    let applied = rig.apply(ROUTE, json!({ "backends": backends })).await;
    let run = rig.finished(&applied).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    run.outputs["port"].as_u64().unwrap()
}

#[tokio::test]
async fn connections_are_shared_out_by_weight_evenly_spread() {
    let rig = Rig::new().await;
    let (a, b) = (backend("a").await, backend("b").await);
    let port = route(&rig, json!([{ "address": a, "weight": 3 }, { "address": b, "weight": 1 }])).await;
    let answers = ask(port, 8).await;
    assert_eq!(answers.matches('a').count(), 6);
    assert_eq!(answers.matches('b').count(), 2);
    assert!(!answers.contains("bb"), "the smaller share is spread out, not bunched: {answers}");

    let info = &rig.hosting.routers()[0];
    let counted: Vec<(u32, u64, u64)> =
        info.backends.iter().map(|b| (b.weight, b.connections, b.failures)).collect();
    assert_eq!(counted, [(3, 6, 0), (1, 2, 0)]);
}

#[tokio::test]
async fn changing_the_weights_moves_new_connections_without_moving_the_port() {
    let rig = Rig::new().await;
    let (a, b) = (backend("a").await, backend("b").await);
    let port = route(&rig, json!([{ "address": a, "weight": 1 }, { "address": b, "weight": 0 }])).await;
    assert_eq!(ask(port, 4).await, "aaaa", "a weight of zero gets nothing");

    let again = route(&rig, json!([{ "address": a, "weight": 0 }, { "address": b, "weight": 1 }])).await;
    assert_eq!(again, port, "re-applying keeps the same router on the same port");
    assert_eq!(ask(port, 4).await, "bbbb");
    assert_eq!(rig.events("router.routed").await.len(), 2);
}

#[tokio::test]
async fn a_backend_that_will_not_answer_costs_the_client_nothing_and_is_counted() {
    let rig = Rig::new().await;
    let (good, (dead, _held)) = (backend("a").await, dead_address());
    let port = route(&rig, json!([{ "address": dead, "weight": 1 }, { "address": good, "weight": 1 }])).await;
    assert_eq!(ask(port, 6).await, "aaaaaa", "every connection was answered");

    let info = &rig.hosting.routers()[0];
    assert_eq!((info.backends[0].connections, info.backends[0].failures), (3, 3));
    assert_eq!((info.backends[1].connections, info.backends[1].failures), (6, 0));

    let series = rig
        .journal
        .query_metrics(journal::MetricQuery {
            name: "router.connection".into(),
            group_by: vec!["backend".into()],
            ..Default::default()
        })
        .await
        .unwrap();
    let failed: u64 = series.iter().map(|s| s.total.errors).sum();
    assert_eq!(failed, 3, "failed connections are measurements too");
}

const OBSERVE: &str = "\
blueprint: watch
permissions: [fleet]
params: { backend: { type: string }, max: 0 }
steps:
  look:
    do: router.observe
    with: { router: front, backend: ${params.backend}, for: 20s, max_fail_ratio: ${params.max}, min_connections: 1 }
";

/// Watches `backend` for the 20s window while four connections go through.
async fn observe(rig: &Rig, port: u64, backend: String) -> conductor::RunInfo {
    let since = rig.clock.sleep_count();
    let applied = rig.apply(OBSERVE, json!({ "backend": backend })).await;
    soon("the observation to begin", rig.clock.sleeps_started(since + 1)).await;
    ask(port, 4).await;
    rig.clock.advance(Duration::from_secs(20));
    rig.finished(&applied).await
}

#[tokio::test]
async fn observing_a_backend_passes_or_fails_on_what_happened_in_the_window() {
    let rig = Rig::new().await;
    let (good, (dead, _held)) = (backend("a").await, dead_address());
    let port = route(&rig, json!([{ "address": dead, "weight": 1 }, { "address": good, "weight": 1 }])).await;
    // Traffic before the window does not count.
    ask(port, 4).await;

    let run = observe(&rig, port, good.clone()).await;
    assert_eq!(step(&run, "look").output, json!({ "connections": 4, "failures": 0, "ratio": 0.0 }));

    let run = observe(&rig, port, dead.clone()).await;
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(
        step(&run, "look").error.as_deref(),
        Some(format!("`{dead}` did not pass observation: 2 of 2 connections failed").as_str())
    );
    assert_eq!(step(&run, "look").output["ratio"], 1.0);
}

#[tokio::test]
async fn observing_a_backend_that_saw_no_traffic_can_be_made_to_fail() {
    let rig = Rig::new().await;
    let good = backend("a").await;
    route(&rig, json!([{ "address": good, "weight": 1 }])).await;
    let since = rig.clock.sleep_count();
    let applied = rig.apply(OBSERVE, json!({ "backend": good })).await;
    soon("the observation to begin", rig.clock.sleeps_started(since + 1)).await;
    rig.clock.advance(Duration::from_secs(20));
    let run = rig.finished(&applied).await;
    assert!(
        step(&run, "look")
            .error
            .as_deref()
            .unwrap()
            .ends_with("it received 0 connections; at least 1 were expected")
    );
}

#[tokio::test]
async fn a_router_comes_back_after_a_restart_and_goes_away_with_its_instance() {
    let mut rig = Rig::new().await;
    let a = backend("a").await;
    let port = route(&rig, json!([{ "address": a, "weight": 1 }])).await;

    rig.restart().await;
    let routers = rig.hosting.routers();
    if routers.is_empty() {
        let failed =
            rig.journal.read(journal::EventQuery { topics: "router.failed".into(), ..Default::default() });
        panic!(
            "the router did not come back: {:?}",
            failed.await.unwrap().iter().map(|e| e.data.clone()).collect::<Vec<_>>()
        );
    }
    assert_eq!(routers[0].listen as u64, port);
    assert_eq!(ask(port, 2).await, "aa", "same port, same backends");

    let instance = rig.engine.instances(Default::default()).await.unwrap().remove(0);
    rig.engine.stop(&instance.id).await.unwrap();
    assert!(rig.hosting.routers().is_empty());
    soon("the port to close", async {
        while TcpStream::connect(("127.0.0.1", port as u16)).await.is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn routing_to_a_service_that_does_not_exist_is_refused() {
    let rig = Rig::new().await;
    let applied = rig.apply(ROUTE, json!({ "backends": [{ "service": "ghost", "weight": 1 }] })).await;
    let run = rig.finished(&applied).await;
    assert_eq!(step(&run, "route").error.as_deref(), Some("there is no service called `ghost` to route to"));
}
