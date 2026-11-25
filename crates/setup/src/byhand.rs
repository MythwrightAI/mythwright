//! An agent added by hand, which is an instance of `std/agent` like any
//! other agent the factory starts.
//!
//! What an agent is set up as is a blueprint's to say, so that a project
//! may set its agents up another way by forking it; the routes that add
//! one and change one (`POST /api/agents`, `PATCH /api/agents/{name}`,
//! `POST /api/channels/{id}/agents`, and the commands over them) apply
//! that text keyed by the agent's
//! name, for its project and under its environment, as whoever asked,
//! and answer with the slot as it then is. A text runs as whoever
//! applied it and raises nothing: what is held to whoever asks is still
//! held in the action (`agent.ensure`), and is asked here first only so
//! that a refusal is said in its own words and kind, before anything is
//! applied.
//!
//! **What is not applied.** An agent this factory does not start, which
//! nothing brings up (`agents::external`); one that something else
//! counts and would bring up again (a fleet, by its row), which is said
//! of as it always was and changed in its row; and a change of what an
//! agent added by hand holds, or of its project, while it is stopped or
//! leaving, which is made in its row: applying would bring it in again.
//!
//! **Its project and environment** are those it is applied for and
//! under, and an instance has one of each for its life: an agent moved
//! has its text applied in the other project and the one it had stopped
//! after, which stops nothing of it, since it is held by the new one (an
//! agent goes with the last thing that holds it); one put under another
//! environment of its project has its text stopped and applied again
//! under that one, once nothing is in hand.
//!
//! **Taken up once.** An agent added by hand before agents had a text of
//! their own has none, and a migration noted every agent there was
//! (`slots.owes`): a start gives each that is in and that nothing brings
//! up its text, applied over its slot as it stands so that nothing of it
//! changes, and takes the note off ([`take_up`]). Stopped in between, the
//! next start finds the text it applied and only takes the note off.

use agents::{Agents, Brought, EXTERNAL, Slot, SlotSpec};
use conductor::{Apply, Engine, RunStatus};
use kernel::{Error, Fields, Principal, Result};
use serde_json::{Value, json};

/// The text an agent is set up by.
pub const TEXT: &str = "std/agent";

/// Adds an agent by hand, or says it again: its text applied with what
/// is said over what it says now.
pub async fn add(
    engine: &Engine,
    agents: &Agents,
    mut spec: SlotSpec,
    mut more: Fields,
    who: &Principal,
) -> Result<Slot> {
    taken_in(&mut spec, &mut more)?;
    let name = blueprint::expr::slug(&spec.name);
    let existing = agents.find(&name).await?;
    if spec.adapter.as_deref() == Some(EXTERNAL) || existing.as_ref().is_some_and(Slot::external) {
        return agents.ensure(spec, who).await;
    }
    agents.may_ensure(spec.clone(), who, false).await?;
    let brought = agents.brought_by(&name, engine).await?;
    if let Brought::By(_) = brought {
        return agents.ensure(spec, who).await;
    }
    may_apply(who)?;
    let project = spec.project.clone().or_else(|| existing.as_ref().and_then(|slot| slot.project.clone()));
    // Moved and told nothing of one, it works under none, as a move always leaves it.
    let environment = spec
        .environment
        .clone()
        .or_else(|| {
            existing.as_ref().filter(|slot| slot.project == project).map(|slot| slot.environment.clone())
        })
        .unwrap_or_default();
    let mut params = base(&brought, existing.as_ref());
    params.extend(said(&spec));
    params.extend(more);
    match &brought {
        // Moved: its text applied where it now is first, which holds it,
        // and the one it had stopped after, which then stops nothing of
        // it: it keeps its credential and whatever it was at.
        Brought::Own(own) if own.project != project => {
            let slot = agents.slot(&name).await?;
            agents.may_move(&slot, project.as_ref(), who, &brought).await?;
            applied(engine, &name, params, project, environment, who).await?;
            engine.stop(&own.id).await?;
        }
        // Put under another environment of its project, which is its
        // text's for good: stopped, once nothing is in hand, and applied
        // again under that one.
        Brought::Own(own) if own.environment != environment => {
            nothing_in_hand(agents, &name).await?;
            engine.stop(&own.id).await?;
            applied(engine, &name, params, project, environment, who).await?;
        }
        _ => applied(engine, &name, params, project, environment, who).await?,
    }
    agents.slot(&name).await
}

