//! A comparison whose shares grow by a rule its user writes: a gradual
//! rollout, as a service's canary is rolled out.
//!
//! A comparison that shares its things out (one not asked) gives each new
//! thing the way that has had least of its share. A rule says that one of
//! its ways (`grows`) is to be given more of the things while what was
//! counted of them holds against another way's (`against`): over at least
//! `over` things of each that the count it names said something of since
//! the shares were last set, how often a thing of it came to `came` is no
//! more often (or, said `at_least`, at least as often) than the other's.
//! Each time that holds its part of the whole goes up by `step` points,
//! to `most` at most; each time it does not, its part stays where it is,
//! or where the rule says a `least`, goes back a step, to that at least.
//! Either way what is read begins again from there, and what was decided
//! is said with why ([`Shared`]).
//!
//! Everything here is a function of plain values: what the counts come to
//! ([`decide`]), what shares a part of the whole comes to ([`shares_at`]),
//! what is wrong with a rule as it is given ([`wrong`]). Nothing here
//! counts, keeps or changes anything, and nothing here scores: the rule
//! is whoever wrote it's, and it only reads the counts as they say.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Comparison, Way};

/// A rule a comparison's shares change by.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// The way whose share grows.
    pub grows: String,
    /// The way it is held against.
    pub against: String,
    /// The count it goes by, as the comparison calls it.
    pub count: String,
    /// What that count came to which is held: the last of several fields
    /// says, as for what is accepted ([`super::accepted`]).
    pub came: String,
    /// Held to coming to it at least as often as the other way, in place
    /// of no more often: for what is wanted (passed) and not for what is
    /// not (sent back).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub at_least: bool,
    /// How many things of each way are read before anything is decided.
    pub over: u32,
    /// The points of the whole its share goes up by each time it holds.
    pub step: u32,
    /// The most of the whole it is given: a part of a hundred.
    pub most: u32,
    /// Where what is held came more often, the least of a hundred its
    /// share goes back to, a step at a time: a canary rolled back. Said
    /// nothing, the share stays where it is. One word for both the falling
    /// back and how far, as `most` is for growing: a falling back with no
    /// floor would take the way to nothing, which is ending the comparison.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub least: Option<u32>,
}

/// What the rule has read since the shares were last set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rolling {
    /// When the shares were last set, by the rule or by somebody: what was
    /// said before is no part of what is read now.
    pub since: i64,
    /// Each thing the rule's count said something of since then: the way
    /// it went, and whether it came to what is held.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub seen: BTreeMap<String, Seen>,
    /// The rule has taken its way as far as it goes, and decides nothing more.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

/// A thing as the rule has read it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seen {
    pub way: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub came: bool,
}

/// The shares said again: by the rule, or by somebody, and why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shared {
    pub at: i64,
    /// Who set them: an actor, or [`RULE`] where the rule did.
    pub by: String,
    /// Each way's share before, and after: the same where the rule held it.
    pub was: BTreeMap<String, u32>,
    pub now: BTreeMap<String, u32>,
    pub why: String,
}

/// Who set the shares where the rule did.
pub const RULE: &str = "rule";

/// How many things of a way the rule has read, and of those how many came
/// to what is held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub things: u32,
    pub came: u32,
}

/// What the rule comes to, read now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rolled {
    /// Not enough has been read of one of the two ways yet.
    Wait,
    /// It holds: the way that grows is given `to` of a hundred, and where
    /// that is as far as the rule goes, it is `done`.
    Raised { to: u32, why: String, done: bool },
    /// It does not hold: the shares stay as they are.
    Held { why: String },
    /// It does not hold and the rule falls back: the way that grows goes
    /// back to `to` of a hundred, no less than the rule's least.
    Lowered { to: u32, why: String },
}

/// The ways' shares, by name.
pub fn shares(ways: &[Way]) -> BTreeMap<String, u32> {
    ways.iter().map(|way| (way.name.clone(), way.share)).collect()
}

