//! What blueprints can do with agents: bring one in, give it work, ask it
//! to decide.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Registry};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

use crate::{Agents, Candidate, Order, Slot, SlotSpec, TurnInfo, TurnRequest, dispatch};

pub(crate) fn add<F, Fut>(registry: &mut Registry, agents: &Agents, spec: ActionSpec, run: F)
where
    F: Fn(Agents, ActionCx, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let agents = agents.clone();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(run(agents.clone(), cx, args))
    });
}

/// Whether `folder` is a folder within a project, written from its top.
fn within(folder: &str) -> bool {
    let parts = std::path::Path::new(folder).components();
    !folder.is_empty() && parts.into_iter().all(|part| matches!(part, std::path::Component::Normal(_)))
}

pub(crate) fn text(args: &Fields, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string).filter(|text| !text.is_empty())
}

/// A list that was said, each of its items as text; nothing, for one that
/// was not said at all.
pub(crate) fn listed(args: &Fields, key: &str) -> Option<Vec<String>> {
    args.get(key).and_then(Value::as_array).map(|items| items.iter().map(blueprint::expr::display).collect())
}

/// Who the work is named for, where anyone is: whoever the step names
/// (`slot`), and where it names nobody, whoever the work is given to
/// (`given`), so that a text hands on both and chooses between neither.
/// A record names an agent as `agent:<name>` and a person otherwise
/// (`user:<id>`): a step may hand on what a record says as it is, and one
/// given to a person names no agent.
fn named_in(args: &Fields) -> Option<String> {
    let said = text(args, "slot").or_else(|| text(args, "given"))?;
    match said.split_once(':') {
        Some(("agent", name)) => Some(name.to_string()).filter(|name| !name.is_empty()),
        Some(_) => None,
        None => Some(said),
    }
}

/// Whether the step itself names the agent (`slot`), and not only a
/// record it hands on (`given`): one the step names is meant, of
/// whichever role, where the role a record's agent is of is the stage's
/// to hold it to.
fn named_by_the_step(args: &Fields) -> bool {
    text(args, "slot").is_some()
}

/// The agent to use: the one named, else whichever is available for the
/// instance's project, of the role asked for if one was. `Ok(None)` means
/// none is, and the caller said that is acceptable.
async fn pick(agents: &Agents, cx: &ActionCx, args: &Fields) -> Result<Option<Slot>, ActionError> {
    let optional = args.get("if_available").and_then(Value::as_bool).unwrap_or(false);
    let role = text(args, "role");
    // Said of the step whoever it comes to: a word that names no way of
    // choosing is wrong the first time, and not only once there is a choice.
    let order = Order::named(&text(args, "pick").unwrap_or_default())?;
    // Agents the step says may not take it, whoever names them.
    let not = listed(args, "not").unwrap_or_default();
    // One that is leaving takes nothing new, named or not.
    let named = match named_in(args) {
        Some(name) if not.contains(&name) => {
            return match optional {
                true => Ok(None),
                false => Err(ActionError::new(format!(
                    "`{name}` is named for this, and is one of those that may not take it (`not`)"
                ))),
            };
        }
        Some(name) => {
            let slot = agents.slot(&name).await.ok();
            // Nor does one this factory does not start, ever: what
            // names it is its own to take up and nobody else's, so the
            // work goes to nobody, said by name where that fails the step.
            if let Some(outside) = slot.as_ref().filter(|slot| slot.external()) {
                return match optional {
                    true => Ok(None),
                    false => Err(ActionError::new(crate::external::not_given(&outside.name))),
                };
            }
            Some(slot.filter(|slot| slot.active && !slot.leaving))
        }
        None => None,
    };
    // Named beside a role, it is that one where it is of the role and of
    // this project. One a record names that is of another role is another
    // stage's (whoever planned a card is not whoever works it); one the
    // step itself names is meant whatever its role (a card's A/B test
    // hands its rounds to the winner's agent so). One of another project
    // is nobody's here: the work then goes as if nobody was named.
    let ours = |slot: &Slot| cx.project().is_none() || slot.project.as_deref() == cx.project();
    let meant = named_by_the_step(args);
    let named = match named {
        Some(Some(slot)) if role.is_some() && !ours(&slot) => None,
        Some(Some(slot)) if !meant && role.as_ref().is_some_and(|role| slot.role != *role) => None,
        other => other,
    };
    let found = match named {
        Some(found) => found,
        // No agent named. A conversation is carried on where it was held,
        // so one that is named and has been held is its agent's, while
        // that agent is still of the role asked for; anything else is
        // whichever agent is available.
        None => {
            let holder = match text(args, "thread") {
                Some(thread) => agents.holding(&thread).await?,
                None => None,
            };
            let holder = holder.filter(|slot| !not.contains(&slot.name));
            match holder.filter(|slot| role.as_ref().is_none_or(|role| slot.role == *role)) {
                Some(slot) => Some(slot),
                None => {
                    // Asked with others about one thing, it is not one of
                    // the agents the others are held with.
                    let beside = match (text(args, "apart"), text(args, "thread")) {
                        (Some(prefix), Some(thread)) => agents.beside(&prefix, &thread).await?,
                        _ => Vec::new(),
                    };
                    // Those held apart are passed over while there is anyone
                    // else; those that may not take it are never given it.
                    let mut could = agents.could_take(cx.project(), role.as_deref(), &beside, order).await?;
                    could.retain(|one| !not.contains(&one.slot.name));
                    dispatched(cx, args, role.as_deref(), could).await
                }
            }
        }
    };
    // Work is given by somebody who could do it: an agent that holds more
    // than whoever asks, or that runs commands where they may not, is not
    // a way round what they hold.
    if let Some(slot) = &found {
        let by = cx.principal();
        let beyond = slot.scopes.iter().find(|scope| !by.can(scope)).cloned();
        let beyond =
            beyond.or_else(|| (slot.permission_mode == "full" && !by.can("shell")).then(|| "shell".into()));
        if let Some(beyond) = beyond {
            return Err(ActionError::new(format!(
                "{} cannot give work to `{}`: that agent holds `{beyond}`, and whoever applied this does not",
                by.name, slot.name
            )));
        }
    }
    match found {
        Some(slot) => Ok(Some(slot)),
        None if optional => Ok(None),
        None => Err(ActionError::new(match (named_in(args), role) {
            (None, Some(role)) => format!(
                "no agent in the role `{role}` is active; bring some up with `agent.ensure` and `role: {role}`, \
                 or pass `if_available: true`"
            ),
            (Some(name), _) => format!("there is no active agent called `{name}`"),
            (None, None) => {
                "no agent is available; bring one in with `agent.ensure`, or pass `if_available: true`"
                    .to_string()
            }
        })),
    }
}

