//! Tickets that are things there already: what a plan may take up of
//! what was written before it, whether it fits, and what each thing then
//! waits on.
//!
//! A plan's tickets usually become new things, one each
//! (`deciding::tickets`). Work that was written up before anybody planned
//! it is rows there already, and a plan may put those in order too: a
//! ticket that says `card` is that row, and says what it waits on as any
//! ticket does, by a place in the list or by the id of a row. Whoever asks
//! for the plan says which rows it gathers; the plan may be given only
//! those.
//!
//! Three things are worked out here, each a function of what it is handed
//! and none of them reading or writing anything: the words that show the
//! gathered rows to whoever plans ([`named`]); whether a plan fits what is
//! there, asked before anything is written, so that a plan is taken whole
//! or not at all ([`fit`]); and, once the tickets that are new have come
//! to rows of their own, what every one of them waits on and is part of
//! ([`placed`]). A row is read by the cells its table keeps these in,
//! which whoever asks names; nothing here knows what a row is of.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

/// The cells of a row that these read, by what each is called in its
/// table, and the states that mean something here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cells {
    /// What it is called, and what is wanted.
    pub title: String,
    pub body: String,
    /// The ids of the rows it waits on.
    pub waits: String,
    /// What it is part of.
    pub within: String,
    /// Where it is, and its labels.
    pub state: String,
    pub labels: String,
    /// The state of one that is done: nothing waits on such a row, and it
    /// is left where it is.
    pub over: String,
    /// The state rows begin in: only such rows are made ready.
    pub begins: String,
}

impl Default for Cells {
    fn default() -> Cells {
        let word = str::to_string;
        Cells {
            title: word("title"),
            body: word("body"),
            waits: word("after"),
            within: word("part_of"),
            state: word("status"),
            labels: word("labels"),
            over: String::new(),
            begins: String::new(),
        }
    }
}

/// A row there already, as its cells say it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Thing {
    pub id: String,
    pub title: String,
    pub body: String,
    pub state: String,
    pub waits: Vec<String>,
    pub within: String,
    pub labels: Vec<String>,
}

/// The words a cell holds, whether it keeps them as a list or as one
/// text; nothing, for a cell that says nothing.
pub fn words(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => {
            items.iter().filter_map(Value::as_str).filter(|one| !one.is_empty()).map(str::to_string).collect()
        }
        Some(Value::String(one)) if !one.is_empty() => vec![one.clone()],
        _ => Vec::new(),
    }
}

/// The rows of a table as these read them. One with no `id` is no row.
pub fn things(rows: &[Value], cells: &Cells) -> Vec<Thing> {
    let text =
        |row: &Value, cell: &str| row.get(cell).and_then(Value::as_str).unwrap_or_default().to_string();
    rows.iter()
        .filter(|row| row.get("id").and_then(Value::as_str).is_some_and(|id| !id.is_empty()))
        .map(|row| Thing {
            id: text(row, "id"),
            title: text(row, &cells.title),
            body: text(row, &cells.body),
            state: text(row, &cells.state),
            waits: words(row.get(&cells.waits)),
            within: text(row, &cells.within),
            labels: words(row.get(&cells.labels)),
        })
        .collect()
}

/// Names as a sentence lists them: `a`, `a and b`, `a, b and c`.
pub fn listed(names: &[String]) -> String {
    match names.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// The rows a plan is to put in order, said for whoever plans.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Named {
    /// Those that are there, in the order they were named, and after them
    /// those that carry the label.
    pub ids: Vec<String>,
    /// Those named that are no row of the table.
    pub missing: Vec<String>,
    /// Each of them in words: its id, where it is, what it is called and
    /// what is wanted, what it waits on and what it is part of.
    pub shown: String,
    /// Whether the label brought any that were not named: then what is
    /// named is to be written down, so that the group is the same from
    /// then on.
    pub fix: bool,
}