/// The part of a hundred of the things `name` is given by the shares,
/// to the nearest.
pub fn percent(ways: &[Way], name: &str) -> u32 {
    let sum: u64 = ways.iter().map(|way| u64::from(way.share)).sum();
    let share = ways.iter().find(|way| way.name == name).map_or(0, |way| u64::from(way.share));
    if sum == 0 { 0 } else { ((share * 200 + sum) / (2 * sum)) as u32 }
}

/// The shares that give `grows` `to` of a hundred, the rest shared among
/// the others as their shares were, each given one at least: in the order
/// the ways are listed. Where `grows` is not a way, or there is no other,
/// the shares as they are.
pub fn shares_at(ways: &[Way], grows: &str, to: u32) -> Vec<u32> {
    let mut now: Vec<u32> = ways.iter().map(|way| way.share).collect();
    let others: Vec<usize> = (0..ways.len()).filter(|at| ways[*at].name != grows).collect();
    let Some(at) = ways.iter().position(|way| way.name == grows) else { return now };
    if others.is_empty() || to == 0 || to >= 100 {
        return now;
    }
    let rest = u64::from(100 - to).max(others.len() as u64);
    let sum: u64 = others.iter().map(|at| u64::from(ways[*at].share.max(1))).sum();
    // Each its part of the rest, rounded down, and what is left over to
    // those that lost most by it, the first listed first among equals.
    let mut parts: Vec<(usize, u64, u64)> = others
        .iter()
        .map(|at| {
            let share = u64::from(ways[*at].share.max(1));
            (*at, rest * share / sum, rest * share % sum)
        })
        .collect();
    let mut left = rest - parts.iter().map(|part| part.1).sum::<u64>();
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|a, b| parts[*b].2.cmp(&parts[*a].2).then(a.cmp(b)));
    for index in order.into_iter().cycle() {
        if left == 0 {
            break;
        }
        parts[index].1 += 1;
        left -= 1;
    }
    // None is given nothing: one is taken from the largest for each.
    while let Some(none) = parts.iter().position(|part| part.1 == 0) {
        let largest = (0..parts.len()).max_by_key(|index| (parts[*index].1, usize::MAX - index)).unwrap_or(0);
        parts[largest].1 -= 1;
        parts[none].1 = 1;
    }
    now[at] = to;
    for (at, part, _) in parts {
        now[at] = part as u32;
    }
    now
}

/// Whether what a count came to (`came`) is what the rule holds
/// (`held`): the whole of it, or the last of several fields.
pub fn came_to(held: &str, came: &str) -> bool {
    let last = came.rsplit('>').next().unwrap_or_default();
    !held.is_empty() && (held == came || held == last)
}

/// How many things of `way` the rule has read, and how many came to what
/// is held.
pub fn tally(rolling: &Rolling, way: &str) -> Tally {
    rolling.seen.values().filter(|seen| seen.way == way).fold(Tally::default(), |tally, seen| Tally {
        things: tally.things + 1,
        came: tally.came + u32::from(seen.came),
    })
}

/// What `rule` comes to, the way that grows being given `now` of a
/// hundred, with what was read of it (`grows`) and of the way it is held
/// against (`against`).
pub fn decide(rule: &Rule, now: u32, grows: Tally, against: Tally) -> Rolled {
    if grows.things < rule.over || against.things < rule.over || now >= rule.most {
        return Rolled::Wait;
    }
    // Compared without dividing: came / things of one against the other's.
    let (one, other) = (
        u64::from(grows.came) * u64::from(against.things),
        u64::from(against.came) * u64::from(grows.things),
    );
    let holds = if rule.at_least { one >= other } else { one <= other };
    let read = format!(
        "`{}` came to `{}` of its `{}` for {} of the {} things read since its share was last set, and `{}` for {} \
         of {}",
        rule.grows,
        rule.came,
        rule.count,
        grows.came,
        grows.things,
        rule.against,
        against.came,
        against.things
    );
    let (often, not) = match rule.at_least {
        true => ("at least as often", "less often"),
        false => ("no more often", "more often"),
    };
    if !holds && let Some(least) = rule.least.filter(|least| now > *least) {
        let to = now.saturating_sub(rule.step).max(least);
        let floor = if to == least { ", the least its rule gives it" } else { "" };
        return Rolled::Lowered {
            to,
            why: format!("{read}: {not}, so its share goes back from {now}% to {to}%{floor}."),
        };
    }
    if !holds {
        return Rolled::Held {
            why: format!("{read}: {not}, so its share stays at {now}%, and what comes next is read again."),
        };
    }
    let to = (now + rule.step).min(rule.most);
    let done = to >= rule.most;
    let last = if done { ", as far as its rule takes it" } else { "" };
    Rolled::Raised {
        to,
        done,
        why: format!("{read}: {often}, so its share goes from {now}% to {to}%{last}."),
    }
}