/// Changes what an agent holds: by applying its text again where it was
/// added by hand and is in, and in its row otherwise.
pub async fn give(
    engine: &Engine,
    agents: &Agents,
    name: &str,
    scopes: Vec<String>,
    who: &Principal,
) -> Result<Slot> {
    let name = blueprint::expr::slug(name);
    let (slot, own) = match in_by_hand(engine, agents, &name).await? {
        Some(found) => found,
        None => return agents.give(&name, scopes, who).await,
    };
    let spec = SlotSpec { name: name.clone(), scopes: Some(scopes.clone()), ..SlotSpec::default() };
    agents.may_ensure(spec, who, true).await?;
    may_apply(who)?;
    let mut params = base(&Brought::Own(own.clone()), Some(&slot));
    params.insert("holds".into(), json!(scopes));
    applied(engine, &name, params, own.project.clone(), own.environment.clone(), who).await?;
    agents.slot(&name).await
}

/// Moves an agent to another project, or to none: its text applied there
/// where it was added by hand and is in, under no environment as a move
/// always leaves it, and the one it had stopped after, which then stops
/// nothing of it; in its row otherwise.
pub async fn move_to(
    engine: &Engine,
    agents: &Agents,
    name: &str,
    project: Option<String>,
    who: &Principal,
) -> Result<Slot> {
    let name = blueprint::expr::slug(name);
    let (slot, own) = match in_by_hand(engine, agents, &name).await? {
        Some(found) => found,
        None => return agents.move_to(&name, project, who, engine).await,
    };
    let brought = Brought::Own(own.clone());
    if !agents.may_move(&slot, project.as_ref(), who, &brought).await? {
        return Ok(slot);
    }
    may_apply(who)?;
    let params = base(&brought, Some(&slot));
    applied(engine, &name, params, project, String::new(), who).await?;
    engine.stop(&own.id).await?;
    agents.slot(&name).await
}

/// Gives each agent a migration noted its text, once ([`agents::Agents::owed_texts`]):
/// one that is in, is not leaving and that nothing running brings up, over
/// its slot as it stands. Asked as a node starts, once its conductors
/// are up; what cannot be applied is left as it was, and the note is
/// taken off all the same.
pub async fn take_up(engine: &Engine, agents: &Agents) {
    let Ok(owed) = agents.owed_texts().await else { return };
    if owed.is_empty() {
        return;
    }
    let Ok(brought) = agents.bringing(engine).await else { return };
    for name in owed {
        let slot = agents.find(&name).await.ok().flatten();
        if let Some(slot) = slot.filter(|slot| !slot.external() && slot.active && !slot.leaving)
            && !brought.contains_key(&name)
        {
            let params = base(&Brought::Nobody, Some(&slot));
            let request =
                request(&name, params, slot.project.clone(), slot.environment.clone(), &Principal::system());
            // Not waited for: a conductor goes on with it as with anything
            // applied, a start that comes between included.
            if engine.apply(Apply { force: false, ..request }).await.is_err() {
                // Left as it was, with no text: brought in again by hand, it has one.
            }
        }
        let _ = agents.owes_no_text(&name).await;
    }
}

/// The slot `name` where it was added by hand and is in, with its own text.
async fn in_by_hand(
    engine: &Engine,
    agents: &Agents,
    name: &str,
) -> Result<Option<(Slot, conductor::InstanceInfo)>> {
    let Brought::Own(own) = agents.brought_by(name, engine).await? else { return Ok(None) };
    let slot = agents.slot(name).await?;
    Ok((slot.active && !slot.leaving).then_some((slot, own)))
}

/// What the text is applied with before anything is said: what its own
/// instance was applied with (a fork's own params among it), with the
/// slot as it stands over that, since the slot may have been changed in
/// its row while it was stopped.
fn base(brought: &Brought, slot: Option<&Slot>) -> Fields {
    let mut params = match brought {
        Brought::Own(own) => own.params.clone(),
        _ => Fields::new(),
    };
    // Whether it leaves, and a channel it was brought into once, are said
    // each time and are not what it is: it may have left that channel since.
    params.remove("leave");
    params.remove("joins");
    if let Some(slot) = slot {
        params.extend(params_of(slot));
    }
    params
}

/// A slot as it stands, as the text's params: applied with these, it is
/// as it was.
pub fn params_of(slot: &Slot) -> Fields {
    let on = |yes: bool| if yes { "on" } else { "off" };
    let secrets: Vec<String> =
        slot.secrets.iter().map(|(variable, name)| format!("{variable}={name}")).collect();
    let mut params = Fields::new();
    for (name, value) in [
        ("name", json!(slot.name)),
        ("role", json!(slot.role)),
        ("rank", json!(slot.rank)),
        ("instructions", json!(slot.instructions)),
        ("adapter", json!(slot.adapter)),
        ("model", json!(slot.model.clone().unwrap_or_default())),
        ("permission", json!(slot.permission_mode)),
        ("holds", json!(slot.scopes)),
        ("wall", json!(on(slot.wall))),
        ("reads", json!(slot.reads)),
        ("changes", json!(slot.changes)),
        ("git", json!(slot.git)),
        ("secrets", json!(secrets)),
        ("signins", json!(slot.signins)),
        ("outlives", json!(on(slot.outlives))),
        ("web", json!(on(slot.web))),
    ] {
        params.insert(name.into(), value);
    }
    params
}