/// Whoever of `could` the work goes to: the one the rule for it says
/// (`std/dispatch`, as the step's instance may run it), and where the rule
/// says nobody, or cannot be asked, the first of them, which is who the
/// asker's `pick` puts first. What is wrong with a rule is said in the
/// step's log, and the work is not held up for it.
async fn dispatched(cx: &ActionCx, args: &Fields, role: Option<&str>, could: Vec<Candidate>) -> Option<Slot> {
    let first = could.first()?.slot.clone();
    let rule = cx.engine().text_within(cx.instance(), cx.principal(), dispatch::RULE, cx.project()).await;
    let Some(rule) = rule else { return Some(first) };
    let asked = dispatch::Asked {
        project: cx.project().unwrap_or_default().to_string(),
        role: role.unwrap_or_default().to_string(),
        picks: text(args, "pick").unwrap_or_else(|| "first".to_string()),
        about: args.get("about").cloned().unwrap_or(Value::Null),
        agents: could.iter().map(dispatch::offered).collect(),
    };
    match dispatch::ask(&rule, &asked) {
        Ok(Some(name)) => could.into_iter().find(|one| one.slot.name == name).map(|one| one.slot),
        Ok(None) => Some(first),
        Err(why) => {
            let rule = dispatch::RULE;
            cx.log(
                'e',
                &format!("the rule for who work goes to, `{rule}`, {why}; it went to `{}`", first.name),
            );
            Some(first)
        }
    }
}

/// Stops a turn if the step waiting on it goes away first: cancelled, timed
/// out, or dropped. An agent is never left working on something nobody is
/// waiting for. When the conductor itself is shutting down the turn is left
/// as it is, to be run again by the next one.
struct StopOnDrop {
    agents: Agents,
    engine: conductor::Engine,
    turn: Option<String>,
}

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        if let Some(turn) = self.turn.take()
            && !self.engine.is_halting()
        {
            let agents = self.agents.clone();
            tokio::spawn(async move { agents.cancel(&turn).await });
        }
    }
}

/// Waits for a turn that is in an agent's queue.
pub(crate) async fn turn(agents: &Agents, cx: &ActionCx, id: String) -> Result<TurnInfo, ActionError> {
    let mut guard =
        StopOnDrop { agents: agents.clone(), engine: cx.engine().clone(), turn: Some(id.clone()) };
    let finished = agents.wait(&id).await?;
    guard.turn = None;
    Ok(finished)
}

/// What the argument `thread` is for.
const THREAD: &str = "continue this conversation (default: one per instance). One held in a channel (`channel:<its id>`, or `thread:<the message it hangs from>`) is that channel's: whoever applied this reads the channel (`channels:read`), or there is no such conversation";

/// What the argument `about` is for, said once for both actions that take it.
const ABOUT: &str = "what the work is, for the rule that says who it goes to where nobody is named (`std/dispatch`): whatever that rule reads, such as `{ card: shop-4, title: ..., labels: [...] }`";

const PICK: &str = "with no `slot`, and no conversation held: which it is of those with as little in hand as each \
                    other. `first`, the one brought up first (default); `turn`, each in turn; `least`, the one \
                    given the least so far; `rank`, the one ranked highest";

const APART: &str = "with a `role` and a `thread`: not an agent that another conversation is held with whose name \
                     begins with this, while there is anyone else; so that several asked about one thing (threads \
                     `review:7:1`, `review:7:2`, apart `review:7:`) are each a different agent";

