//! A step that starts another blueprint: `call` and `spawn`.
//!
//! Both make a child of the instance the step runs in. A child is found
//! again by its key under that parent: what the step says with `as:`, or
//! the step's own name. So the same step run again finds the child it made
//! and does not make another.
//!
//! In a `for_each` there is a child for each element. A step that says
//! nothing has them named for it, `<step>#<position>`. One that says `as:`
//! has said what each is called, and that is its key, with nothing added:
//! `as: "card-${ item.id }"` is `card-shop-3` wherever in the list that
//! card comes. Two elements given one name would be one child applied over
//! twice, so that is refused.
//!
//! The two differ in what they ask ([`crate::update`]). A `spawn` says the
//! child should be there, and is an apply of it like anyone's: one that is
//! there already takes what is now asked by the one rule. A `call` asks
//! for the child's steps to be run and waits for what they give: they are
//! run, unless the child is already running that very text with those very
//! params.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use blueprint::{Blueprint, Step, StepKind, template};
use journal::EventQuery;
use kernel::{Error, Fields, Result};
use runs::{InstanceInfo, InstanceStatus, RunStatus};
use serde_json::{Value, json};

use crate::action::{ActionError, Element};
use crate::comparing::Start;
use crate::engine::{Apply, Engine, Live};
use crate::labels;
use crate::run::RunCx;
use crate::sink::Sink;
use crate::step::{Done, Each, permit};
use crate::update::Asker;

/// Children of one instance that an earlier fault misnamed, by the name
/// each should have had: `(its id, the blueprint it is of)`.
///
/// A `for_each` step with `as:` used to have the element's position added
/// to the name all the same (`card-shop-3#0`). The child a handler started
/// for the same thing was called what `as:` says, so the two never met and
/// there were two instances for one card. Looked up once for a step that
/// names its children, so that each such child can be put right as its
/// element comes round ([`start`]): it takes its rightful name when
/// nothing has it, and is stopped when something does.
pub(crate) type Misnamed = HashMap<String, Vec<(String, String)>>;

/// What the step's children are to be looked through for before its
/// elements are gone over: nothing, unless it names them.
pub(crate) async fn misnamed(cx: &RunCx, step: &Step) -> Misnamed {
    let names = step.key.is_some() && matches!(step.kind, StepKind::Call(_) | StepKind::Spawn(_));
    if !names {
        return Misnamed::new();
    }
    cx.engine.inner.store.misnamed_children(&cx.live.id).await.unwrap_or_default()
}

/// Puts right the children an earlier fault called `<key>#<position>`.
async fn put_right(cx: &RunCx, blueprint: &str, key: &str, each: &Each<'_>, sink: &Sink) {
    let Some(found) = each.misnamed.get(key) else { return };
    let store = &cx.engine.inner.store;
    let project = cx.live.meta().project;
    let mut taken =
        matches!(store.live_instance_by_key(Some(&cx.live.id), key, project.as_deref()).await, Ok(Some(_)));
    for (id, of) in found {
        if of != blueprint {
            continue;
        }
        if !taken {
            let now = cx.engine.own().now_ms();
            if store.rename_instance(id, key, now).await.is_ok() {
                if let Some(live) = cx.engine.live(id) {
                    live.meta.lock().expect("live lock").key = key.to_string();
                }
                sink.line(
                    'o',
                    &format!("`{key}` was running under a name with its position added; it is `{key}` again"),
                );
                taken = true;
            }
            continue;
        }
        if cx.engine.stop(id).await.is_ok() {
            sink.line(
                'o',
                &format!(
                    "a second instance for `{key}`, started under a name with its position added, was stopped"
                ),
            );
        }
    }
}

/// What the child of a `call` or `spawn` step that says `as:` is called
/// for the element in `scope`; nothing for any other step, or where `as:`
/// cannot be worked out.
pub(crate) fn named(step: &Step, scope: &Fields) -> Option<String> {
    let key = step.key.as_ref().filter(|_| matches!(step.kind, StepKind::Call(_) | StepKind::Spawn(_)))?;
    let rendered = template::render(&Value::String(key.clone()), scope).ok()?;
    rendered.as_str().map(str::to_string)
}

