//! Who is given a place: what a pool works out, as a function of what it
//! keeps and what has just happened.
//!
//! A pool is a blueprint (`std/pool`) that keeps who holds a place and
//! who waits for one. What its two lists become when something asks or
//! gives a place back is worked out here, from plain values, with
//! nothing kept and no engine in it: a blueprint asks it as `pool.turn`
//! (`blocks::pool`), and the runner offers it under the names it always
//! had (`conductor::pool`).

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// Which of those waiting is given a place that is free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Whoever asked first.
    First,
    /// Whoever asked last.
    Last,
    /// Whoever said the highest priority as it asked, and of those that
    /// said the same, whoever asked first: one that said none said 0.
    Priority,
}

impl Order {
    pub fn named(word: &str) -> Result<Order, String> {
        match word {
            "" | "first" => Ok(Order::First),
            "last" => Ok(Order::Last),
            "priority" => Ok(Order::Priority),
            other => Err(format!(
                "`order` is which of those waiting is given a place: `first` or `last` to have asked, or the \
                 `priority` each said as it asked; `{other}` is none of them"
            )),
        }
    }
}

/// The priority each that holds a place or waits for one said as it
/// asked, by who it is: kept for as long as it holds or waits, as what it
/// asked about is ([`Abouts`]).
pub type Priorities = BTreeMap<String, f64>;

/// What `who` said of its priority as it asked: 0, where it said none.
pub fn priority_of(priorities: &Priorities, who: &str) -> f64 {
    priorities.get(who).copied().unwrap_or(0.0)
}

/// What is kept of each one's priority once `came` has happened: what was
/// kept before, with what whoever has just asked says (`says`). One that
/// asks again and says nothing is of the priority it was. Asked before a
/// turn is worked out, so that whoever has just asked is ranked by it.
pub fn ranked(mut priorities: Priorities, came: Came<'_>, says: Option<f64>) -> Priorities {
    if let (Came::Asked(who), Some(priority)) = (came, says) {
        priorities.insert(who.to_string(), priority);
    }
    priorities
}

/// What is kept of each one's priority after a turn that left `held` and
/// `waiting`: of whoever still holds a place or waits for one, and nobody
/// else.
pub fn kept_priorities(mut priorities: Priorities, held: &[String], waiting: &[String]) -> Priorities {
    priorities.retain(|who, _| held.contains(who) || waiting.contains(who));
    priorities
}