fn report(turn: &TurnInfo) -> Value {
    json!({
        "available": true,
        "slot": turn.slot,
        "reply": turn.reply,
        "decision": turn.decision,
        "cost_usd": turn.cost_usd,
        "refused": turn.refused,
        "duration_ms": turn.finished.unwrap_or(turn.created) - turn.started.unwrap_or(turn.created),
        "again": turn.restarts,
    })
}

/// What the argument `after_restart` is for, and `restarts` with it.
const AFTER_RESTART: &str = "what becomes of the work when the daemon is stopped in the middle of it and started \
                             again: `again`, it goes on (the agent's program is taken up where it is, where it \
                             outlives the daemon, and is otherwise asked again), which is the default; `leave`, \
                             it is not begun again, and this fails saying so, for a person to look";
const RESTARTS: &str = "how many such stops in the middle of this work it goes on after at most (default 3): past \
                        that it is left for a person whatever `after_restart` says";

/// What the argument `asking` is for.
const ASKING: &str = "which asking this is, in a word of the step's own that it would say again for this asking and \
                      for no other (the event that set it off, say: `\"${ string(event.seq) }\"`). Where a restart takes away \
                      a step that keeps nothing while it waits, or after its agent has answered, what is applied by \
                      the same principal under the same key and asks the same again in the same word is given the \
                      turn it asked for then, as it is by now, and the agent is not asked twice. With no word, only \
                      a turn that was still in hand is found";

/// What the argument `seats` is for.
const SEATS: &str = "ask so many agents at once, each for itself, and count what they choose: a council. Each seat is \
                     a conversation of its own (`<thread>:<seat>`), an agent apart from the other seats' while there \
                     are enough, and an asking of its own (`<asking>:<seat>`). It is decided as soon as it can be, \
                     those still looking are then called off, and what it came to is `came`. A seat that could not \
                     be asked, or whose agent could not do it, has chosen neither. With 0, or not said, one agent \
                     is asked, as where there is no council";

/// `agent.task`: an action of its own and not a function, because a step
/// that was waiting for an agent when the conductor stopped has something
/// to go on from, the turn it was waiting for.
struct Task {
    spec: ActionSpec,
    agents: Agents,
}

impl conductor::Action for Task {
    fn spec(&self) -> &ActionSpec {
        &self.spec
    }

    fn run(&self, cx: ActionCx, args: Fields) -> BoxFut<'static, ActionResult> {
        Box::pin(task(self.agents.clone(), cx, args))
    }

    /// The conductor was started again while this waited: the turn it
    /// waited for is in the agent's queue still, taken up where it is or
    /// asked again there, and is waited for as before.
    fn resume(&self, cx: ActionCx, args: Fields, saved: Value) -> BoxFut<'static, ActionResult> {
        let agents = self.agents.clone();
        Box::pin(async move {
            // A council waited for several, and goes on waiting for those.
            if saved["seats"].is_array() {
                return crate::council::resume(agents, cx, args, saved).await;
            }
            let Some(id) = saved["turn"].as_str().map(str::to_string) else {
                return task(agents, cx, args).await;
            };
            // It is this step's, and nobody else's to be given.
            agents.claimed(&id);
            cx.log('s', "went on waiting for the agent after a restart");
            held(&agents, &cx, &args, id).await
        })
    }
}

/// What a step that asked for work is given of the turn it came to.
///
/// Work the built-in assistant was given and could not do is said with
/// the one command that gives that agent a program which can: here the
/// runner can be asked whose the agent is, a fleet's row or nobody's,
/// which the assistant itself could not say.
pub(crate) async fn answered(agents: &Agents, cx: &ActionCx, finished: TurnInfo) -> ActionResult {
    let Some(error) = &finished.error else {
        let mut said = report(&finished);
        said["unhidden"] = json!(agents.unhidden(&finished).await);
        return Ok(said);
    };
    let why = match agents.find(&finished.slot).await {
        Ok(Some(slot)) if error.starts_with(crate::builtin::CANNOT) => {
            crate::builtin::cannot(&slot.name, &agents.whose(&slot, cx.engine()).await)
        }
        _ => error.clone(),
    };
    Err(ActionError::new(format!("{} could not do it: {why}", finished.slot)).with_data(report(&finished)))
}

/// What is said of the step whoever the work comes to: whether work cut
/// off by a stop is begun again, and after how many such stops at most.
pub(crate) fn restarting(cx: &ActionCx, args: &Fields) -> Result<(bool, Option<u32>), ActionError> {
    let again = match text(args, "after_restart").as_deref() {
        None | Some("again") => true,
        Some("leave") => false,
        Some(other) => {
            return Err(ActionError::new(format!(
                "`after_restart` is `again` or `leave`, and not `{other}`"
            )));
        }
    };
    let most = args.get("restarts").and_then(Value::as_u64).map(|most| most as u32);
    // Done again from its start: the daemon stopped before this step had
    // kept which turn it waited for. With a word for which asking it is,
    // the turn it asked for then is found below, if it had got as far as
    // asking. With none it cannot tell whether it had, and fails as a
    // step that cannot be done twice rather than ask a second time.
    if cx.again() && text(args, "asking").is_none_or(|word| word.trim().is_empty()) {
        return Err(ActionError::new(
            "was interrupted by a restart, and this action is not safe to repeat automatically",
        ));
    }
    Ok((again, most))
}

