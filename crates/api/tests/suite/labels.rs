//! Labels, over HTTP: the same work given to two agents, each lot said to
//! be a variant, and their measurements asked for side by side.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// One piece of work for one agent: something the built-in assistant can do.
const LANE: &str = "\
blueprint: lane
permissions: [agents]
params:
  agent:
  title:
steps:
  work:
    do: agent.task
    with: { slot: \"${params.agent}\", prompt: \"add a card ${params.title}\" }
";

/// The same work handed to two agents, and each lot said to be a variant.
const TRIAL: &str = "\
blueprint: trial
permissions: [blueprints]
params:
  item: 7
steps:
  a:
    call: lane
    as: lane-a
    labels: { experiment: \"cart-${params.item}\", variant: a }
    with: { agent: ada, title: A cart }
  b:
    call: lane
    as: lane-b
    labels: { experiment: \"cart-${params.item}\", variant: b }
    with: { agent: grace, title: A cart }
";

/// A project with a board, two agents that are the built-in assistant, and
/// the two blueprints in it.
async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    for agent in ["ada", "grace"] {
        rig.send("/api/agents", json!({ "name": agent, "adapter": "builtin", "project": "shop" })).await;
    }
    for (name, content) in [("lane", LANE), ("trial", TRIAL)] {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let body = json!({ "path": path, "content": content });
        let (status, answer) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
}

/// `(the labels a series is split by, how many were measured)`, for each series.
fn counted(answer: &Value) -> Vec<(Value, u64)> {
    let series = answer["series"].as_array().unwrap();
    series.iter().map(|one| (one["labels"].clone(), one["total"]["count"].as_u64().unwrap())).collect()
}

#[tokio::test]
async fn the_turns_two_agents_take_for_the_same_work_are_measured_side_by_side() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // A turn that is for nobody's experiment, to be told from the others.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "ada" })).await;
    rig.send("/api/channels/general/messages", json!({ "body": "hello" })).await;
    rig.count("agent.turn.finished", 1).await;

    let applied =
        rig.send("/api/apply", json!({ "blueprint": "trial", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let board = rig.read("/api/board/shop").await;
    assert_eq!(board["items"].as_array().unwrap().len(), 2, "each agent did the work: {board}");

    // The turn each agent took carries what the step that called its lane said.
    let turns = rig.read("/api/metrics?name=agent.turn&by=variant,slot&labels=experiment:cart-7").await;
    assert_eq!(
        counted(&turns),
        [(json!({ "slot": "ada", "variant": "a" }), 1), (json!({ "slot": "grace", "variant": "b" }), 1)]
    );
    // So do the steps and the runs of each lane, and nothing of the blueprint that called them.
    let steps = rig.read("/api/metrics?name=step&by=blueprint,variant&labels=experiment:cart-7").await;
    assert_eq!(
        counted(&steps),
        [
            (json!({ "blueprint": "lane", "variant": "a" }), 1),
            (json!({ "blueprint": "lane", "variant": "b" }), 1)
        ]
    );
    let runs = rig.read("/api/metrics?name=run&by=variant&labels=blueprint:trial").await;
    assert_eq!(counted(&runs), [(json!({}), 1)]);

    // What was asked before there were labels is answered as it was.
    let by_slot = rig.read("/api/metrics?name=agent.turn&by=slot").await;
    assert_eq!(counted(&by_slot), [(json!({ "slot": "ada" }), 2), (json!({ "slot": "grace" }), 1)]);
    assert_eq!(counted(&rig.read("/api/metrics?name=agent.turn").await), [(json!({}), 3)]);

    // The lanes say what they are labelled with.
    let lanes = rig.read(&format!("/api/instances?parent={}", applied["instance"].as_str().unwrap())).await;
    let labels: Vec<&Value> =
        lanes["instances"].as_array().unwrap().iter().map(|one| &one["labels"]).collect();
    assert!(labels.contains(&&json!({ "experiment": "cart-7", "variant": "a" })), "{labels:?}");
}

#[tokio::test]
async fn the_labels_there_are_can_be_asked_for_overall_or_for_a_project() {
    let rig = Rig::new().await;
    shop(&rig).await;
    assert_eq!(
        rig.read("/api/metrics/labels").await,
        json!({ "labels": {} }),
        "none until a blueprint says one"
    );

    rig.send("/api/apply", json!({ "blueprint": "trial", "project": "shop", "wait": true })).await;
    rig.send(
        "/api/apply",
        json!({ "blueprint": "trial", "project": "shop", "params": { "item": 8 }, "force": true, "wait": true }),
    )
    .await;
    let all = json!({ "labels": { "experiment": ["cart-7", "cart-8"], "variant": ["a", "b"] } });
    assert_eq!(rig.read("/api/metrics/labels").await, all);
    assert_eq!(rig.read("/api/metrics/labels?labels=project:shop").await, all);
    assert_eq!(rig.read("/api/metrics/labels?labels=project:nonesuch").await, json!({ "labels": {} }));
    // Of one measurement, and narrowed by a label like any other.
    assert_eq!(
        rig.read("/api/metrics/labels?name=agent.turn&labels=experiment:cart-8,variant:b").await,
        json!({ "labels": { "experiment": ["cart-8"], "variant": ["b"] } })
    );
    // After a restart they are still what is asked by.
    let mut rig = rig;
    rig.restart().await;
    assert_eq!(rig.read("/api/metrics/labels?since=2h").await, all);
}

#[tokio::test]
async fn a_blueprint_applied_directly_can_be_given_labels_and_a_bad_one_is_refused() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let lane = |labels: Value| {
        json!({
            "blueprint": "lane", "project": "shop", "wait": true, "labels": labels,
            "params": { "agent": "ada", "title": "By hand" },
        })
    };
    let applied = rig.send("/api/apply", lane(json!({ "experiment": "by-hand", "round": 2 }))).await;
    assert_eq!(applied["finished"]["status"], "ok", "{}", applied["finished"]);
    let turns = rig.read("/api/metrics?name=agent.turn&by=experiment,round").await;
    assert_eq!(counted(&turns), [(json!({ "experiment": "by-hand", "round": "2" }), 1)]);

    for (labels, why) in [
        (json!({ "step": "a" }), "`step` cannot be a label: measurements already say that of themselves"),
        (json!({ "Which One": "a" }), "`Which One` cannot be a label's name"),
        (json!({ "variant": ["a", "b"] }), "the label `variant` must be text or a number"),
        (json!("variant-a"), "`labels` must be a mapping from a label's name to its value"),
    ] {
        let (status, answer) = rig.post("/api/apply", lane(labels)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        let message = answer["error"]["message"].as_str().unwrap();
        assert!(message.contains(why), "{message}");
    }
}

/// The example that gives one piece of work to two agents.
const COMPARED: &str = include_str!("../../../../examples/scenarios/compare-agents.bp");

#[tokio::test]
async fn the_example_that_compares_two_agents_has_each_work_a_card_of_its_own_and_measures_the_two_apart() {
    let (rig, _at) = crate::fleets::rig().await;
    let dir = crate::fleets::shop(&rig, json!({ "workers": 2, "reviewers": 0 })).await;
    let params = json!({ "project": "shop", "title": "Add a cart", "agent_a": "shop-worker-1", "agent_b": "shop-worker-2" });
    let applied = rig
        .send("/api/apply", json!({ "project": "shop", "source": COMPARED, "params": params, "wait": true }))
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let outputs = &applied["finished"]["outputs"];
    let (a, b) = (outputs["a"].as_str().unwrap().to_string(), outputs["b"].as_str().unwrap().to_string());
    crate::lanes::moved(&rig, &a, "review", 0).await;
    crate::lanes::moved(&rig, &b, "review", 0).await;

    // Each was worked by the agent it was given to, once, on a branch of its own.
    for (card, agent, other) in [(&a, "shop-worker-1", "B"), (&b, "shop-worker-2", "A")] {
        let asked = crate::fleets::given(&rig, agent).await;
        assert_eq!(asked.len(), 1, "{agent}: {asked:?}");
        assert!(
            asked[0].contains(&format!("Card {card}: Add a cart"))
                && !asked[0].contains(&format!("({other}:"))
        );
        assert_eq!(crate::lanes::git(&dir, &["log", "--format=%s", "-1", &format!("lane/{card}")]), {
            let title =
                if other == "B" { "Add a cart (A: shop-worker-1)" } else { "Add a cart (B: shop-worker-2)" };
            format!("{card}: {title}")
        });
    }
    // And what was measured under each carries which of the two it was.
    let measured = rig.read("/api/metrics?name=agent.turn&by=variant").await;
    let counted = |variant: &str| {
        measured["series"]
            .as_array()
            .unwrap()
            .iter()
            .find(|series| series["labels"]["variant"] == variant)
            .map(|series| series["total"]["count"].clone())
    };
    assert_eq!((counted("a"), counted("b")), (Some(json!(1)), Some(json!(1))), "{measured}");
}