/// What a request said of an agent, as the text's params: what it did not
/// say is left to what it is.
fn said(spec: &SlotSpec) -> Fields {
    let on = |yes: &Option<bool>| yes.map(|yes| json!(if yes { "on" } else { "off" }));
    let mut params = Fields::new();
    params.insert("name".into(), json!(blueprint::expr::slug(&spec.name)));
    for (name, value) in [
        ("role", spec.role.as_ref().map(|said| json!(said))),
        ("rank", spec.rank.map(|said| json!(said))),
        ("instructions", spec.instructions.as_ref().map(|said| json!(said))),
        ("adapter", spec.adapter.as_ref().map(|said| json!(said))),
        ("model", spec.model.as_ref().map(|said| json!(said))),
        ("permission", spec.permission_mode.as_ref().map(|said| json!(said))),
        ("holds", spec.scopes.as_ref().map(|said| json!(said))),
        ("wall", on(&spec.wall)),
        ("reads", spec.reads.as_ref().map(|said| json!(said))),
        ("changes", spec.changes.as_ref().map(|said| json!(said))),
        ("git", spec.git.as_ref().map(|said| json!(said))),
        ("secrets", spec.secrets.as_ref().map(|said| json!(said))),
        ("signins", spec.signins.as_ref().map(|said| json!(said))),
        ("outlives", on(&spec.outlives)),
        ("web", on(&spec.web)),
        ("joins", spec.channel.as_ref().map(|said| json!(said))),
    ] {
        if let Some(value) = value {
            params.insert(name.into(), value);
        }
    }
    params
}

/// What of the text's own params a request said under `params` (as a form
/// made from them says everything) and not by name, taken in as said by
/// name, so that it is asked of whoever asks before anything is applied.
fn taken_in(spec: &mut SlotSpec, more: &mut Fields) -> Result<()> {
    more.remove("name");
    more.remove("leave");
    let text = |more: &mut Fields, name: &str| {
        more.remove(name).and_then(|value| value.as_str().map(str::to_string)).filter(|said| !said.is_empty())
    };
    // What it is told and its model are said as they are, empty too: a form says what it is set up as.
    let said = |more: &mut Fields, name: &str| {
        more.remove(name).and_then(|value| value.as_str().map(str::to_string))
    };
    let list = |more: &mut Fields, name: &str| {
        more.remove(name).and_then(|value| {
            value.as_array().map(|items| items.iter().map(blueprint::expr::display).collect())
        })
    };
    let word = |said: Option<String>| -> Result<Option<bool>> {
        match said.as_deref() {
            None => Ok(None),
            Some("on") => Ok(Some(true)),
            Some("off") => Ok(Some(false)),
            Some(other) => Err(Error::invalid(format!("`{other}` is neither `on` nor `off`"))),
        }
    };
    spec.role =
        spec.role.take().or_else(|| more.remove("role").and_then(|value| value.as_str().map(str::to_string)));
    spec.rank = spec.rank.or_else(|| more.remove("rank").and_then(|value| value.as_i64()));
    spec.instructions = spec.instructions.take().or_else(|| said(more, "instructions"));
    spec.adapter = spec.adapter.take().or_else(|| text(more, "adapter"));
    spec.model = spec.model.take().or_else(|| said(more, "model"));
    spec.permission_mode = spec.permission_mode.take().or_else(|| text(more, "permission"));
    spec.scopes = spec.scopes.take().or_else(|| list(more, "holds"));
    spec.reads = spec.reads.take().or_else(|| list(more, "reads"));
    spec.changes = spec.changes.take().or_else(|| list(more, "changes"));
    spec.secrets = spec.secrets.take().or_else(|| list(more, "secrets"));
    spec.signins = spec.signins.take().or_else(|| list(more, "signins"));
    spec.git = spec.git.take().or_else(|| text(more, "git"));
    spec.channel = spec.channel.take().or_else(|| text(more, "joins"));
    if spec.wall.is_none() {
        spec.wall = word(text(more, "wall"))?;
    }
    if spec.outlives.is_none() {
        spec.outlives = word(text(more, "outlives"))?;
    }
    if spec.web.is_none() {
        spec.web = word(text(more, "web"))?;
    }
    // What was said both ways is said by name: under `params` it is passed over.
    for name in ["role", "rank", "instructions", "adapter", "model", "permission", "holds", "reads"] {
        more.remove(name);
    }
    for name in ["changes", "secrets", "signins", "git", "joins", "wall", "outlives", "web"] {
        more.remove(name);
    }
    Ok(())
}

