//! `pool.turn`: who is given a place, the one thing a pool works out.
//!
//! `std/pool` keeps who holds a place and who waits for one, hears what
//! asks and what gives a place back, and says who may begin. Working out
//! what its two lists become when one of those things happens is done
//! here: a function of what the pool keeps and what has just happened,
//! which keeps nothing itself (`kernel::pool`, which the runner offers
//! as `conductor::pool`). The blueprint says when, keeps the answer
//! and tells those it concerns. The one thing asked of anyone is whether
//! each of those the pool names is still there: one that is gone holds
//! nothing and waits for nothing, whether or not the pool was told.

use blueprint::{ActionSpec, ValueType};
use conductor::pool::{
    Abouts, Came, NAMED, Order, Priorities, kept_about, kept_priorities, ranked, told, turn_ranked, who,
};
use conductor::{ActionError, ActionResult, Registry};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

/// Adds `pool.turn`, which is [`turn`] for a blueprint.
pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "pool.turn",
        "Work out who of those waiting for a place is given one, after something asked, gave one back or ended.",
    )
    .deciding()
    .required("held", ValueType::List, "who holds a place now, each by the instance it is")
    .required("waiting", ValueType::List, "who waits for one, in the order they asked")
    .arg("limit", ValueType::Int, "how many hold a place at once; less than 1 is as many as ask (default 1)")
    .arg("paused", ValueType::String, "why no place is given for now; empty while places are given")
    .arg("order", ValueType::String, "which of those waiting is given a place: `first` to have asked (default), `last`, or the highest `priority` each said as it asked, the first to have asked of those as high")
    .arg("asks", ValueType::String, "an instance that asks for a place, or `key:<name>` for a name one is asked as")
    .arg("gives_back", ValueType::String, "an instance, or such a name, that gives back the place it held")
    .arg("asks_as", ValueType::String, "the name a place is asked as, where it is asked by a name: it is then the name that asks, and not the instance")
    .arg("gives_back_as", ValueType::String, "the name a place is given back as, in the same way")
    .arg("ended", ValueType::String, "an instance that has ended, holding a place or waiting for one")
    .arg("about", ValueType::String, "what whoever asks says it asks about: a thing that waits with it, to be said back when it is told that it waits")
    .arg("abouts", ValueType::Object, "what each that holds a place or waits for one asked about, by who it is, as it was kept")
    .arg("priority", ValueType::Any, "the priority whoever asks says it has, a number: what `order: priority` goes by, the higher first; 0 where it says none")
    .arg("priorities", ValueType::Object, "the priority each that holds a place or waits for one said as it asked, by who it is, as it was kept")
    .output("held", "who holds a place after it")
    .output("waiting", "who still waits")
    .output("given", "who was given a place just now, in the order they were given it")
    .output("who", "who it was that asked or gave back, as the pool keeps them; empty where nobody did")
    .output("granted", "each that was given a place: `to`, the name it asked `as` if any, and whether it `waited`, which whoever has just asked or given a place back did not")
    .output("waiters", "each that waits and holds no place: `to`, and `about`, what it asked about: what it said when it asked, and otherwise the name it asked as, if any")
    .output("about", "what whoever has just asked, or given a place back, asked about, in the same way")
    .output("abouts", "what each that now holds a place or waits for one asked about, by who it is: to be kept, and handed back the next time")
    .output("priorities", "the priority each that now holds a place or waits for one said, by who it is: to be kept, and handed back the next time")
    .output("waits", "whether whoever has just asked, or given a place back, now waits for one and holds none: one that asks again while it holds a place is at work, and is told it waits only once it has given that back")
    .idempotent();
    registry.add_fn(spec, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            // A place is held, and waited for, by something that can end,
            // and the pool is told when it does. It also looks for itself,
            // each time: what asked and was gone before the pool heard it
            // ask (a conductor stopped between the two) would otherwise be
            // given a place that nothing would ever give back.
            // A name a place was asked as, by something that looks after
            // many things and asks for each, is no instance and is never
            // gone: it is there until whoever asked gives its place back.
            // Asked of the engine once for each that is named, before
            // anything is worked out: one in a conductor that is taking
            // up what it keeps is looked for where it is written.
            let named = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let mut all: Vec<String> =
                named("held").iter().chain(&named("waiting")).map(blueprint::expr::display).collect();
            all.extend(text(&args, "asks").map(str::to_string));
            for kept in ["abouts", "priorities"] {
                all.extend(
                    args.get(kept)
                        .and_then(Value::as_object)
                        .into_iter()
                        .flatten()
                        .map(|(who, _)| who.clone()),
                );
            }
            let mut there = std::collections::HashSet::new();
            for who in all {
                if !who.starts_with(NAMED) && !there.contains(&who) && cx.engine().is_there(&who).await {
                    there.insert(who);
                }
            }
            let here = |who: &String| who.starts_with(NAMED) || there.contains(who);
            let list = |key: &str| -> Vec<String> {
                named(key).iter().map(blueprint::expr::display).filter(here).collect()
            };
            let said = |key: &str| text(&args, key);
            // Whoever asks, and whoever gives back: each by the name it
            // does so as, where it does so by a name. One that asks and is
            // gone asks for nothing.
            let asks = who(said("asks"), said("asks_as")).filter(here);
            let gives_back = who(said("gives_back"), said("gives_back_as"));
            let came = match (asks.as_deref(), gives_back.as_deref(), said("ended")) {
                (Some(who), None, None) => Came::Asked(who),
                (None, Some(who), None) => Came::GaveBack(who),
                (None, None, Some(who)) => Came::Ended(who),
                (None, None, None) => Came::Nothing,
                _ => {
                    return Err(ActionError::new(
                        "one thing happens at a time: say one of `asks`, `gives_back` and `ended`",
                    ));
                }
            };
            let limit = args.get("limit").and_then(Value::as_i64).unwrap_or(1);
            let order = Order::named(said("order").unwrap_or_default()).map_err(ActionError::new)?;
            // Who waited, holding nothing, before this turn: one of
            // those that asks again and is given a place did wait.
            let (held, waiting) = (list("held"), list("waiting"));
            let sat: Vec<String> = waiting.iter().filter(|who| !held.contains(who)).cloned().collect();
            // The priority each said as it asked, as it was kept, of those
            // that are still there, with what whoever asks now says.
            let kept: Priorities = args
                .get("priorities")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter(|(who, _)| here(who))
                .filter_map(|(who, priority)| priority.as_f64().map(|priority| (who.clone(), priority)))
                .collect();
            let says = args.get("priority").and_then(number);
            let priorities = ranked(kept, came, says);
            let open = said("paused").is_none();
            let (held, waiting, given) = turn_ranked(held, waiting, came, limit, open, order, &priorities);
            let priorities = kept_priorities(priorities, &held, &waiting);
            // What each asked about, as it was kept: of those that are still there.
            let kept: Abouts = args
                .get("abouts")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter(|(who, _)| here(who))
                .map(|(who, about)| (who.clone(), blueprint::expr::display(about)))
                .collect();
            let abouts = kept_about(kept, came, said("about"), &held, &waiting);
            let told = told(&given, &waiting, &held, came, &abouts, &sat);
            let who = asks.as_deref().or(gives_back.as_deref()).unwrap_or_default();
            Ok(json!({
                "held": held,
                "waiting": waiting,
                "given": given,
                "who": who,
                "granted": told.granted,
                "waiters": told.waiters,
                "waits": told.waits,
                "about": told.about,
                "abouts": abouts,
                "priorities": priorities,
            }))
        })
    });
}

/// A number, as a priority is said: written as one, or as text that reads
/// as one (a template that is more than one hole is text). Nothing, where
/// it is neither, and whoever said it said no priority.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

fn text<'a>(args: &'a Fields, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str).filter(|text| !text.is_empty())
}
