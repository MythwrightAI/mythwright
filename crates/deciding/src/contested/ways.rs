//! What a contest's contenders come to: the ways of the comparison it is
//! counted under, the letter a judge knows each by, where each works, how
//! much of what each changed a judge is shown, and what is wrong with a
//! setup.
//!
//! A contender is written as a role of the fleet (`otter`: whichever agent
//! of that role takes it, whose row says the model), an agent by its name
//! (`@shop-slate-1`), or the name of a way the contest's comparison has
//! already (a fork, a version, other words, said where the comparison was
//! started). Any number of contenders, two or more.
//!
//! Where judging is blind, each is shown by a letter, `A` to `Z` and then
//! `AA` and on, in an order turned by the item so that no contender is
//! always `A`; else by its way. Each works in a place of its own under a
//! key that is the item's and its letter's where judging is blind
//! (`<item>--a`), so that a place says nothing of whose it is to a judge
//! who reads it, and the item's and the way's where not
//! (`<item>--<way>`); and is not shown the others' places. What a judge is shown of each is bounded,
//! so that many entries together still fit what a judge reads.
//!
//! A shadow test names its incumbent (`shadow`), the contender whose work
//! is the item's whatever the judges say: it is one of the contenders,
//! the first where the contenders as written leave it out, so that a test
//! of one challenger beside the usual setup is written as the challenger
//! alone. Which entry it is, is said; what becomes of it is
//! `contest.outcome`'s.
//!
//! A contender whose row says its work is over (kept, not kept, failed or
//! judged) is not started again where a test is taken up after a stop:
//! its place may be half given to the item already, and started again it
//! would make its place anew under a branch the item's place is on.

use std::collections::BTreeSet;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use super::{count, letter, said, text};
use crate::add;

/// The most of a patch an entry is shown: what `place.changed` gives at most.
pub const MOST_PATCH: u64 = 1 << 20;
/// The least, however many there are: enough to read a change by.
pub const LEAST_PATCH: u64 = 4 << 10;
/// What every entry's patch together comes to where nothing is said.
pub const ALL_PATCH: u64 = 128 << 10;

/// What each contender's work is handed of how it is finished where it
/// is worked: `finish`, `later` and `lasts`, each only where it is said,
/// so that a way that is an earlier text taking none of it is handed none
/// and is not refused for a param it never declared.
pub fn finishing(finish: &[Value], later: &[Value], lasts: &str) -> serde_json::Map<String, Value> {
    let mut handed = serde_json::Map::new();
    for (key, list) in [("finish", finish), ("later", later)] {
        if !list.is_empty() {
            handed.insert(key.into(), json!(list));
        }
    }
    if !lasts.trim().is_empty() {
        handed.insert("lasts".into(), json!(lasts.trim()));
    }
    handed
}

/// An agent of the fleet, by its name and its role.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub role: String,
}

/// A contest as it is set up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Setup {
    /// The item, and the contest by its id.
    pub item: String,
    pub contest: String,
    /// The contenders as written.
    pub contenders: Vec<String>,
    /// The comparison it is counted under, where one is named, and the ways
    /// that one has already.
    pub compared: String,
    pub known: Vec<String>,
    /// The fleet's agents, with their roles; nothing where they could not be read.
    pub members: Vec<Member>,
    /// Who judges: a role, or an agent by `@` its name.
    pub judges: String,
    pub blind: bool,
    /// How many bytes of patch a judge is shown of every entry together.
    pub patch: u64,
    /// The incumbent of a shadow test, as written; nothing for a test of equals.
    pub shadow: String,
    /// What the entries' rows say already, each its name and its state.
    pub rows: Vec<(String, String)>,
}

/// One contender, as it is started and shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// What it was written as, and the way it comes to.
    pub written: String,
    pub way: String,
    /// What the way hands the work: a role or an agent, or nothing for a
    /// way the comparison says for itself.
    pub hands: Option<(&'static str, String)>,
    /// What a judge picks it by.
    pub letter: String,
    /// The key of its place, and of its row.
    pub key: String,
    pub name: String,
    /// The bytes of what it changed a judge is shown.
    pub patch: u64,
    /// The other entries' places, which it is not shown.
    pub hidden: Vec<String>,
    /// It is the incumbent of a shadow test.
    pub incumbent: bool,
    /// Its row says its work is over, and it is not started again.
    pub over: bool,
}

/// What the contenders come to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ways {
    /// `ready`, or `wrong` with why.
    pub came: &'static str,
    pub why: String,
    /// The comparison it is counted under.
    pub comparison: String,
    /// The entries, in the order a judge is shown them.
    pub entries: Vec<Entry>,
    /// The agents that work an entry, by name, which may not judge.
    pub not: Vec<String>,
    /// The incumbent's way, where it is a shadow test.
    pub incumbent: String,
}

