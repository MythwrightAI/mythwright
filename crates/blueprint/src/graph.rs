//! A blueprint as a picture: nodes, edges and groups.
//!
//! This is the one projection every view of a blueprint is drawn from, so the
//! node canvas, the run timeline and `--graph` on the command line can never
//! disagree about what the graph is. Each node carries the lines it came from,
//! which is how clicking a node finds its text and how a diagnostic finds its
//! node.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::duration;
use crate::model::*;

#[derive(Clone, Debug, Serialize)]
pub struct Graph {
    pub name: String,
    pub description: Option<String>,
    pub params: Vec<GraphParam>,
    pub state: serde_json::Map<String, Value>,
    pub permissions: Option<Vec<String>>,
    pub groups: Vec<Group>,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphParam {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: &'static str,
    pub default: Option<Value>,
    pub required: bool,
    pub description: Option<String>,
    /// The values it may have, when the blueprint says which; otherwise none.
    pub choices: Vec<Value>,
}

/// A cluster of nodes: the main flow, one per handler, and the attachments.
#[derive(Clone, Debug, Serialize)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphNode {
    /// Unique in the graph: `<group>/<name>`.
    pub id: String,
    pub group: String,
    /// The name as written in the file. A step inside a task has the task's
    /// name before its own, `build.compile`, which is what it is called in
    /// a run, so that how a run went can be drawn on the graph by name.
    pub name: String,
    /// `action`, `call`, `spawn`, `task`, `trigger` or `attachment`.
    pub kind: &'static str,
    /// The action or blueprint this node runs, the trigger's source, or
    /// for a task the names of the steps inside it.
    pub target: String,
    /// Distance from the start of its group; the node view's column.
    pub layer: u32,
    pub line: u32,
    pub end_line: u32,
    pub description: Option<String>,
    pub when: Option<String>,
    pub for_each: Option<String>,
    pub timeout: Option<String>,
    pub retry: Option<String>,
    pub device: Option<String>,
    pub measure: Option<String>,
    pub allow_failure: bool,
    pub with: Value,
    pub hooks: Vec<GraphHook>,
    pub watch: Vec<GraphWatch>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphHook {
    /// `on_start`, `on_success`, `on_fail`, `on_timeout` or `on_finish`.
    pub point: &'static str,
    pub action: String,
    pub with: Value,
    pub when: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphWatch {
    pub rule: String,
    pub action: String,
    pub with: Value,
    pub fail: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    /// `needs` (written in the file), `then` (file order), `fires` (from a
    /// trigger to the first steps of its handler) or `starts` (from a task
    /// to the first steps inside it).
    pub kind: &'static str,
}

pub fn graph(blueprint: &Blueprint) -> Graph {
    let mut graph = Graph {
        name: blueprint.name.clone(),
        description: blueprint.description.clone(),
        params: blueprint
            .params
            .iter()
            .map(|param| GraphParam {
                name: param.name.clone(),
                ty: param.ty.name(),
                default: param.default.clone(),
                required: param.required,
                description: param.description.clone(),
                choices: param.choices.clone(),
            })
            .collect(),
        state: blueprint.state.clone(),
        permissions: blueprint.permissions.clone(),
        groups: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    if !blueprint.uses.is_empty() {
        graph.groups.push(Group { id: "uses".into(), label: "attached".into(), kind: "uses" });
        for attachment in &blueprint.uses {
            graph.nodes.push(GraphNode {
                id: format!("uses/{}", attachment.key),
                group: "uses".into(),
                name: attachment.key.clone(),
                kind: "attachment",
                target: attachment.blueprint.clone(),
                layer: 0,
                line: attachment.span.start.line,
                end_line: attachment.span.end.line,
                with: attachment.with.clone(),
                ..blank_node()
            });
        }
    }

    if !blueprint.flow.steps.is_empty() {
        graph.groups.push(Group { id: "main".into(), label: "steps".into(), kind: "main" });
        add_flow(&mut graph, "main", "", &blueprint.flow, 0, None);
    }

    for trigger in &blueprint.triggers {
        let group = format!("on.{}", trigger.name);
        graph.groups.push(Group {
            id: group.clone(),
            label: format!("on {}", trigger.name),
            kind: "handler",
        });
        let (target, with) = match &trigger.source {
            TriggerSource::Event(pattern) => (format!("event {pattern}"), Value::Null),
            TriggerSource::Every(ms) => (format!("every {}", duration::format(*ms)), Value::Null),
            TriggerSource::At(at) => (format!("at {at}"), Value::Null),
            TriggerSource::Poll { every_ms, action, with } => {
                (format!("poll {action} every {}", duration::format(*every_ms)), with.clone())
            }
        };
        let trigger_id = format!("{group}/@");
        graph.nodes.push(GraphNode {
            id: trigger_id.clone(),
            group: group.clone(),
            name: trigger.name.clone(),
            kind: "trigger",
            target,
            layer: 0,
            line: trigger.span.start.line,
            end_line: trigger.span.start.line,
            when: trigger.when.as_ref().map(|cond| cond.source.clone()),
            with,
            ..blank_node()
        });
        add_flow(&mut graph, &group, "", &trigger.flow, 1, Some((&trigger_id, "fires")));
    }
    graph
}

fn blank_node() -> GraphNode {
    GraphNode {
        id: String::new(),
        group: String::new(),
        name: String::new(),
        kind: "action",
        target: String::new(),
        layer: 0,
        line: 0,
        end_line: 0,
        description: None,
        when: None,
        for_each: None,
        timeout: None,
        retry: None,
        device: None,
        measure: None,
        allow_failure: false,
        with: Value::Null,
        hooks: Vec::new(),
        watch: Vec::new(),
    }
}

/// Adds the steps of `flow` to `group`. `prefix` is what goes before their
/// names (the tasks they are inside), and `entry` is the node that sets the
/// first of them going, with the kind of edge that says so.
fn add_flow(
    graph: &mut Graph,
    group: &str,
    prefix: &str,
    flow: &Flow,
    first_layer: u32,
    entry: Option<(&str, &'static str)>,
) {
    let layers = layers(flow);
    for step in &flow.steps {
        let name = within::name(prefix, &step.id);
        let id = format!("{group}/{name}");
        let layer = first_layer + layers.get(step.id.as_str()).copied().unwrap_or(0);
        let hooks = [
            ("on_start", &step.on_start),
            ("on_success", &step.on_success),
            ("on_fail", &step.on_fail),
            ("on_timeout", &step.on_timeout),
            ("on_finish", &step.on_finish),
        ]
        .into_iter()
        .flat_map(|(point, hooks)| {
            hooks.iter().map(move |hook| GraphHook {
                point,
                action: hook.action.clone(),
                with: hook.with.clone(),
                when: hook.when.as_ref().map(|cond| cond.source.clone()),
            })
        })
        .collect();
        let watch = step
            .watch
            .iter()
            .map(|watch| GraphWatch {
                rule: match &watch.cond {
                    WatchCond::After(timing) => format!("after {}", said(timing)),
                    WatchCond::Every(timing) => format!("every {}", said(timing)),
                    WatchCond::Log(pattern) => format!("log matches {pattern}"),
                },
                action: watch.hook.action.clone(),
                with: watch.hook.with.clone(),
                fail: watch.fail,
            })
            .collect();
        graph.nodes.push(GraphNode {
            id: id.clone(),
            group: group.to_string(),
            name: name.clone(),
            kind: match step.kind {
                StepKind::Do(_) => "action",
                StepKind::Call(_) => "call",
                StepKind::Spawn(_) => "spawn",
                StepKind::Task => "task",
            },
            target: match step.kind {
                StepKind::Task => {
                    step.inner.steps.iter().map(|inner| inner.id.as_str()).collect::<Vec<_>>().join(", ")
                }
                _ => step.kind.target().to_string(),
            },
            layer,
            line: step.span.start.line,
            end_line: step.span.end.line,
            description: step.description.clone(),
            when: step.when.as_ref().map(|cond| cond.source.clone()),
            for_each: step.for_each.as_ref().map(|cond| cond.source.clone()),
            timeout: step.timeout.as_ref().map(|timing| timing.source.clone()),
            retry: step
                .retry
                .as_ref()
                .filter(|retry| retry.max > 0)
                .map(|retry| format!("{} more, {} apart", retry.max, duration::format(retry.backoff_ms))),
            device: step.device.clone(),
            measure: step.measure.clone(),
            allow_failure: step.allow_failure,
            with: step.with.clone(),
            hooks,
            watch,
        });
        for need in &step.needs {
            if flow.step(need).is_some() {
                graph.edges.push(Edge {
                    from: format!("{group}/{}", within::name(prefix, need)),
                    to: id.clone(),
                    kind: if step.needs_implicit { "then" } else { "needs" },
                });
            }
        }
        if let (Some((from, kind)), true) = (entry, step.needs.is_empty()) {
            graph.edges.push(Edge { from: from.to_string(), to: id.clone(), kind });
        }
        // The steps of a task are drawn after it in the same group, under
        // its name, so that a view which knows nothing of tasks still
        // draws each of them and can say how each went.
        if step.kind == StepKind::Task {
            add_flow(graph, group, &within::task(&name), &step.inner, layer + 1, Some((&id, "starts")));
        }
    }
}

/// The longest chain of `needs` leading to each step, counting a task on
/// the way as the columns it takes. A cycle (which the checker reports)
/// leaves the steps on it at whatever depth was reached.
fn layers(flow: &Flow) -> BTreeMap<&str, u32> {
    let widths: BTreeMap<&str, u32> = flow.steps.iter().map(|step| (step.id.as_str(), width(step))).collect();
    let mut layers: BTreeMap<&str, u32> = flow.steps.iter().map(|step| (step.id.as_str(), 0)).collect();
    for _ in 0..flow.steps.len() {
        let mut changed = false;
        for step in &flow.steps {
            let after = step
                .needs
                .iter()
                .filter_map(|need| Some(layers.get(need.as_str())? + widths.get(need.as_str())?))
                .max();
            if let Some(after) = after {
                let entry = layers.get_mut(step.id.as_str()).expect("every step has a layer");
                if *entry < after {
                    *entry = after;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    layers
}

/// How many columns a step takes: one, and for a task those of the steps
/// inside it as well, which are drawn to its right.
fn width(step: &Step) -> u32 {
    let layers = layers(&step.inner);
    1 + step.inner.steps.iter().map(|inner| layers[inner.id.as_str()] + width(inner)).max().unwrap_or(0)
}

/// A time as the graph says it: the shortest spelling of one written out,
/// and one worked out as the step begins as it is written.
fn said(timing: &Timing) -> String {
    timing.fixed.map_or_else(|| timing.source.clone(), duration::format)
}