/// The rule in a sentence, as it is shown.
pub fn said(rule: &Rule) -> String {
    let often = if rule.at_least { "at least as often as" } else { "no more often than" };
    format!(
        "`{}` is given {} more of every hundred things at a time, up to {}, each time that over {} things of each \
         since its share was last set its `{}` came to `{}` {often} `{}`'s did{}",
        rule.grows,
        rule.step,
        rule.most,
        rule.over,
        rule.count,
        rule.came,
        rule.against,
        match rule.least {
            Some(least) => format!(", and goes back as much each time it did not, to {least} at least"),
            None => String::new(),
        }
    )
}

/// What is wrong with `rule` for `one`, as somebody gives it; nothing
/// where it can be.
pub fn wrong(rule: &Rule, one: &Comparison) -> Option<String> {
    let name = &one.name;
    if one.asked {
        return Some(format!(
            "each start that names `{name}` names its way, so its shares say nothing and no rule changes them"
        ));
    }
    if one.ended.is_some() {
        return Some(format!("`{name}` has ended, and its shares are no longer changed"));
    }
    let ways: Vec<&str> = one.ways.iter().map(|way| way.name.as_str()).collect();
    for (said, what) in [(&rule.grows, "grows"), (&rule.against, "is held against")] {
        if one.way(said).is_none() {
            return Some(format!(
                "the way that {what} is `{said}`, and `{name}` has no such way (its ways are {})",
                ways.join(", ")
            ));
        }
    }
    if rule.grows == rule.against {
        return Some(format!("`{}` cannot be held against itself; say another of its ways", rule.grows));
    }
    if !one.counts.iter().any(|count| count.name == rule.count) {
        let counts: Vec<&str> = one.counts.iter().map(|count| count.name.as_str()).collect();
        return Some(match counts.is_empty() {
            true => format!("the rule goes by the count `{}`, and `{name}` counts nothing", rule.count),
            false => format!(
                "the rule goes by the count `{}`, and `{name}` counts {}",
                rule.count,
                counts.join(", ")
            ),
        });
    }
    if rule.came.trim().is_empty() {
        return Some(format!("say what `{}` came to that the rule holds, such as `failed`", rule.count));
    }
    if rule.over == 0 {
        return Some("the rule reads at least one thing of each way before it decides anything".into());
    }
    if rule.step == 0 || rule.most == 0 || rule.most >= 100 {
        return Some(format!(
            "a share grows by a part of a hundred and to less than all of it: the step is {} and the most is {}; \
             all of it is ending the comparison with the way that goes on",
            rule.step, rule.most
        ));
    }
    let others = one.ways.len() as u32 - 1;
    if 100 - rule.most < others {
        return Some(format!(
            "at {}% `{}` would leave the other {others} ways less than one each; say less",
            rule.most, rule.grows
        ));
    }
    let now = percent(&one.ways, &rule.grows);
    if now >= rule.most {
        return Some(format!(
            "`{}` is given {now}% of the things already, which is the most the rule says ({}%)",
            rule.grows, rule.most
        ));
    }
    if let Some(least) = rule.least
        && (least == 0 || least > now)
    {
        return Some(format!(
            "the least `{}` goes back to is {least}%, and it is given {now}% now; say no more than that, and at \
             least 1: a way given nothing is ending the comparison",
            rule.grows
        ));
    }
    None
}