/// What a step asks to be started, and what it is told as it is.
pub(crate) struct Asked<'a> {
    /// The blueprint, as the step names it.
    pub name: &'a str,
    /// What `with` came to: the child's params.
    pub args: Fields,
    pub each: Option<Each<'a>>,
    pub sink: &'a Sink,
    /// Where the child is put while a `call` waits for it, so that a call
    /// that is given up on can stop it.
    pub child: &'a Mutex<Option<String>>,
    /// What the step had saved when the conductor stopped, if it was under
    /// way then: for a `call`, where the instances stood before it asked.
    pub saved: Option<Value>,
    /// The step was under way when the conductor stopped, and is being
    /// done again.
    pub again: bool,
}

/// What a call gives of a run of its child that has ended.
fn given(run: runs::RunInfo, instance: &str) -> std::result::Result<Done, ActionError> {
    match run.status {
        RunStatus::Ok => {
            let mut output = run.outputs.as_object().cloned().unwrap_or_default();
            output.insert("instance".into(), instance.into());
            Ok(Done::Out(Value::Object(output)))
        }
        _ => Err(ActionError::new(run.error.unwrap_or_else(|| "the called blueprint did not finish".into()))),
    }
}

/// What the child this call made gives, where the call was under way when
/// the conductor stopped.
///
/// A call that is picked up again is not asking anew: it asked before the
/// conductor stopped, and its child is the one that was made then. Asking
/// again by name would find the child only while it is running, and one
/// that had ended, or that ends in the moment between looking and asking,
/// would be made a second time and its steps run again. So before a call
/// asks, it writes down how many instances there had ever been (`after`);
/// the child it then makes comes after that, under its name, whatever else
/// had or had not been written down, and is found here by that, running or
/// not. Its run is waited for and the call is given what it gives, or
/// fails as it fails. Nothing where the call had made no child yet, or
/// where the child was stopped before its steps ended: then it asks, as a
/// call does.
async fn had(
    cx: &RunCx,
    key: &str,
    after: i64,
    waiting_on: &Mutex<Option<String>>,
    asking: &kernel::asking::Asking,
) -> std::result::Result<Option<Done>, ActionError> {
    let store = &cx.engine.inner.store;
    let Some(child) = store.child_since(&cx.live.id, key, after).await? else { return Ok(None) };
    let mut run = match store.last_main_run(&child.id).await? {
        Some(run) => run,
        // Its steps never ran: it is begun now, or had failed.
        None => match never_ran(cx, &child).await? {
            Some(run) => match store.run(&run).await? {
                Some(run) => run,
                None => return Ok(None),
            },
            None => return Ok(None),
        },
    };
    // Begun by the conductor's start, which asked nothing: kept as asked for here.
    store.asked_for(&run.id, &child.id, asking.clone()).await?;
    if run.status == RunStatus::Running {
        *waiting_on.lock().expect("step lock") = Some(child.id.clone());
        run = cx.engine.wait_run(&run.id).await?;
        waiting_on.lock().expect("step lock").take();
    }
    match run.status {
        RunStatus::Ok | RunStatus::Failed => given(run, &child.id).map(Some),
        _ => Ok(None),
    }
}