/// What has just happened to a pool, if anything has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Came<'a> {
    /// Something asks for a place. It joins those waiting; one that asks
    /// while it holds a place waits for its next.
    Asked(&'a str),
    /// A place is given back by whoever held it.
    GaveBack(&'a str),
    /// Whoever held a place, or waited for one, has ended: its place is
    /// free and it waits for nothing.
    Ended(&'a str),
    /// Nothing but that there may be places to give: it goes on after a
    /// pause, or has another limit.
    Nothing,
}

/// A pool as it stands after `came`: who holds a place, who waits, and
/// who was given one just now, in the order they were given it.
///
/// A place is given to one that waits and holds none, for as long as there
/// is one to give: while `open`, and while fewer hold one than `limit`
/// says (less than 1 is as many as ask).
pub fn turn(
    held: Vec<String>,
    waiting: Vec<String>,
    came: Came<'_>,
    limit: i64,
    open: bool,
    order: Order,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    turn_ranked(held, waiting, came, limit, open, order, &Priorities::new())
}

/// [`turn`], where those waiting may have said a priority as they asked
/// (`priorities`, [`ranked`]), which is what [`Order::Priority`] goes by.
pub fn turn_ranked(
    mut held: Vec<String>,
    mut waiting: Vec<String>,
    came: Came<'_>,
    limit: i64,
    open: bool,
    order: Order,
    priorities: &Priorities,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    match came {
        Came::Asked(who) if !waiting.iter().any(|waits| waits == who) => waiting.push(who.to_string()),
        Came::Asked(_) | Came::Nothing => {}
        Came::GaveBack(who) => held.retain(|holds| holds != who),
        Came::Ended(who) => {
            held.retain(|holds| holds != who);
            waiting.retain(|waits| waits != who);
        }
    }
    let mut given = Vec::new();
    while open && (limit < 1 || (held.len() as i64) < limit) {
        let free = |who: &String| !held.contains(who);
        let next = match order {
            Order::First => waiting.iter().position(free),
            Order::Last => waiting.iter().rposition(free),
            // The highest, and of several as high the first to have
            // asked: a later one is taken only where it is higher.
            Order::Priority => waiting
                .iter()
                .enumerate()
                .filter(|(_, who)| free(who))
                .fold(None, |best: Option<(usize, f64)>, (at, who)| {
                    let priority = priority_of(priorities, who);
                    match best {
                        Some((_, highest)) if priority <= highest => best,
                        _ => Some((at, priority)),
                    }
                })
                .map(|(at, _)| at),
        };
        let Some(next) = next else { break };
        let who = waiting.remove(next);
        held.push(who.clone());
        given.push(who);
    }
    (held, waiting, given)
}

/// How a name is written that a place is asked as, in place of the
/// instance that asks: what no instance is called. A pool keeps it so
/// among those who hold and wait ([`who`]).
pub const NAMED: &str = "key:";

/// Who it is that asks for a place or gives one back, as a pool keeps
/// them: the name it does so as, where it does so by a name, and
/// otherwise the instance that it is. Nobody, where neither is said.
pub fn who(instance: Option<&str>, named: Option<&str>) -> Option<String> {
    match named.filter(|name| !name.is_empty()) {
        Some(name) => Some(format!("{NAMED}{name}")),
        None => instance.filter(|instance| !instance.is_empty()).map(str::to_string),
    }
}

/// The name one that holds or waits asked as; nothing, for an instance.
pub fn name_of(who: &str) -> &str {
    who.strip_prefix(NAMED).unwrap_or_default()
}

/// What a pool says of a turn: to each that was given a place, and of
/// each that waits for one.
///
/// One that asks again while it holds a place waits for its next, and the
/// pool keeps it among those waiting; but it is at work, and is not said
/// to wait: what it is about is not sitting anywhere unexplained. It is
/// told that it waits when it has given its place back and has to wait
/// for its next behind somebody else, and not before. Said the other way,
/// something asked for twice (a card said to be due by two things that
/// both found it) was told it waited its turn behind itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Told {
    /// Each that was given a place just now: `to`, who; `as`, the name it
    /// asked as, where it asked by one; and `waited`, whether it had to
    /// wait. The one this turn was about did not: it has just asked, or
    /// is given its next place as it gives the last back. Unless it was
    /// waiting already, holding nothing, and asked again: that one was
    /// told it waits and is owed being told its turn came. A name is
    /// asked for by more than one thing that finds it (a card said to
    /// be due twice), and where the second asking came as a pool went
    /// on after a pause, the card it was for kept saying that it waited
    /// through its being worked and after.
    pub granted: Vec<Value>,
    /// Each that waits and holds no place: `to`, who; and `about`, what
    /// it asked about ([`about_of`]).
    pub waiters: Vec<Value>,
    /// Whether whoever has just asked, or given a place back, now waits
    /// for one and holds none.
    pub waits: bool,
    /// What whoever has just asked, or given a place back, asked about;
    /// nothing where the turn was about nobody.
    pub about: String,
}

/// What each that holds a place or waits for one asked about, by who it
/// is: said when it asked, and kept for as long as it holds or waits.
pub type Abouts = BTreeMap<String, String>;

/// What `who` asked about: what it said when it asked, where it said, and
/// otherwise the name it asked as, which is nothing for an instance.
pub fn about_of<'a>(abouts: &'a Abouts, who: &'a str) -> &'a str {
    abouts.get(who).map(String::as_str).filter(|about| !about.is_empty()).unwrap_or_else(|| name_of(who))
}

/// What is kept of what each asked about after a turn that `came` of
/// something and left `held` and `waiting`: what was kept before, with
/// what whoever has just asked says it is about (`says`), and without
/// whoever neither holds a place nor waits for one any more. One that
/// asks again and says nothing of what about is about what it was.
pub fn kept_about(
    mut abouts: Abouts,
    came: Came<'_>,
    says: Option<&str>,
    held: &[String],
    waiting: &[String],
) -> Abouts {
    if let (Came::Asked(who), Some(about)) = (came, says.filter(|about| !about.is_empty())) {
        abouts.insert(who.to_string(), about.to_string());
    }
    abouts.retain(|who, _| held.contains(who) || waiting.contains(who));
    abouts
}

/// What is said after a turn that `came` of something, gave places to
/// `given`, and left `held` holding one and `waiting` for one, each
/// having asked about what `abouts` keeps; `sat` are those that waited
/// and held no place before the turn.
pub fn told(
    given: &[String],
    waiting: &[String],
    held: &[String],
    came: Came<'_>,
    abouts: &Abouts,
    sat: &[String],
) -> Told {
    // Whoever the turn was about: who asked, or who gave a place back.
    let subject = match came {
        Came::Asked(who) | Came::GaveBack(who) => Some(who),
        Came::Ended(_) | Came::Nothing => None,
    };
    let sits = |who: &str| waiting.iter().any(|waits| waits == who) && !held.iter().any(|holds| holds == who);
    let granted = given
        .iter()
        .map(|who| {
            let waited = subject != Some(who.as_str()) || sat.contains(who);
            json!({ "to": who, "as": name_of(who), "waited": waited })
        })
        .collect();
    let waiters = waiting
        .iter()
        .filter(|who| sits(who))
        .map(|who| json!({ "to": who, "about": about_of(abouts, who) }))
        .collect();
    Told {
        granted,
        waiters,
        waits: subject.is_some_and(sits),
        about: subject.map(|who| about_of(abouts, who).to_string()).unwrap_or_default(),
    }
}