/// What came of asking for one turn: the turn, or that there was nobody
/// to ask and the step said that would do.
pub(crate) enum Asked {
    Turn(String),
    Nobody,
}

/// Chooses who the work goes to and puts it in their queue, or finds the
/// turn this asking had before a restart. Asked by whoever holds
/// `choosing`: two choices made together would both find the same agent
/// with nothing in hand.
pub(crate) async fn ask(
    agents: &Agents,
    cx: &ActionCx,
    args: &Fields,
    (again, most): (bool, Option<u32>),
) -> Result<Asked, ActionError> {
    let Some(slot) = pick(agents, cx, args).await? else { return Ok(Asked::Nobody) };
    // Said now, of the step, and not later as the agent's failure.
    let cwd = text(args, "cwd");
    agents.workdir(&slot, cwd.as_deref(), None).map_err(ActionError::new)?;
    let hidden = listed(args, "hidden").unwrap_or_default();
    if let Some(wrong) = hidden.iter().find(|folder| !within(folder)) {
        return Err(ActionError::new(format!(
            "`hidden` is folders within the project, written from its top, such as `.lanes/shop-1--a`; `{wrong}` \
             is not one"
        )));
    }
    // Where the agent says what it did, in its own name. Whoever
    // asks must be allowed to post there: an agent is not a way to
    // say something in a channel the asker may not speak in.
    let channel = text(args, "channel").filter(|channel| !channel.trim().is_empty());
    if channel.is_some() && !cx.principal().can("channels:write") {
        return Err(ActionError::new(format!(
            "{} may not use `channels:write`, so an agent cannot be asked to say its reply in a channel",
            cx.principal().name
        )));
    }
    let thread = text(args, "thread").unwrap_or_else(|| format!("instance:{}", cx.instance()));
    let reply_to = text(args, "answers").filter(|_| channel.is_some());
    // Said now, of the step, and not when the work is done and there is nowhere to say it.
    let into = match &channel {
        Some(channel) => Some(agents.inner.studio.channels.writable(channel, cx.principal()).await?),
        None => None,
    };
    let (name, slot_up) = (slot.name.clone(), slot.active && !slot.leaving && !slot.external());
    let request = TurnRequest {
        slot: slot.name,
        kind: "task".into(),
        thread,
        prompt: text(args, "prompt").unwrap_or_default(),
        reply_to,
        labels: cx.labels().clone(),
        cwd,
        hidden,
        channel,
        options: listed(args, "options").unwrap_or_default(),
        again: Some(again),
        most,
        // Held to the time the step is given, or the turn (`within`), by
        // whoever finds it at work when no step is there to hold it.
        until: [cx.limit().map(|limit| limit.as_millis() as i64), within_of(args)?]
            .into_iter()
            .flatten()
            .min()
            .map(|limit| cx.clock().now_ms() + limit),
        ..TurnRequest::default()
    };
    // Which asking this is, where the step says, and whose: whoever
    // applied this, under the name its instance is known by where it was
    // started. An instance that keeps nothing is another one when it is
    // begun again, and is known by the same name; and so is whatever
    // such an instance had started. So what started it is named by which
    // instance it is only where that one is kept, and by its own name
    // under what started it where it is not, as far up as that goes.
    // Nobody else's word finds its turn.
    let asking = text(args, "asking").filter(|word| !word.trim().is_empty());
    let asked_by = match &asking {
        Some(_) => {
            let own = cx.engine().instance(cx.instance()).await?;
            let mut names = vec![own.key];
            let mut above = own.parent;
            let mut under = String::new();
            while let Some(id) = above.take() {
                let one = cx.engine().instance(&id).await?;
                if keeps_nothing(&one.source) {
                    names.push(one.key);
                    above = one.parent;
                } else {
                    under = id;
                }
            }
            names.reverse();
            Some(format!("{}/{under}/{}", cx.principal().id, names.join("/")))
        }
        None => None,
    };
    let request = TurnRequest { asking, asked_by, ..request };
    // An agent says something only in a channel it is in: asked to say what it did in an open one,
    // it is brought in by whoever asks, before it works, so that what it says itself is its own there.
    if let Some((reader, found)) = &into {
        let (actor, open) = (format!("agent:{name}"), !found.kept && found.archived.is_none());
        if slot_up && open && !found.members.iter().any(|member| member.actor == actor) {
            let _ = agents.inner.studio.channels.bring_asked(reader, &found.id, &name).await;
        }
    }
    // Asked exactly this before, by something a restart took away, the
    // agent is not asked again: that turn is this step's now, as it is
    // by now.
    let asked = match agents.claim(&request).await {
        Some(id) => {
            cx.log('s', "this was asked of the agent before a restart, and that asking is gone on with");
            id
        }
        None => agents.enqueue_chosen(request).await?,
    };
    Ok(Asked::Turn(asked))
}