/// What becomes of the child a step made before the conductor stopped,
/// where its steps never ran: the run of them, now under way; or nothing,
/// where it was stopped before it was begun, and is asked for as anything
/// is.
///
/// A conductor stopped between a child being written down and its first
/// run leaves one that is owed its beginning. It is begun here, in its
/// own turn, by the one way that is done ([`Engine::begin_owed`]), unless
/// the conductor's start came to it first: then the run that began is
/// found, or the child had failed. One that could not be begun has failed
/// and said so, before the conductor stopped or as it started, and the
/// step fails as it did then, with what it was answered. Asked for again
/// by name, it would no longer be found, and another would be made, fail,
/// and say so a second time.
async fn never_ran(cx: &RunCx, child: &InstanceInfo) -> std::result::Result<Option<String>, ActionError> {
    let failed = |child: &InstanceInfo| {
        ActionError::new(crate::unbegun::why(child.error.as_deref().unwrap_or_default()))
    };
    match child.status {
        InstanceStatus::Failed => Err(failed(child)),
        InstanceStatus::Starting => {
            if let Some(live) = cx.engine.live(&child.id) {
                let begun = {
                    let _turn = live.apply_lock.lock().await;
                    cx.engine.begin_owed(&live, "step").await?
                };
                if begun.is_some() {
                    return Ok(begun);
                }
            }
            // No longer starting, or no longer in hand: what the start
            // made of it since it was read. Where it began its steps and
            // they have ended by now, that run is what the step was
            // owed; asked for again by name, they were run a second time.
            let store = &cx.engine.inner.store;
            match store.instance(&child.id).await? {
                Some(now) if now.status == InstanceStatus::Failed => Err(failed(&now)),
                _ => Ok(store.last_main_run(&child.id).await?.map(|run| run.id)),
            }
        }
        _ => Ok(None),
    }
}

/// The child a `spawn` step made before the conductor stopped, where the
/// step is being done again.
///
/// A `spawn` is over as soon as its child is there, so a step found under
/// way had not yet been answered: its child was not written down, or was
/// and had not been begun, or could not be begun; or was begun since by
/// the conductor's start, whose steps may have ended by now, so that
/// nothing of it is alive under the name. It is this step's where it was
/// said to have started after the step began; where the journal no longer
/// says when that was, nothing is known and the step asks, as it always
/// did. Asked for by name where the child had ended, another was made.
async fn made_here(cx: &RunCx, step: &Step, key: &str, element: Option<&Element>) -> Option<InstanceInfo> {
    let store = &cx.engine.inner.store;
    let last = store.last_instance_by_key(&cx.live.id, key).await.ok()??;
    let began = match element {
        Some(element) => store.element_began(&cx.run_id, &step.id, element.at).await.ok()??,
        None => crate::intrinsic::began(cx, &step.id).await?,
    };
    // Said after the step began, or it is not this step's: looked for
    // from there.
    let started = EventQuery {
        instance: Some(last.id.clone()),
        topics: "instance.started".into(),
        after: began,
        limit: 1,
        ..Default::default()
    };
    let said = cx.engine.own().read(started).await.ok()?.into_iter().next()?;
    (said.seq > began).then_some(last)
}