/// The rows `ids` name and, with a `label`, every row that carries it and
/// is not done, as whoever plans is shown them.
pub fn named(rows: &[Thing], ids: &[String], label: &str, cells: &Cells) -> Named {
    let by_id: BTreeMap<&str, &Thing> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut answer = Named::default();
    for id in ids {
        match by_id.contains_key(id.as_str()) {
            true if !answer.ids.contains(id) => answer.ids.push(id.clone()),
            true => {}
            false if !answer.missing.contains(id) => answer.missing.push(id.clone()),
            false => {}
        }
    }
    let asked = answer.ids.len();
    if !label.is_empty() {
        let done = |row: &Thing| !cells.over.is_empty() && row.state == cells.over;
        for row in rows.iter().filter(|row| row.labels.iter().any(|one| one == label) && !done(row)) {
            if !answer.ids.contains(&row.id) {
                answer.ids.push(row.id.clone());
            }
        }
    }
    answer.fix = answer.ids.len() > asked;
    let mut said: Vec<String> = Vec::new();
    for id in &answer.ids {
        let row = by_id[id.as_str()];
        let mut lines = vec![match row.state.is_empty() {
            true => format!("{}: {}", row.id, row.title),
            false => format!("{}, in {}: {}", row.id, row.state, row.title),
        }];
        if !row.body.trim().is_empty() {
            lines.push(row.body.trim().to_string());
        }
        let mut more = Vec::new();
        if !row.waits.is_empty() {
            more.push(format!("It waits on {}.", listed(&row.waits)));
        }
        if !row.within.is_empty() {
            more.push(format!("It is part of {}.", row.within));
        }
        if !more.is_empty() {
            lines.push(more.join(" "));
        }
        said.push(lines.join("\n"));
    }
    for id in &answer.missing {
        said.push(format!("{id} was named, and there is no such thing."));
    }
    answer.shown = said.join("\n\n");
    answer
}

/// Whether a plan fits what is there.
///
/// Written out as the language's `came!` writes the words of a block
/// (`ALL`, `word`, `words`): the kernel stands on nothing here, the
/// language included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// It does: its tickets can be taken as they are written.
    Fits,
    /// It does not, and nothing of it is to be written.
    Unfit,
    /// It lists no tickets and gathers nothing: it is one piece of
    /// work, whole.
    Whole,
}

impl Fit {
    /// Every word it can come to, in the order they are declared.
    pub const ALL: &'static [Fit] = &[Fit::Fits, Fit::Unfit, Fit::Whole];

    /// The word a blueprint reads.
    pub const fn word(self) -> &'static str {
        match self {
            Fit::Fits => "fits",
            Fit::Unfit => "unfit",
            Fit::Whole => "whole",
        }
    }

    /// The words, as a sentence lists them: `` `a`, `b` or `c` ``.
    pub fn words() -> String {
        let words: Vec<String> = Self::ALL.iter().map(|one| format!("`{}`", one.word())).collect();
        match words.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} or {last}", rest.join(", ")),
            _ => words.concat(),
        }
    }
}

/// A plan held against what is there.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitted {
    pub came: Fit,
    /// What is wrong with it, a sentence for each thing; or, for a plan
    /// that is whole because its block could not be read, why not.
    pub why: String,
    /// The tickets that are new, in the order they are listed, each with
    /// its `title` and `body`: a row is to be made for each.
    pub new: Vec<Value>,
}

/// What a ticket is, or a gathered row no ticket names.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    /// A row there already, by its id.
    There(String),
    /// A ticket that is new, by its place in the list, counted from 1.
    New(usize),
}

impl Key {
    fn said(&self) -> String {
        match self {
            Key::There(id) => id.clone(),
            Key::New(place) => format!("ticket {place}"),
        }
    }
}

/// One thing of a plan: a ticket, or a gathered row the plan leaves as it is.
#[derive(Clone, Debug)]
struct Entry {
    key: Key,
    /// What it waits on once the plan is taken.
    waits: Vec<Key>,
    /// What it waited on before, for a row there already.
    was: Vec<String>,
}