async fn task(agents: Agents, cx: ActionCx, args: Fields) -> ActionResult {
    // Said of the step, whoever the work comes to.
    let restarts = restarting(&cx, &args)?;
    // Asked of several agents at once, it is a council; a text that hands
    // its count on whatever it is says none with 0.
    if crate::council::count(&args, "seats").is_some_and(|seats| seats != 0) {
        return crate::council::council(agents, cx, args, restarts).await;
    }
    // A conversation that is a channel's is read by having an agent go
    // on with it: whoever applied this reads that channel, or it is not
    // there.
    if let Some(thread) = text(&args, "thread") {
        agents.may_go_on(&thread, cx.principal()).await?;
    }
    // Who it goes to is chosen and the work put in their queue as
    // one thing: two choices made together would both find the
    // same agent with nothing in hand.
    let chosen = agents.inner.choosing.lock().await;
    let id = match ask(&agents, &cx, &args, restarts).await? {
        Asked::Turn(id) => id,
        Asked::Nobody => {
            // Who was named and could not take it, for whoever says so.
            let named = named_in(&args).unwrap_or_default();
            // And whether that one is an agent this factory does not start:
            // the work is then its own to take up, and nobody was asked.
            let external = agents.find(&named).await?.is_some_and(|slot| slot.external());
            let none = json!({ "available": false, "reply": "", "decision": null, "named": named, "external": external });
            return Ok(none);
        }
    };
    drop(chosen);
    // From here a conductor that is started again goes on waiting for the
    // turn, whatever was said of work in hand then. A turn that was to be
    // left is failed by whoever finds it in hand, which says so; one that
    // had ended has its answer, which is not thrown away to ask a person
    // for what was already given.
    cx.checkpoint(json!({ "turn": id })).await?;
    held(&agents, &cx, &args, id).await
}

/// How long a turn may take (`within`), where it is said.
fn within_of(args: &Fields) -> Result<Option<i64>, ActionError> {
    let Some(said) = text(args, "within").filter(|said| !said.trim().is_empty()) else { return Ok(None) };
    let millis = blueprint::duration::parse(said.trim()).map_err(|_| {
        ActionError::new(format!("`within` is how long the turn may take, as `45m`; `{said}` is no time"))
    })?;
    Ok(Some(millis as i64))
}

/// Waits for a turn held to the time it was given (`within`): past it its
/// program is stopped, and what the step is given says so (`cut`) and
/// fails nothing, so that whoever asked may ask the agent what it had
/// done. The time is the turn's own, kept with it, so a step taken up
/// after a restart holds it to the same.
async fn held(agents: &Agents, cx: &ActionCx, args: &Fields, id: String) -> ActionResult {
    if within_of(args)?.is_none() {
        return answered(agents, cx, turn(agents, cx, id).await?).await;
    }
    let until = agents.turn(&id).await?.until.unwrap_or(i64::MAX);
    let clock = cx.clock().clone();
    let waited = turn(agents, cx, id.clone());
    tokio::pin!(waited);
    let finished = tokio::select! {
        biased;
        finished = &mut waited => finished?,
        () = clock.sleep_until(until) => {
            agents.cancel(&id).await;
            waited.await?
        }
    };
    if finished.error.is_none() || clock.now_ms() < until {
        return answered(agents, cx, finished).await;
    }
    cx.log('s', "its time ran out, and its program was stopped");
    let mut said = report(&finished);
    (said["cut"], said["reply"], said["decision"]) = (json!(true), json!(""), Value::Null);
    Ok(said)
}