/// Starts the blueprint a `call` or `spawn` step names, as a child of the
/// instance the step runs in, or applies over the child that is there.
pub(crate) async fn start(
    cx: &RunCx,
    step: &Step,
    scope: &Fields,
    asked: Asked<'_>,
) -> std::result::Result<Done, ActionError> {
    let Asked { name, args, each, sink, child, saved, again } = asked;
    // Where what this step saves is written, when it is one of a loop.
    let element = each.as_ref().map(|each| Element { items: each.items.clone(), at: each.index });
    let name = template::render(&Value::String(name.to_string()), scope)
        .map_err(|err| format!("blueprint name: {err}"))?
        .as_str()
        .unwrap_or_default()
        .to_string();
    let key = match (&step.key, &each) {
        (Some(key), each) => {
            let key = template::render(&Value::String(key.clone()), scope)
                .map_err(|err| format!("`as`: {err}"))?
                .as_str()
                .unwrap_or_default()
                .to_string();
            if let Some(each) = each {
                let mut named = each.named.lock().expect("step lock");
                if named.contains(&key) {
                    return Err(ActionError::new(format!(
                        "`as` came to `{key}` for two elements of `for_each`, which would be one child applied \
                         over twice. Put what tells the elements apart in it (`${{ item }}`, `${{ index }}`), \
                         or leave `as` out and each is named `{}#<position>`",
                        step.id
                    )));
                }
                named.push(key.clone());
            }
            key
        }
        (None, Some(each)) => format!("{}#{}", step.id, each.index),
        (None, None) => step.id.clone(),
    };
    let environment = step.environment.as_ref().map(|named| template::render(&json!(named), scope));
    let environment = environment.transpose().map_err(|err| format!("`environment`: {err}"))?;
    let environment =
        environment.as_ref().and_then(Value::as_str).filter(|named| !named.is_empty()).map(str::to_string);
    permit(&cx.blueprint, &cx.principal, "blueprints:run")?;
    // A text that calls itself, or texts that call each other, are held to
    // how deep a tree may go: the step fails, saying so, before anything
    // is started.
    let chain = cx.engine.chain_above(&cx.live.id).await;
    if let Some(why) = crate::deepest::too_deep(&name, step.kind.verb(), &chain) {
        return Err(ActionError::new(why));
    }
    // Where a comparison says how this name is started, it is started one
    // of its ways, and what is under it is measured with which.
    let mut labels = labels::under(cx, step, scope)?;
    let project = cx.live.meta().project;
    let start = Start {
        project: project.as_deref(),
        name: &name,
        args: &args,
        parent: &cx.live.id,
        key: &key,
        whom: &cx.principal,
    };
    let (name, source, args) = match cx.engine.way_for(&start, &cx.source(Some(&step.id)), &mut labels).await
    {
        Some(way) => (way.blueprint, way.source, way.args),
        None => (name, None, args),
    };
    sink.handed(Some((&name, &key)), &args);
    if step.key.is_some()
        && let Some(each) = &each
    {
        put_right(cx, &name, &key, each, sink).await;
    }
    let spawned = matches!(step.kind, StepKind::Spawn(_));
    // Known to be waited on before it is asked for: one that fails at once
    // says its failure is this step's too. A step with something to do
    // when it fails may be told there to go on, so it too may let it be;
    // said of one that then does not, a failure is told of twice, which
    // is the side to be wrong on.
    let lets = step.allow_failure || !step.on_fail.is_empty() || !step.on_timeout.is_empty();
    let _waiting = (!spawned).then(|| cx.engine.inner.calls.waiting(&cx.live.id, &key, lets));
    // The run a call begins keeps which step asked for it, and by what
    // the event that set this run off named the asking (`kernel::asking`).
    let by = crate::carry::asked_by(&cx.run_id, &step.id);
    let asked = kernel::asking::Asking::named(&cx.event);
    let asking = kernel::asking::Asking { parent: cx.live.id.clone(), key: key.clone(), by, asked };
    if !spawned {
        match saved.as_ref().and_then(|saved| saved.get("after")).and_then(Value::as_i64) {
            // Under way when the conductor stopped: the child it made then
            // is waited for, and not asked for again.
            Some(after) => {
                let had = had(cx, &key, after, child, &asking);
                if let Some(gave) = kernel::asking::within(asking.clone(), had).await? {
                    return Ok(gave);
                }
            }
            // Asking for the first time: where the instances stood is
            // written down first, so that whatever it makes can be told.
            None if !cx.kept() => {}
            None => {
                let after = json!({ "after": cx.engine.inner.store.newest_instance().await? });
                let noted = match &element {
                    Some(element) => Element::around(&element.items, Some((element.at, after))),
                    None => after,
                };
                cx.engine.save_checkpoint(&cx.run_id, &step.id, noted).await?;
            }
        }
    }
    // The child it made: begun here where its steps never ran, or begun
    // by the conductor's start, whose run is there whether or not it has
    // ended.
    if spawned
        && again
        && let Some(made) = made_here(cx, step, &key, element.as_ref()).await
        && (never_ran(cx, &made).await?.is_some()
            || cx.engine.inner.store.last_main_run(&made.id).await?.is_some())
    {
        return Ok(Done::Out(json!({ "instance": made.id })));
    }
    // A text this step asked its child to be carried over to, answered
    // before the conductor stopped: the run that was to carry it over
    // says so, once the text was taken, however that ended. Asked again,
    // one that failed as it was taken was carried over and failed a
    // second time. One that still waits is asked for again, and answers
    // itself.
    let by = crate::carry::asked_by(&cx.run_id, &step.id);
    if spawned
        && again
        && let Some(made) = cx.engine.inner.store.carried_for(&cx.live.id, &key, &by).await?
    {
        return Ok(Done::Out(json!({ "instance": made })));
    }
    let asker = if spawned {
        // What its run is handling, if it is an event's: a child that takes
        // a text because of this hears that event in what the text adds.
        Asker::Spawn { during: cx.event.get("seq").and_then(Value::as_u64).filter(|seq| *seq > 0) }
    } else {
        Asker::Call
    };
    let applied = cx.engine.apply_from(
        Apply {
            blueprint: name,
            source,
            params: args,
            key: Some(key),
            parent: Some(cx.live.id.clone()),
            target: None,
            project,
            // A child is its parent's to start: it acts for whoever
            // applied the text this step is written in.
            principal: cx.principal.clone(),
            force: false,
            take: false,
            labels,
            // Under its parent's, or the one the step names where its
            // parent is under none (`environments::held::chosen`).
            environment,
        },
        Some((cx.source(Some(&step.id)), asker)),
        None,
        None,
    );
    let applied = match spawned {
        true => applied.await?,
        false => kernel::asking::within(asking, applied).await?,
    };
    if spawned {
        return Ok(Done::Out(json!({ "instance": applied.instance })));
    }
    // What a call that is cut short takes down with it is the child it
    // made. One that was there before the call is not the call's: it is
    // left as it is, and the steps the call asked for go on, as they do
    // when whoever applied something goes away. Stopped with the call,
    // everything such a child had under it went too: a fleet applied
    // over while a changed row was being taken, by a handler in its call
    // of what arranges the fleet, lost every stage, pool and role it had.
    if applied.created {
        *child.lock().expect("step lock") = Some(applied.instance.clone());
    }
    let run = match &applied.run {
        Some(run) => cx.engine.wait_run(run).await?,
        // Nothing changed, so nothing ran: report what it last produced.
        None => match cx.engine.inner.store.last_main_run(&applied.instance).await? {
            Some(run) => run,
            None => return Ok(Done::Out(json!({ "instance": applied.instance }))),
        },
    };
    child.lock().expect("step lock").take();
    given(run, &applied.instance)
}