/// A plan worked out against what is there: its entries in order, the
/// tickets first; or what is wrong with it.
fn resolve(
    tickets: &[Value],
    may: &[String],
    rows: &[Thing],
    of: &str,
    cells: &Cells,
) -> Result<Vec<Entry>, Vec<String>> {
    let by_id: BTreeMap<&str, &Thing> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let done = |id: &str| by_id.get(id).is_some_and(|row| !cells.over.is_empty() && row.state == cells.over);
    let mut wrong: Vec<String> = Vec::new();
    // The rows tickets name outright, which no ticket comes by any other way.
    let outright: BTreeSet<&str> =
        tickets.iter().filter_map(|ticket| ticket.get("card").and_then(Value::as_str)).collect();
    // What each ticket is.
    let mut keys: Vec<Key> = Vec::new();
    for (at, ticket) in tickets.iter().enumerate() {
        let place = at + 1;
        let key = match ticket.get("card") {
            None | Some(Value::Null) => {
                let title = ticket.get("title").and_then(Value::as_str).map(str::trim).unwrap_or_default();
                if title.is_empty() {
                    wrong.push(format!(
                        "Ticket {place} names nothing there already and has no `title`: it is one or the other."
                    ));
                }
                // Made before: a row that is part of this already and is
                // called what the ticket is. A plan taken up a second
                // time, having been cut short the first, makes nothing
                // twice.
                let made = rows.iter().find(|row| {
                    !title.is_empty()
                        && row.within == of
                        && row.title.trim() == title
                        && !outright.contains(row.id.as_str())
                        && !keys.contains(&Key::There(row.id.clone()))
                });
                match made {
                    Some(row) => Key::There(row.id.clone()),
                    None => Key::New(place),
                }
            }
            Some(Value::String(id)) => {
                if !may.contains(id) {
                    wrong.push(format!(
                        "Ticket {place} is {id}, which is not one of those this was asked to put in order{}.",
                        match may.is_empty() {
                            true => String::new(),
                            false => format!(" ({})", listed(may)),
                        }
                    ));
                } else if !by_id.contains_key(id.as_str()) {
                    wrong.push(format!("Ticket {place} is {id}, and there is no such thing any more."));
                } else if keys.contains(&Key::There(id.clone())) {
                    wrong.push(format!("{id} is named by two tickets, and is one thing."));
                }
                Key::There(id.clone())
            }
            Some(other) => {
                wrong.push(format!(
                    "`card` of ticket {place} is the id of something there already (\"shop-3\"); it is {other}."
                ));
                Key::New(place)
            }
        };
        keys.push(key);
    }
    // Those gathered that no ticket names are taken as they stand, and so
    // is whatever is part of this already and no ticket came to.
    let named: BTreeSet<&Key> = keys.iter().collect();
    let mut unnamed: Vec<Key> = Vec::new();
    let theirs = rows.iter().filter(|row| row.within == of).map(|row| &row.id);
    for id in may.iter().filter(|id| by_id.contains_key(id.as_str())).chain(theirs) {
        let key = Key::There(id.clone());
        if !named.contains(&key) && !unnamed.contains(&key) {
            unnamed.push(key);
        }
    }
    // The rows of this plan, which what a ticket says of its waiting is
    // the whole of: what a row waited on outside them is kept beside it.
    let inside: BTreeSet<String> = keys
        .iter()
        .chain(&unnamed)
        .filter_map(|key| match key {
            Key::There(id) => Some(id.clone()),
            Key::New(_) => None,
        })
        .collect();
    let mut entries: Vec<Entry> = Vec::new();
    for (at, ticket) in tickets.iter().enumerate() {
        let place = at + 1;
        let was: Vec<String> = match &keys[at] {
            Key::There(id) => by_id.get(id.as_str()).map(|row| row.waits.clone()).unwrap_or_default(),
            Key::New(_) => Vec::new(),
        };
        let waits: Vec<Key> = match ticket.get("after") {
            // It says nothing of what it waits on: a row is left as it was.
            None | Some(Value::Null) => was.iter().map(|id| Key::There(id.clone())).collect(),
            Some(Value::Array(after)) => {
                let mut waits: Vec<Key> = Vec::new();
                for one in after {
                    let key = match one {
                        Value::Number(number) => {
                            match number
                                .as_u64()
                                .and_then(|n| usize::try_from(n).ok())
                                .filter(|n| (1..=keys.len()).contains(n))
                            {
                                Some(n) => Some(keys[n - 1].clone()),
                                None => {
                                    wrong.push(format!(
                                        "Ticket {place} waits on ticket {number}, and the tickets are 1 to {}.",
                                        keys.len()
                                    ));
                                    None
                                }
                            }
                        }
                        Value::String(id) if by_id.contains_key(id.as_str()) => Some(Key::There(id.clone())),
                        Value::String(id) => {
                            wrong.push(format!("Ticket {place} waits on {id}, and there is no such thing."));
                            None
                        }
                        other => {
                            wrong.push(format!(
                                "`after` of ticket {place} lists what it waits on, each a place in the list (1) or \
                                 the id of something there already (\"shop-3\"); one of them is {other}."
                            ));
                            None
                        }
                    };
                    if let Some(key) = key.filter(|key| !waits.contains(key)) {
                        waits.push(key);
                    }
                }
                // What a row waited on outside this plan is kept.
                for id in was.iter().filter(|id| !inside.contains(*id)) {
                    let key = Key::There(id.clone());
                    if !waits.contains(&key) {
                        waits.push(key);
                    }
                }
                waits
            }
            Some(other) => {
                wrong.push(format!(
                    "`after` of ticket {place} is the list of what it waits on ([1, \"shop-3\"]); it is {other}."
                ));
                Vec::new()
            }
        };
        entries.push(Entry { key: keys[at].clone(), waits, was });
    }
    for key in unnamed {
        let Key::There(id) = &key else { continue };
        let was = by_id[id.as_str()].waits.clone();
        let waits = was.iter().map(|id| Key::There(id.clone())).collect();
        entries.push(Entry { key, waits, was });
    }
    // Nothing waits on what is done, a row whose ticket said what it
    // waits on least of all; one left as it was is left exactly so.
    for (at, entry) in entries.iter_mut().enumerate() {
        let said =
            tickets.get(at).is_some_and(|ticket| ticket.get("after").is_some_and(|after| !after.is_null()));
        if said {
            entry.waits.retain(|key| !matches!(key, Key::There(id) if done(id)));
        }
    }
    // Part of something else already: said, and not taken.
    for entry in &entries {
        let Key::There(id) = &entry.key else { continue };
        let within = by_id.get(id.as_str()).map(|row| row.within.as_str()).unwrap_or_default();
        if !within.is_empty() && within != of {
            wrong.push(format!(
                "{id} is part of {within} already: leave it out of this, or take it out of that first."
            ));
        }
    }
    // Nothing waits on itself, by any road: through tickets, through rows
    // of this plan and through what rows outside it already wait on.
    let mut roads: BTreeMap<Key, Vec<Key>> = BTreeMap::new();
    for row in rows.iter().filter(|row| !done(&row.id)) {
        let waits = row.waits.iter().filter(|id| !done(id)).map(|id| Key::There(id.clone())).collect();
        roads.insert(Key::There(row.id.clone()), waits);
    }
    for entry in &entries {
        let waits =
            entry.waits.iter().filter(|key| !matches!(key, Key::There(id) if done(id))).cloned().collect();
        roads.insert(entry.key.clone(), waits);
    }
    let mut told: BTreeSet<Key> = BTreeSet::new();
    for entry in &entries {
        if let Some(road) = round(&entry.key, &roads) {
            // One sentence for a ring, whichever of its things it is found from.
            if road.iter().any(|key| told.contains(key)) {
                continue;
            }
            told.extend(road.iter().cloned());
            // "a waits on b, which waits on c, which waits on a".
            let rest: Vec<String> = road[1..].iter().map(Key::said).collect();
            wrong.push(format!(
                "{} would wait on itself: {} waits on {}.",
                capital(&road[0].said()),
                road[0].said(),
                rest.join(", which waits on ")
            ));
        }
    }
    match wrong.is_empty() {
        true => Ok(entries),
        false => Err(wrong),
    }
}

