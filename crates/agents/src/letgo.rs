//! Letting go of what was asked before a restart, by something that is
//! over without having asked for it again (`agent.let_go`).
//!
//! A turn an agent had in hand when the daemon stopped, or had waiting in
//! its queue, is begun as it starts, before anybody asks for it, so that
//! the work goes on; what
//! asked for it is asked for again by whatever started it, and is given
//! that turn ([`Agents::claim`]). Something that keeps nothing as it goes
//! may come to an end this time without asking: a card's round begun
//! again finds that somebody moved the card away meanwhile, or is stopped
//! by that move before it has come to its agent. Nothing then waits for
//! the turn, and its agent would work for nobody. So whatever started it,
//! once it is over, lets go of what it asked: each such turn that nobody
//! has asked for since this daemon began is called off, and its program
//! ended ([`Agents::let_go_under`]).

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, Registry};
use serde_json::json;

use crate::Agents;
use crate::actions::{add, keeps_nothing, text};

pub(crate) fn register(registry: &mut Registry, agents: &Agents) {
    add(
        registry,
        agents,
        ActionSpec::new(
            "agent.let_go",
            "Call off what something this started asked of an agent before the daemon last stopped, once it is \
             over without having asked for it again: begun again as the daemon started, nobody waits for it.",
        )
        .example(LET_GO_ASKED)
        .required(
            "under",
            T::String,
            "the key of what this started (`as:` of the step that started it); what that asked, or anything it \
             started, is let go of",
        )
        .capability("agents:write")
        .output("called_off", "the turns called off: none where whatever asked had asked for each again")
        // Stood in, no agent had anything in hand.
        .stands(json!({ "called_off": [] }))
        .idempotent(),
        |agents, cx, args| async move {
            let under = text(&args, "under").ok_or_else(|| ActionError::new("`under` names what this started"))?;
            let asker = asker(&cx, &under).await?;
            let called_off = agents.let_go_under(&asker).await;
            if !called_off.is_empty() {
                cx.log('s', &format!("let go of {}, which nobody asked for again", called_off.join(", ")));
            }
            Ok(json!({ "called_off": called_off }))
        },
    );
}

/// Who `under`, started by this instance, is as a turn it asked for keeps
/// it (`asked_by` of `agent.task`): whoever applied this, the instance
/// above that keeps what it does, and the names under that of those that
/// keep nothing, down to `under`.
async fn asker(cx: &ActionCx, under: &str) -> Result<String, ActionError> {
    let mut names = vec![under.to_string()];
    let mut at = Some(cx.instance().to_string());
    let mut kept = String::new();
    while let Some(id) = at.take() {
        let one = cx.engine().instance(&id).await?;
        if keeps_nothing(&one.source) {
            names.push(one.key);
            at = one.parent;
        } else {
            kept = id;
        }
    }
    names.reverse();
    Ok(format!("{}/{kept}/{}", cx.principal().id, names.join("/")))
}

// What the action is shown by in the Reference and the editor, held to
// checking with no warning (the API suite's `examples.rs`).
const LET_GO_ASKED: &str = "\
blueprint: card-turn
permissions: [agents:write, blueprints:run]

params:
  card: { type: string }

steps:
  turn:
    call: std/turn
    as: \"turn-${params.card}\"
    allow_failure: true
    with: { project: shop, item: \"${params.card}\", role: worker, asking: first }

  let_go:
    description: What was begun again for the card as the daemon started, and not asked for again, is called off.
    do: agent.let_go
    with: { under: \"turn-${params.card}\" }
";