impl Engine {
    /// The run a call of a step of `parent`'s began, where the run that
    /// asked was set off by an event that says `asked` (`runs.asked_by`).
    pub async fn begun_for(&self, parent: &str, asked: &str) -> Result<Option<String>> {
        self.keeper(parent).await.inner.store.begun_for(parent, asked).await
    }

    /// Brings up the blueprints attached with `uses`, as children that watch
    /// this instance.
    pub(crate) async fn start_attachments(&self, live: &Arc<Live>, blueprint: &Blueprint) -> Result<()> {
        self.attach(live, blueprint, false).await
    }

    /// The same as the conductor starts, for an instance it stopped before
    /// the first run of: what it attaches may not have been brought up, so
    /// what is not there is. One that is there is taken up in its own
    /// right, and asked for again would have its steps run twice.
    pub(crate) async fn start_missing_attachments(&self, live: &Arc<Live>) -> Result<()> {
        self.attach(live, &live.blueprint(), true).await
    }

    async fn attach(&self, live: &Arc<Live>, blueprint: &Blueprint, missing: bool) -> Result<()> {
        let meta = live.meta();
        for attachment in &blueprint.uses {
            let store = &self.inner.store;
            if missing && store.live_instance_by_key(Some(&live.id), &attachment.key, None).await?.is_some() {
                continue;
            }
            let scope = self.base_scope(live);
            let with = template::render(&attachment.with, &scope).map_err(Error::invalid)?;
            // Asked by nobody: the text attached is one this instance may run.
            let attached = Apply {
                blueprint: attachment.blueprint.clone(),
                source: None,
                params: with.as_object().cloned().unwrap_or_default(),
                key: Some(attachment.key.clone()),
                parent: Some(live.id.clone()),
                target: Some(live.id.clone()),
                project: meta.project.clone(),
                principal: meta.principal.clone(),
                force: false,
                take: false,
                labels: meta.labels.clone(),
                environment: None,
            };
            self.apply_within(attached, &live.id)
                .await
                .map_err(|err| err.context(format!("attaching `{}`", attachment.key)))?;
        }
        Ok(())
    }
}