fn capital(words: &str) -> String {
    let mut letters = words.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => String::new(),
    }
}

/// The road from `from` back to itself, where there is one: `from`, what
/// it waits on that leads back, and so on, ending with `from` again.
fn round(from: &Key, roads: &BTreeMap<Key, Vec<Key>>) -> Option<Vec<Key>> {
    fn walk(
        at: &Key,
        from: &Key,
        roads: &BTreeMap<Key, Vec<Key>>,
        seen: &mut BTreeSet<Key>,
        road: &mut Vec<Key>,
    ) -> bool {
        for next in roads.get(at).map(Vec::as_slice).unwrap_or_default() {
            if next == from {
                road.push(next.clone());
                return true;
            }
            if seen.insert(next.clone()) {
                road.push(next.clone());
                if walk(next, from, roads, seen, road) {
                    return true;
                }
                road.pop();
            }
        }
        false
    }
    let mut road = vec![from.clone()];
    walk(from, from, roads, &mut BTreeSet::new(), &mut road).then_some(road)
}

/// Whether a plan's tickets fit what is there. `read` is what reading the
/// plan for its tickets came to (`deciding::tickets::Came`, as a word) and
/// `unread` why they could not be read; `may` the rows it was asked to
/// put in order, and `of` what they are to be part of.
pub fn fit(
    read: &str,
    unread: &str,
    tickets: &[Value],
    may: &[String],
    rows: &[Thing],
    of: &str,
    cells: &Cells,
) -> Fitted {
    let came = |came: Fit, why: String, new: Vec<Value>| Fitted { came, why, new };
    if read == "unread" {
        return match may.is_empty() {
            true => came(Fit::Whole, unread.to_string(), Vec::new()),
            false => came(
                Fit::Unfit,
                format!("The tickets its plan ends with could not be read ({unread})."),
                Vec::new(),
            ),
        };
    }
    // What is part of this already: something came of it before.
    let begun = rows.iter().any(|row| row.within == of);
    if tickets.is_empty() && may.is_empty() && !begun {
        return came(Fit::Whole, String::new(), Vec::new());
    }
    // It gathers rows, and none of them is there any more: nothing would
    // come of it.
    if tickets.is_empty() && !begun && !may.iter().any(|id| rows.iter().any(|row| &row.id == id)) {
        return came(
            Fit::Unfit,
            format!(
                "It lists no tickets, and of what it gathers ({}) nothing is there any more.",
                listed(may)
            ),
            Vec::new(),
        );
    }
    match resolve(tickets, may, rows, of, cells) {
        Err(wrong) => came(Fit::Unfit, wrong.join(" "), Vec::new()),
        Ok(entries) => {
            let new = entries
                .iter()
                .filter_map(|entry| match entry.key {
                    Key::New(place) => tickets.get(place - 1),
                    Key::There(_) => None,
                })
                .map(|ticket| {
                    json!({
                        "title": ticket["title"].as_str().unwrap_or_default().trim(),
                        "body": ticket.get("body").and_then(Value::as_str).unwrap_or_default(),
                    })
                })
                .collect();
            came(Fit::Fits, String::new(), new)
        }
    }
}