pub(crate) fn register(registry: &mut Registry, agents: &Agents) {
    add(
        registry,
        agents,
        ActionSpec::new("agent.ensure", "Bring an agent into the fleet, or update the one with this name.")
.example(ENSURE_ASKED)
            .making("an agent", "name")
            .required("name", T::String, "the agent's name")
            .arg("project", T::String, "the project it works in (default: this instance's)")
            .arg("channel", T::String, "a channel it should listen in")
            .arg(
                "reports",
                T::String,
                "a channel it is in to say there what it is asked there, as whatever brings it up reports \
                 there: listed among its members and asked there by its name. Into one kept to its members only \
                 where it may be in one",
            )
            .arg(
                "adapter",
                T::String,
                "what runs it: an installed extension, or `builtin` (default: this device's default)",
            )
            .arg("model", T::String, "which model, if the adapter has a choice")
            .arg("instructions", T::String, "standing instructions, given on every turn")
            .arg("scopes", T::List, "what its credential holds (the usual for an agent, if empty or not said; no more than whoever applied this holds)")
            .arg("permission_mode", T::String, "how much it may do unasked: read, edit (default) or full")
            .arg(
                "role",
                T::String,
                "the role it is brought up in: a word of the fleet's own, by which work is given to whichever \
                 agent of that role is free",
            )
            .arg(
                "rank",
                T::Int,
                "how it is ranked among those of its role, the higher before the lower: read only where work is \
                 given with `pick: rank` (default: 0, or what it was)",
            )
            .arg(
                "signins",
                T::List,
                "what its program signs in with, drawn for each turn from the sign-ins the factory holds, in \
                 order: `subscription`, `key`, or the name of one, e.g. [subscription, key]; needs \
                 `secrets:use`. [] for the sign-in of whoever runs the daemon (default)",
            )
            .arg(
                "wall",
                T::String,
                "whether its program works behind a wall, shown its work and a home of its own and nothing else \
                 of this device: `on` or `off`. A new agent has one; said as nothing, it is as it was. `off` \
                 needs `shell`",
            )
            .arg(
                "reads",
                T::List,
                "what else its program may read behind its wall: folders or files, written from the top or \
                 from the home of whoever runs the daemon (`~/.cargo`); needs `shell`",
            )
            .arg("changes", T::List, "what else it may change there, written the same way; needs `shell`")
            .arg(
                "git",
                T::String,
                "what its program may do with its project's repository behind its wall: `off` (it is not there), \
                 `read` (the usual: what it changes is committed for it), `commit` (it may commit its own work, \
                 and not move the project's own branch) or `write` (all of it). Whatever is said, the \
                 repository's settings and hooks are not its to change. Said as nothing, it is as it was",
            )
            .arg(
                "secrets",
                T::List,
                "the keys its program is handed, each a secret the factory keeps under a variable of the program's \
                 environment, written `VARIABLE=name` (`REMOTE_TOKEN=remote-bot`): a key of another service's, which \
                 says what the agent may do there; needs `secrets:use`. [] for none",
            )
            .arg(
                "outlives",
                T::String,
                "whether its program goes on without the daemon: `on` or `off`. With it on, the program is started \
                 so that it does not end when the daemon does, and a daemon that is started again takes its turn up \
                 where it is, still at work or finished, and the agent is not asked again. A new agent's does not; \
                 said as nothing, it is as it was",
            )
            .arg(
                "web",
                T::String,
                "whether its program may use the web unasked: `on` or `off`. With it on, a program that asks leave \
                 before it looks something up or fetches a page is told it need not, where its package says how; \
                 turning it on takes `shell`. A new agent's may not; said as nothing, it is as it was",
            )
            .capability("agents:write")
            .output("name", "the agent's name")
            .output("adapter", "what runs it")
            .output("role", "the role it is in, if any")
            .idempotent(),
        |agents, cx, args| async move {
            // Under the environment of what brings it up, as anything it starts is.
            let under = cx.engine().instance(cx.instance()).await.ok().map(|info| info.environment);
            let spec = SlotSpec {
                environment: under,
                name: text(&args, "name").unwrap_or_default(),
                adapter: text(&args, "adapter"),
                // Said and empty, a model or what it is told is none: what it is set up as is what is said.
                model: args.get("model").and_then(Value::as_str).map(str::to_string),
                // Always said: the instance's, where nothing else is, and where that is none an
                // agent it brings up works in none (empty), as the text was applied for.
                project: Some(text(&args, "project").or_else(|| cx.project().map(str::to_string)).unwrap_or_default()),
                channel: text(&args, "channel"),
                instructions: args.get("instructions").and_then(Value::as_str).map(str::to_string),
                // Said and empty, it is what an agent usually holds: a
                // row whose cell was emptied goes back to that, where
                // saying nothing would leave it holding what it held.
                scopes: args.get("scopes").and_then(Value::as_array).map(|scopes| match scopes.is_empty() {
                    true => kernel::principal::AGENT_SCOPES.iter().map(|scope| scope.to_string()).collect(),
                    false => scopes.iter().map(blueprint::expr::display).collect(),
                }),
                permission_mode: text(&args, "permission_mode"),
                signins: listed(&args, "signins"),
                // Said as nothing, it is no role: an agent that had one has none.
                role: args.get("role").and_then(Value::as_str).map(str::to_string),
                rank: args.get("rank").and_then(Value::as_i64),
                wall: crate::walled::said(args.get("wall"))?,
                reads: listed(&args, "reads"),
                changes: listed(&args, "changes"),
                git: text(&args, "git"),
                secrets: listed(&args, "secrets"),
                outlives: crate::running::said(args.get("outlives"))?,
                web: crate::running::said_of_the_web(args.get("web"))?,
            };
            // One this factory does not start is nothing a blueprint
            // brings up, counts or holds: it is never a row of a fleet's.
            let outside = agents.find(&blueprint::expr::slug(&spec.name)).await?.filter(Slot::external);
            if let Some(outside) = outside {
                return Err(ActionError::new(format!(
                    "`{name}` is an external agent, which this factory does not start: nothing that runs here \
                     brings it up or counts it. Call the agent this brings up something else, or take `{name}` \
                     away first (`{cli} agent remove {name}`)",
                    name = outside.name,
                    cli = brand::CLI
                )));
            }
            let slot = agents.ensure(spec, cx.principal()).await?;
            if let Some(reports) = text(&args, "reports") {
                agents.reports_in(&reports, &slot, cx.principal()).await?;
            }
            cx.hold(&format!("agent:{}", slot.name)).await?;
            let mut out = json!({ "name": slot.name, "adapter": slot.adapter, "project": slot.project });
            if !slot.role.is_empty() {
                out["role"] = json!(slot.role);
            }
            Ok(out)
        },
    );

    registry.add(Task {
        spec: ActionSpec::new("agent.task", "Give an agent a piece of work and wait for it to finish.")
.example(TASK_ASKED)
            .required("prompt", T::String, "what to do")
            .arg(
                "slot",
                T::String,
                "which agent, by its name, or as a record names one (`agent:<name>`; what names a person names no \
                 agent). Default: the one a `thread` that is named was last held with, or else whichever is \
                 available for the project, one with nothing in hand first",
            )
            .arg(
                "given",
                T::String,
                "who the work is given to, as a record says it, where `slot` names nobody: the same is taken of \
                 it as of `slot`",
            )
            .arg(
                "role",
                T::String,
                "with no `slot`: only an agent brought up in this role, whichever of them has nothing in hand",
            )
            .arg("thread", T::String, THREAD)
            .arg("apart", T::String, APART)
            .arg("not", T::List, "agents that may not take it, by name, whatever names them: such as those whose work it looks at")
            .arg("pick", T::String, PICK)
            .arg("about", T::Object, ABOUT)
            .arg(
                "cwd",
                T::String,
                "the directory the agent works in, relative to its project (default: the project itself); a conversation is carried on only where it was held",
            )
            .arg(
                "hidden",
                T::List,
                "folders within its project, from the project's top, its program is not shown, such as the places other pieces of work on the same item are done in; held by its wall, and by nothing where it has none (`unhidden`)",
            )
            .arg(
                "if_available",
                T::Bool,
                "succeed with `available: false` when there is no agent, instead of failing",
            )
            .arg(
                "channel",
                T::String,
                "a channel the agent says its reply in, as itself, when it has finished (default: it is said nowhere); needs `channels:write` as well. Where `thread` is one of that channel's (`thread:<message>`), it is said in that thread, and the agent is shown what was said there",
            )
            .arg("answers", T::String, "with `channel`: the id of a message there its reply answers, said beside it")
            .arg(
                "options",
                T::List,
                "ways the work may end that the agent chooses between, e.g. [done, ask]; the choice is `decision`, and is null when it made none",
            )
            .arg("after_restart", T::String, AFTER_RESTART)
            .arg("restarts", T::Int, RESTARTS)
            .arg("asking", T::String, ASKING)
            .arg("within", T::String, "how long the turn may take, as `45m`: past it the agent's program is stopped and this ends with `cut`, failing nothing, for whoever asked to ask it what it had done; one agent's, and not a council's")
            .arg("seats", T::Int, SEATS)
            .arg("needed", T::Int, "with `seats`: how many have to choose `yes` for it to pass; more than half of them, if 0 or not said")
            .arg("looker", T::String, "with `seats` 1: the one agent who looks, by its name, in place of one chosen of the role")
            .arg("yes", T::String, "with `seats`: the choice counted for the work (default `approve`)")
            .arg("no", T::String, "with `seats`: the choice counted against it (default `send_back`)")
            // The same done twice where it says which asking it is, and
            // refused by itself where it does not (`task`).
            .idempotent()
            .capability("agents:write")
            .output("reply", "what the agent said, without the line it chose on")
            .output("decision", "the option it chose, where it was given some; null when it chose none")
            .output("available", "whether there was an agent to ask")
            .output("named", "with `if_available`, when there was none: the agent that was named and is not up, if one was")
            .output("external", "with `if_available`, when there was none: whether the agent that was named is one this factory does not start, which takes up what is given to it itself")
            .output("slot", "the agent that did it")
            .output("unhidden", "with `hidden`: that its program was shown them all the same, having no wall to hide them")
            .output("cost_usd", "what the turn cost, where the adapter reports it")
            .output("refused", "what the agent's program said it was refused as it went, each by a word, where its package says how it tells; none where it said nothing")
            .output("again", "how many times the daemon was stopped in the middle of it and started again; 0 where it never was")
            .output("cut", "with `within`: its time ran out and its program was stopped, saying nothing")
            .output("duration_ms", "how long the turn took")
            .output("came", "with `seats`: `passed`, `failed` or `undecided`")
            .output("by", "with `seats`: who decided it, those who chose `yes` or those who chose `no`")
            .output("words", "with `seats`: what they said, each as `<agent>: <words>` where several look")
            .output("approved", "with `seats`: how many chose `yes`")
            .output("against", "with `seats`: how many chose `no`")
            .output("votes", "with `seats`: each seat that was counted, in order, with who sat, what it chose and what it said")
            .output("went_on", "with `seats`: each agent the daemon was stopped in the middle of, with how often")
            // The two rules a turn is given by, which are asked and
            // which no text that gives work writes.
            .reaching(dispatch::RULE)
            .reaching(crate::rule::RULE),
        agents: agents.clone(),
    });

    add(
        registry,
        agents,
        ActionSpec::new("agent.consult", "Ask an agent to look at something and choose what happens next.")
.example(CONSULT_ASKED)
            .required("prompt", T::String, "what to look at, and what the choice is about")
            .required("options", T::List, "the answers to choose between, e.g. [retry, fail]")
            .arg(
                "slot",
                T::String,
                "which agent (default: the one a `thread` that is named was last held with, or else whichever is \
                 available for the project, one with nothing in hand first)",
            )
            .arg(
                "role",
                T::String,
                "with no `slot`: only an agent brought up in this role, whichever of them has nothing in hand",
            )
            .arg("thread", T::String, THREAD)
            .arg("apart", T::String, APART)
            .arg("pick", T::String, PICK)
            .arg("about", T::Object, ABOUT)
            .arg(
                "if_available",
                T::Bool,
                "succeed with `available: false` when there is no agent, instead of failing",
            )
            .capability("agents:write")
            .output(
                "decision",
                "the option chosen; in a failure hook, `retry`, `continue` and `fail` take effect",
            )
            .output("reply", "what the agent said")
            .output("available", "whether there was an agent to ask")
            .reaching(dispatch::RULE)
            .reaching(crate::rule::RULE),
        |agents, cx, args| async move {
            if let Some(thread) = text(&args, "thread") {
                agents.may_go_on(&thread, cx.principal()).await?;
            }
            let chosen = agents.inner.choosing.lock().await;
            let Some(slot) = pick(&agents, &cx, &args).await? else {
                return Ok(json!({ "available": false, "reply": "", "decision": null }));
            };
            let options: Vec<String> = args
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(blueprint::expr::display)
                .collect();
            if options.is_empty() {
                return Err(ActionError::new("`options` needs at least one answer to choose from"));
            }
            let request = TurnRequest {
                slot: slot.name,
                kind: "consult".into(),
                thread: text(&args, "thread").unwrap_or_else(|| format!("instance:{}", cx.instance())),
                prompt: text(&args, "prompt").unwrap_or_default(),
                options,
                labels: cx.labels().clone(),
                ..TurnRequest::default()
            };
            let id = agents.enqueue_chosen(request).await?;
            drop(chosen);
            let finished = turn(&agents, &cx, id).await?;
            match &finished.error {
                None => Ok(report(&finished)),
                Some(error) => Err(ActionError::new(format!("{} did not decide: {error}", finished.slot))
                    .with_data(report(&finished))),
            }
        },
    );

    add(
        registry,
        agents,
        ActionSpec::new("agent.stop", "Stop an agent: at once, or when it has finished what it has in hand.")
            .example(STOP_ASKED)
            .required("name", T::String, "the agent's name")
            .arg(
                "idle",
                T::Bool,
                "let it finish what it has in hand first: it is given no more work from now, and is stopped \
                 when it has none (default: it is stopped at once, and what it was doing with it)",
            )
            .capability("agents:write")
            .output("stopped", "the agent's name")
            .output("leaving", "whether it is still finishing what it had in hand")
            .idempotent(),
        |agents, cx, args| async move {
            let name = text(&args, "name").unwrap_or_default();
            // One stopped already is left so, and only let go of: what a
            // role held itself, and what its own text stopped first.
            let stopped = agents.find(&name).await?.is_some_and(|slot| !slot.active);
            let leaving = if stopped {
                false
            } else if args.get("idle").and_then(Value::as_bool).unwrap_or(false) {
                !agents.leave(&name).await?
            } else {
                agents.deactivate(&name).await?;
                false
            };
            let _ = cx.release(&format!("agent:{name}")).await;
            Ok(json!({ "stopped": name, "leaving": leaving }))
        },
    );
}

