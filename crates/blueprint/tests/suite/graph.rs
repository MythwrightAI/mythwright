use blueprint::graph;
use serde_json::json;

use crate::check;

fn kv_graph() -> blueprint::Graph {
    let blueprint = check(include_str!("../fixtures/kv-cache.bp")).into_result().unwrap();
    graph(&blueprint)
}

#[test]
fn every_step_and_handler_becomes_a_node() {
    let graph = kv_graph();
    let ids: Vec<&str> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "main/check",
            "main/low_resources",
            "main/give_up",
            "main/build",
            "main/host",
            "on.unhealthy/@",
            "on.unhealthy/run"
        ]
    );
    let groups: Vec<(&str, &str)> = graph.groups.iter().map(|g| (g.id.as_str(), g.kind)).collect();
    assert_eq!(groups, [("main", "main"), ("on.unhealthy", "handler")]);
}

#[test]
fn edges_say_whether_the_order_was_written_or_implied() {
    let graph = kv_graph();
    let edges: Vec<(&str, &str, &str)> =
        graph.edges.iter().map(|edge| (edge.from.as_str(), edge.to.as_str(), edge.kind)).collect();
    assert_eq!(
        edges,
        [
            ("main/check", "main/low_resources", "then"),
            ("main/low_resources", "main/give_up", "then"),
            ("main/check", "main/build", "needs"),
            ("main/give_up", "main/build", "needs"),
            ("main/build", "main/host", "then"),
            ("on.unhealthy/@", "on.unhealthy/run", "fires"),
        ]
    );
}

#[test]
fn layers_follow_the_longest_path() {
    let graph = kv_graph();
    let layer = |id: &str| graph.nodes.iter().find(|node| node.id == id).unwrap().layer;
    assert_eq!(layer("main/check"), 0);
    assert_eq!(layer("main/low_resources"), 1);
    assert_eq!(layer("main/give_up"), 2);
    assert_eq!(layer("main/build"), 3, "build waits for give_up, not just check");
    assert_eq!(layer("main/host"), 4);
    assert_eq!(layer("on.unhealthy/@"), 0);
    assert_eq!(layer("on.unhealthy/run"), 1);
}

#[test]
fn a_node_carries_what_the_drawer_shows() {
    let graph = kv_graph();
    let build = graph.nodes.iter().find(|node| node.id == "main/build").unwrap();
    assert_eq!(build.kind, "action");
    assert_eq!(build.target, "shell");
    assert_eq!(build.timeout.as_deref(), Some("30m"));
    assert_eq!(build.retry.as_deref(), Some("1 more, 10s apart"));
    assert_eq!(build.measure.as_deref(), Some("build"));
    assert_eq!(build.watch[0].rule, "after 5m");
    assert_eq!(build.watch[1].rule, r"log matches ^error(\[E\d+\])?:");
    let points: Vec<&str> = build.hooks.iter().map(|hook| hook.point).collect();
    // In the order they are done: what is done about a failure, then what
    // is done after any ending.
    assert_eq!(points, ["on_fail", "on_finish"]);
    assert_eq!(build.hooks[0].when.as_deref(), Some("params.helper != \"\""));

    let low = graph.nodes.iter().find(|node| node.id == "main/low_resources").unwrap();
    assert_eq!(low.when.as_deref(), Some("!steps.check.out.ok"));

    let host = graph.nodes.iter().find(|node| node.id == "main/host").unwrap();
    assert_eq!((host.kind, host.target.as_str()), ("call", "std/slot"));

    let trigger = graph.nodes.iter().find(|node| node.id == "on.unhealthy/@").unwrap();
    assert_eq!((trigger.kind, trigger.target.as_str()), ("trigger", "event service.unhealthy"));
}

#[test]
fn a_node_knows_its_lines() {
    let source = "blueprint: b\nsteps:\n  first:\n    do: noop\n  second:\n    do: log\n    with:\n      message: hi\n";
    let graph = graph(&check(source).into_result().unwrap());
    let lines: Vec<(u32, u32)> = graph.nodes.iter().map(|node| (node.line, node.end_line)).collect();
    assert_eq!(lines, [(3, 4), (5, 8)]);
}

#[test]
fn parallel_branches_share_a_layer() {
    let source = "\
blueprint: b
steps:
  start: { do: noop }
  left: { do: noop, needs: [start] }
  right: { do: noop, needs: [start] }
  join: { do: noop, needs: [left, right] }
";
    let graph = graph(&check(source).into_result().unwrap());
    let layers: Vec<u32> = graph.nodes.iter().map(|node| node.layer).collect();
    assert_eq!(layers, [0, 1, 1, 2]);
}

#[test]
fn attachments_are_drawn_beside_the_flow() {
    let source = "blueprint: b\nuses:\n  logs: std/log-feed\n  beat: { blueprint: std/heartbeat, with: { every: 30s } }\nsteps:\n  a: { do: noop }\n";
    let graph = graph(&check(source).into_result().unwrap());
    let uses: Vec<(&str, &str)> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == "attachment")
        .map(|n| (n.name.as_str(), n.target.as_str()))
        .collect();
    assert_eq!(uses, [("logs", "std/log-feed"), ("beat", "std/heartbeat")]);
    assert_eq!(graph.nodes[1].with, json!({ "every": "30s" }));
}

#[test]
fn the_graph_serialises_for_the_client() {
    let value = serde_json::to_value(kv_graph()).unwrap();
    assert_eq!(value["name"], "kv-cache");
    assert_eq!(
        value["params"][0],
        json!({
            "name": "port", "type": "int", "default": 7379, "required": false, "description": null,
            "choices": [],
        })
    );
    // A param that may only be one of a few things says which, so that a
    // form for it can offer them and not ask for them to be typed.
    let profile =
        value["params"].as_array().unwrap().iter().find(|param| param["name"] == "profile").unwrap();
    assert_eq!(profile["choices"], json!(["debug", "release"]));
    assert_eq!(value["nodes"][0]["id"], "main/check");
}