/// What an entry's row says once its contender's work is over.
pub const OVER: &[&str] = &["kept", "unkept", "failed", "judged"];

/// Every word [`ways`] can answer.
pub const WAYS_WORDS: &[&str] = &["ready", "wrong"];

/// Whether `name` can be a way's or a comparison's name: a word of
/// lowercase letters, digits, `-` and `_`, of forty at most.
fn word(name: &str) -> bool {
    let mut letters = name.chars();
    letters.next().is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && name.len() <= 40
        && letters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// Where the letters begin for `item`: the same for an item every time,
/// and not the same for every item.
fn turned(item: &str, many: usize) -> usize {
    let sum = item.bytes().fold(0usize, |sum, byte| sum.wrapping_mul(31).wrapping_add(byte as usize));
    sum % many.max(1)
}

fn wrong(why: String) -> Result<Ways, String> {
    Ok(Ways { came: "wrong", why, ..Default::default() })
}

/// The agents a contender or a judge written as `word` may be.
fn agents_of(word: &str, members: &[Member]) -> BTreeSet<String> {
    match word.strip_prefix('@') {
        Some(name) => BTreeSet::from([name.to_string()]),
        None => members.iter().filter(|one| one.role == word).map(|one| one.name.clone()).collect(),
    }
}

/// What the comparison of a contest's own is called, where it names none
/// it is counted under: the one place that is said.
pub fn own(contest: &str) -> String {
    format!("ab-{contest}")
}

/// The comparisons of their own that `tests` are counted under, in their
/// order: each a contest's row (its `id`, and `compared` where it names a
/// comparison, which has none of its own) or a contest's id alone, as a
/// contest that is gone is known by.
pub fn owned(tests: &[Value]) -> Vec<String> {
    tests
        .iter()
        .filter_map(|test| match test {
            Value::String(id) => Some(id.clone()),
            Value::Object(_) if text(test, "compared").is_empty() => Some(text(test, "id")),
            _ => None,
        })
        .filter(|id| !id.is_empty())
        .map(|id| own(&id))
        .collect()
}

/// What `setup`'s contenders come to, or what is wrong with it.
pub fn ways(setup: &Setup) -> Result<Ways, String> {
    if setup.item.is_empty() || setup.contest.is_empty() {
        return Err(
            "a contest's contenders are worked out for an item and a contest, and one of them is not said"
                .into(),
        );
    }
    // The incumbent is a contender, the first where the contenders leave it out.
    let bare = |word: &str| word.trim_start_matches('@').to_string();
    let mut contenders = setup.contenders.clone();
    let shadow = setup.shadow.trim();
    if !shadow.is_empty() && !contenders.iter().any(|one| one == shadow || bare(one) == bare(shadow)) {
        contenders.insert(0, shadow.to_string());
    }
    let many = contenders.len();
    if many < 2 {
        let said = match contenders.first() {
            Some(one) => format!("one, `{one}`"),
            None => "none".to_string(),
        };
        return wrong(format!(
            "a contest has two contenders or more, to be judged one against another, and this one has {said}; \
             add the contenders it is to try"
        ));
    }
    let comparison = if setup.compared.is_empty() { own(&setup.contest) } else { setup.compared.clone() };
    if !word(&comparison) {
        return wrong(format!(
            "the comparison it is counted under would be called `{comparison}`, which cannot be what a \
             comparison is called (a word of lowercase letters, digits, `-` and `_`, of forty at most); name \
             one in `compared`"
        ));
    }
    let roles: BTreeSet<&str> = setup.members.iter().map(|one| one.role.as_str()).collect();
    let names: BTreeSet<&str> = setup.members.iter().map(|one| one.name.as_str()).collect();
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();
    for written in &contenders {
        let (way, hands) = if let Some(name) = written.strip_prefix('@') {
            if !setup.members.is_empty() && !names.contains(name) {
                return wrong(format!("the contender `{written}` is no agent of the fleet's"));
            }
            (name.to_string(), Some(("agent", name.to_string())))
        } else if setup.known.contains(written) {
            (written.clone(), None)
        } else {
            if !setup.members.is_empty() && !roles.contains(written.as_str()) {
                let known = match setup.known.is_empty() {
                    true => String::new(),
                    false => format!(" nor a way of `{comparison}`"),
                };
                return wrong(format!(
                    "the contender `{written}` is no role of the fleet's{known}; write a role, an agent as \
                     `@` its name, or a way the comparison has"
                ));
            }
            (written.clone(), Some(("role", written.clone())))
        };
        if !word(&way) {
            return wrong(format!(
                "the contender `{written}` cannot be a way of a comparison, which is called a word of \
                 lowercase letters, digits, `-` and `_`, of forty at most"
            ));
        }
        if !seen.insert(way.clone()) {
            return wrong(format!("the contender `{written}` is written twice; each is tried once"));
        }
        let key = format!("{}--{way}", setup.item);
        let name = format!("{}-{way}", setup.contest);
        let incumbent = !shadow.is_empty() && (written == shadow || bare(written) == bare(shadow));
        let over = setup.rows.iter().any(|(row, state)| *row == name && OVER.contains(&state.as_str()));
        entries.push(Entry {
            written: written.clone(),
            way,
            hands,
            key,
            name,
            incumbent,
            over,
            ..Default::default()
        });
    }
    // Who judges may not be all of those who work.
    let workers: BTreeSet<String> =
        contenders.iter().flat_map(|one| agents_of(one, &setup.members)).collect();
    if !setup.judges.is_empty() {
        let judges = agents_of(&setup.judges, &setup.members);
        let clash = contenders.contains(&setup.judges)
            || (!judges.is_empty() && judges.iter().all(|judge| workers.contains(judge)));
        if clash {
            return wrong(format!(
                "every agent `{}` judges by works an entry, and nobody judges their own work; name judges \
                 that are none of the contenders",
                setup.judges
            ));
        }
    }
    // The order a judge is shown them in, and the letter each is picked by.
    let from = turned(&setup.item, many);
    entries.rotate_left(from);
    let total = if setup.patch > 0 { setup.patch } else { ALL_PATCH };
    let each = (total / many as u64).clamp(LEAST_PATCH, MOST_PATCH);
    for (at, entry) in entries.iter_mut().enumerate() {
        entry.letter = if setup.blind { letter(at) } else { entry.way.clone() };
        // Where judging is blind, a place is called after its letter: a
        // judge reads the places, and one called after its way says whose.
        if setup.blind {
            entry.key = format!("{}--{}", setup.item, entry.letter.to_lowercase());
        }
    }
    let places: Vec<String> = entries.iter().map(|one| format!(".lanes/{}", one.key)).collect();
    for (at, entry) in entries.iter_mut().enumerate() {
        entry.patch = each;
        entry.hidden = places.iter().filter(|place| **place != places[at]).cloned().collect();
    }
    let incumbent = entries.iter().find(|one| one.incumbent).map(|one| one.way.clone()).unwrap_or_default();
    let not = workers.into_iter().collect();
    Ok(Ways { came: "ready", why: String::new(), comparison, entries, not, incumbent })
}

/// Adds `contest.ways` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "contest.ways",
            "Say what a contest's contenders come to: the ways of its comparison, the letter a judge picks each by, where each works and how much of its change a judge is shown, or what is wrong with it.",
        )
        .deciding()
        .required("item", T::String, "the item contested")
        .required("contest", T::String, "the contest's id")
        .required("contenders", T::List, "the contenders as written: each a role, an agent as `@` its name, or a way of the comparison")
        .arg("compared", T::String, "the comparison it is counted under; else one of its own, `ab-<contest>`")
        .arg("known", T::List, "the ways that comparison has already")
        .arg("members", T::List, "the fleet's agents, each with its `name` and `role`; contenders are held to them where given")
        .arg("judges", T::String, "who judges: a role, or an agent as `@` its name")
        .arg("blind", T::Bool, "whether a judge knows the entries by letters and not by their ways; true if not said")
        .arg("patch", T::Int, "how many bytes of what was changed a judge is shown of every entry together (128 KiB if not said)")
        .arg("rows", T::List, "the entries' rows as the table has them, each its `title` and `state`: one whose work is over is not started again")
        .arg("shadow", T::String, "the incumbent of a shadow test, as a contender is written: one of the contenders, the first where they leave it out; nothing for a test of equals")
        .arg("finish", T::List, "how a contender's work is finished where it is worked (`finish` of `std/turn`)")
        .arg("later", T::List, "with `finish`: what somebody else checks after it")
        .arg("lasts", T::String, "how long a contender's turn may take, as `45m`")
        .output("finishing", "what each contender's work is handed of how it is finished, `finish`, `later` and `lasts`, each only where it is said: a way that is an earlier text taking none of it is handed none")
        .output("came", "`ready`, or `wrong`")
        .output("why", "what is wrong, where it is")
        .output("comparison", "the comparison it is counted under")
        .output("ways", "the ways to ask of the comparison as `compare.start` takes them, each its `name` and what it hands (`with`); none it has already")
        .output("entries", "each entry in the order a judge is shown them: `way`, `letter`, `key` (its place), `name` (its row), `as` (its child), `patch` (bytes), `hidden` (the other places), `with`")
        .output("not", "the agents that work an entry, which may not judge")
        .output("incumbent", "the incumbent's way, where it is a shadow test; nothing otherwise")
        .output("judge_slot", "the agent who judges, where `judges` names one by `@`; nothing otherwise")
        .output("judge_role", "the role whose agents judge, where `judges` names one; nothing otherwise")
        .output("starts", "the entries still to start, as `entries` says each: all but those whose row says their work is over")
        .output("children", "what each entry is started as (`as`), in the order of `entries`: what is waited for together")
        .output("rows", "a row of the entries' table for each, as `rows.start` takes them: `title` (its name), `contest`, `card`, `way`, `letter`, `key`, `state`, and `incumbent` on the incumbent's")
        .idempotent(),
        |_, args| async move {
            let words = |key: &str| -> Vec<String> {
                let each = args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
                each.iter().filter_map(|one| one.as_str().map(str::to_string)).collect()
            };
            let members = args.get("members").and_then(Value::as_array).cloned().unwrap_or_default();
            let setup = Setup {
                item: said(&args, "item"),
                contest: said(&args, "contest"),
                contenders: words("contenders").into_iter().filter(|one| !one.is_empty()).collect(),
                compared: said(&args, "compared"),
                known: words("known"),
                members: members.iter().map(|one| Member { name: text(one, "name"), role: text(one, "role") }).collect(),
                judges: said(&args, "judges"),
                blind: args.get("blind").and_then(Value::as_bool).unwrap_or(true),
                patch: count(&args, "patch").max(0) as u64,
                shadow: said(&args, "shadow"),
                rows: args
                    .get("rows")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|row| (text(row, "title"), text(row, "state")))
                    .collect(),
            };
            let came = ways(&setup).map_err(ActionError::new)?;
            let with = |entry: &Entry| match &entry.hands {
                Some((key, value)) => json!({ *key: value }),
                None => json!({}),
            };
            // A way the comparison has already is said there, and not again.
            let ways: Vec<Value> = came
                .entries
                .iter()
                .filter(|one| one.hands.is_some())
                .map(|one| json!({ "name": one.way, "with": with(one) }))
                .collect();
            let entries: Vec<Value> = came
                .entries
                .iter()
                .map(|one| {
                    json!({
                        "way": one.way, "letter": one.letter, "key": one.key, "name": one.name,
                        "as": format!("entry-{}", one.way), "patch": one.patch, "hidden": one.hidden, "with": with(one),
                    })
                })
                .collect();
            let starts: Vec<Value> =
                came.entries.iter().zip(&entries).filter(|(one, _)| !one.over).map(|(_, value)| value.clone()).collect();
            let children: Vec<String> = came.entries.iter().map(|one| format!("entry-{}", one.way)).collect();
            let rows: Vec<Value> = came
                .entries
                .iter()
                .map(|one| {
                    let mut row = json!({
                        "title": one.name, "contest": setup.contest, "card": setup.item, "way": one.way,
                        "letter": one.letter, "key": one.key, "state": "working",
                    });
                    if one.incumbent {
                        row["incumbent"] = json!(true);
                    }
                    row
                })
                .collect();
            let listed = |key: &str| -> Vec<Value> { args.get(key).and_then(Value::as_array).cloned().unwrap_or_default() };
            let finishing = finishing(&listed("finish"), &listed("later"), &said(&args, "lasts"));
            Ok(json!({
                "finishing": finishing,
                "came": came.came, "why": came.why, "comparison": came.comparison, "ways": ways,
                "entries": entries, "starts": starts, "not": came.not, "children": children, "rows": rows, "incumbent": came.incumbent,
                "judge_slot": setup.judges.strip_prefix('@').unwrap_or_default(),
                "judge_role": if setup.judges.starts_with('@') { "" } else { setup.judges.as_str() },
            }))
        },
    );

    add(
        registry,
        ActionSpec::new(
            "contest.own",
            "Say the comparisons of their own that contests are counted under, those that name none.",
        )
        .deciding()
        .required(
            "tests",
            T::List,
            "the contests: each its row (`id`, and `compared` where it names one), or its id alone",
        )
        .output("own", "what each of those that names no comparison is counted under, in their order")
        .idempotent(),
        |_, args| async move {
            let tests = args.get("tests").and_then(Value::as_array).cloned().unwrap_or_default();
            Ok(json!({ "own": owned(&tests) }))
        },
    );
}