/// Whether a blueprint's text says that what it is doing is not written
/// down as it goes (`durable: false`, said at the top of the text): an
/// instance of it that the daemon was stopped under is over, and begun
/// again is another instance under the same name.
pub(crate) fn keeps_nothing(source: &str) -> bool {
    source.lines().filter_map(|line| line.strip_prefix("durable:")).any(|said| {
        let said = said.split('#').next().unwrap_or_default().trim();
        said.trim_matches(|quote| quote == '"' || quote == '\'') == "false"
    })
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const ENSURE_ASKED: &str = "\
blueprint: a-builder
permissions: [agents:write]

steps:
  builder:
    do: agent.ensure
    with:
      name: builder
      role: worker
      instructions: \"Build what you are asked to, and say what you did.\"
";
const TASK_ASKED: &str = "\
blueprint: fix-the-build
permissions: [agents:write]

steps:
  fixed:
    do: agent.task
    with: { slot: builder, prompt: The build fails. Find out why and fix it. }
";
const CONSULT_ASKED: &str = "\
blueprint: ship-or-wait
permissions: [agents:write]

steps:
  asked:
    do: agent.consult
    with: { slot: builder, prompt: The tests pass. Should this ship today?, options: [ship, wait] }

  ship:
    when: ${ steps.asked.out.decision == \"ship\" }
    do: emit
    with: { topic: release.asked }
";
const STOP_ASKED: &str = "\
blueprint: builder-done
permissions: [agents:write]

steps:
  stopped:
    do: agent.stop
    with: { name: builder, idle: true }
";