/// What a plan's tickets came to, once those that are new are rows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Placed {
    /// For each ticket in turn, and then each gathered row no ticket
    /// named: its `id`, `after` (the ids of what it waits on) and what it
    /// is part of, under the name of the cell that keeps that.
    pub links: Vec<Value>,
    /// The ids, in that order.
    pub ids: Vec<String>,
    /// Those to be made ready: the new ones, and of those there already
    /// the ones still where rows begin.
    pub ready: Vec<String>,
    /// How many are new, and how many were there already.
    pub added: usize,
    pub taken: usize,
    /// What changed for the rows there already, in sentences.
    pub told: String,
}

/// What every thing of a plan waits on and is part of. `made` is what the
/// tickets that are new came to, one each and in their order, each with
/// its `id`. The plan is one that [`fit`]: handed one that does not, this
/// refuses, saying what is wrong.
pub fn placed(
    tickets: &[Value],
    made: &[Value],
    may: &[String],
    rows: &[Thing],
    of: &str,
    cells: &Cells,
) -> Result<Placed, String> {
    let entries = resolve(tickets, may, rows, of, cells).map_err(|wrong| wrong.join(" "))?;
    let new: Vec<usize> = entries
        .iter()
        .filter_map(|entry| match entry.key {
            Key::New(place) => Some(place),
            Key::There(_) => None,
        })
        .collect();
    if new.len() != made.len() {
        return Err(format!(
            "{} of the tickets are new and {} things were made for them: one each, in the same order",
            new.len(),
            made.len()
        ));
    }
    let mut ids_of: BTreeMap<usize, String> = BTreeMap::new();
    for (place, one) in new.iter().zip(made) {
        match one.get("id").and_then(Value::as_str).filter(|id| !id.is_empty()) {
            Some(id) => ids_of.insert(*place, id.to_string()),
            None => return Err(format!("what ticket {place} came to has no `id`: {one}")),
        };
    }
    let id_of = |key: &Key| match key {
        Key::There(id) => id.clone(),
        Key::New(place) => ids_of[place].clone(),
    };
    let by_id: BTreeMap<&str, &Thing> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    let mut answer = Placed::default();
    let mut told: Vec<String> = Vec::new();
    for entry in &entries {
        let id = id_of(&entry.key);
        let after: Vec<String> = entry.waits.iter().map(id_of).collect();
        let mut link = Map::new();
        link.insert("id".into(), json!(id));
        link.insert("after".into(), json!(after));
        link.insert(cells.within.clone(), json!(of));
        answer.links.push(Value::Object(link));
        match &entry.key {
            Key::New(_) => {
                answer.added += 1;
                answer.ready.push(id.clone());
            }
            Key::There(_) => {
                answer.taken += 1;
                let state = by_id.get(id.as_str()).map(|row| row.state.as_str()).unwrap_or_default();
                let begins = cells.begins.is_empty() || state == cells.begins;
                let over = !cells.over.is_empty() && state == cells.over;
                if begins && !over {
                    answer.ready.push(id.clone());
                }
                if after != entry.was {
                    told.push(match after.is_empty() {
                        true => format!("{id} now waits on nothing."),
                        false => format!("{id} now waits on {}.", listed(&after)),
                    });
                    if !begins && !over && !after.is_empty() {
                        told.push(format!("It is in {state} already, and is not held back."));
                    }
                }
            }
        }
        answer.ids.push(id);
    }
    answer.told = told.join(" ");
    Ok(answer)
}