/// What a change of an agent comes to for the work it has in hand, in a
/// sentence for whoever changed it; nothing for one that was not there.
pub fn came_to(before: Option<&Slot>, after: &Slot) -> Option<String> {
    let before = before?;
    fn program(slot: &Slot) -> impl PartialEq + '_ {
        let words = (&slot.adapter, &slot.model, &slot.permission_mode, &slot.git, &slot.environment);
        (words, slot.wall, &slot.reads, &slot.changes, slot.web, slot.outlives, &slot.signins, &slot.secrets)
    }
    let mut came = Vec::new();
    if !before.active && after.active {
        came.push("it was stopped, and is brought in again".to_string());
    }
    if program(before) != program(after) {
        came.push("its program is started as it is now set up from its next turn, and work it has in hand goes on as it was begun".into());
    }
    if before.instructions != after.instructions {
        came.push("it is told what it is now told from its next turn".into());
    }
    if before.scopes != after.scopes {
        came.push("what it may ask of the factory is the new list from the next thing it asks, in work it has in hand too".into());
    }
    if before.project != after.project {
        let project =
            after.project.as_deref().map_or("no project".to_string(), |project| format!("`{project}`"));
        came.push(format!("it works in {project} from its next turn"));
    }
    let name = &after.name;
    Some(match came.is_empty() {
        true => format!("Nothing of `{name}` changed."),
        false => format!("`{name}` is changed: {}.", came.join("; ")),
    })
}

/// A slot as it is answered, with what its change came to where it was there before.
pub fn answered(before: Option<&Slot>, after: &Slot) -> Value {
    let mut answer = json!(after);
    if let Some(said) = came_to(before, after) {
        answer["said"] = json!(said);
    }
    answer
}

/// Refuses whoever may not apply what an agent is set up by.
fn may_apply(who: &Principal) -> Result<()> {
    if who.can("blueprints:run") {
        return Ok(());
    }
    Err(Error::denied(format!(
        "{} is not allowed to do this (needs `blueprints:run`): an agent is brought in and changed by applying \
         `{TEXT}`, as whoever asks",
        who.name
    )))
}

/// Refuses moving an agent's text while it has work in hand.
async fn nothing_in_hand(agents: &Agents, name: &str) -> Result<()> {
    let in_hand = agents.in_hand(std::iter::once(name)).await?.get(name).copied().unwrap_or(0);
    if in_hand == 0 {
        return Ok(());
    }
    Err(Error::conflict(format!(
        "`{name}` has work in hand, asked under the environment it works under now, and is put under \
         another once that is over: say it again when it has finished (`{cli} agent turns {name}` says what \
         it has), or stop it, which calls that work off (`{cli} agent stop {name}`)",
        cli = brand::CLI
    )))
}

fn request(
    name: &str,
    params: Fields,
    project: Option<String>,
    environment: String,
    who: &Principal,
) -> Apply {
    Apply {
        params,
        key: Some(name.to_string()),
        project,
        // Run again even where nothing changed: said again, one that was
        // stopped is brought in.
        force: true,
        environment: Some(environment),
        ..Apply::new(TEXT, who.clone())
    }
}

/// Applies the text for `name` and waits for its steps: refused in the
/// sentence its steps failed with.
async fn applied(
    engine: &Engine,
    name: &str,
    params: Fields,
    project: Option<String>,
    environment: String,
    who: &Principal,
) -> Result<()> {
    let applied = engine.apply(request(name, params, project, environment, who)).await?;
    let Some(run) = applied.run else { return Ok(()) };
    let finished = engine.wait_run(&run).await?;
    if finished.status == RunStatus::Ok {
        return Ok(());
    }
    let why = finished.error.unwrap_or_else(|| format!("applying `{TEXT}` for `{name}` did not finish well"));
    Err(Error::invalid(why))
}

/// Further params of the text a request hands on as they are (`params`):
/// those a project's own fork of it says.
pub fn more_of(body: &Value) -> Fields {
    body.get("params")
        .and_then(Value::as_object)
        .map(|more| more.clone().into_iter().collect())
        .unwrap_or_default()
}
