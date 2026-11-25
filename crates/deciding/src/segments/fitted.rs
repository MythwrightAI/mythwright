//! What a segment is handed: what its row says and what whoever asks
//! says of itself and of this going through the list, by what the
//! segment declares; and what something that is begun and heard from is
//! begun with.

use blueprint::ValueType;
use serde_json::{Map, Value};

use super::{ALWAYS, CELLS, Run, TIMES};

/// What whoever asks says of itself that is how its list is gone
/// through, and nothing a segment is handed: how many things at most
/// are done in one going through (`most` of a stage's row, which a
/// stage hands whole as what it says of itself) would otherwise be
/// handed to a segment that counts something else by that name, as
/// `std/keep` counts how often its card is given again.
pub const NOT_HANDED: &[&str] = &["most"];

/// What is so of this going through the list as a segment is handed
/// it: what whoever asks said of it, and where that says nothing of them,
/// the labels the item holds and the column it is in (`here`). A list a
/// looking that passed goes on through says the column itself; one gone
/// through as the item came said none, and the merge after a row laid in
/// the looking's place could not be called.
pub fn coming(journey: &Map<String, Value>, labels: &[String], here: &str) -> Map<String, Value> {
    let mut coming = journey.clone();
    coming.entry("labels").or_insert_with(|| Value::from(labels.to_vec()));
    coming.entry("here").or_insert_with(|| Value::from(here));
    coming
}

/// A value said as text, as the kind of thing a param is.
pub fn as_kind(name: &str, said: &Value, kind: ValueType, title: &str) -> Result<Value, String> {
    let Value::String(text) = said else { return Ok(said.clone()) };
    let text = text.trim();
    let not = |what: &str| format!("{title} says `{name}` is `{text}`, and it is {what}");
    match kind {
        ValueType::Int => text.parse::<i64>().map(Value::from).map_err(|_| not("a whole number")),
        ValueType::Number => text.parse::<f64>().map(Value::from).map_err(|_| not("a number")),
        ValueType::Bool => match text {
            "true" | "yes" | "on" => Ok(Value::Bool(true)),
            "false" | "no" | "off" => Ok(Value::Bool(false)),
            _ => Err(not("`true` or `false`")),
        },
        ValueType::List => {
            Ok(Value::from(text.split(',').map(str::trim).filter(|one| !one.is_empty()).collect::<Vec<_>>()))
        }
        _ => Ok(Value::from(text)),
    }
}

/// What a segment is handed beside the item: what its row says, cell by
/// cell and in `with`, each of which the segment has to declare; and of
/// what whoever asks says of itself (`own`) and of this going through
/// the list (`journey`), whatever the segment declares and its row left
/// unsaid. A row that says something its segment has no param for is
/// refused in a sentence.
pub fn fitted(
    run: &Run,
    own: &Map<String, Value>,
    journey: &Map<String, Value>,
    declared: &[(String, ValueType)],
) -> Result<Map<String, Value>, String> {
    let kind = |name: &str| declared.iter().find(|(known, _)| known == name).map(|(_, kind)| *kind);
    let takes = || {
        let mut names: Vec<&str> =
            declared.iter().map(|(name, _)| name.as_str()).filter(|name| !ALWAYS.contains(name)).collect();
        names.sort_unstable();
        match names.is_empty() {
            true => "it takes nothing more than the item".to_string(),
            false => format!("what it takes is {}", names.join(", ")),
        }
    };
    let (title, text) = (&run.title, &run.name);
    let mut with = Map::new();
    // Each with whether it was said in `with`, beside the row's own cells.
    let mut said: Vec<(String, Value, bool)> =
        run.cells.iter().map(|(name, said)| (name.clone(), said.clone(), false)).collect();
    for entry in &run.with {
        let Some((name, value)) = entry.split_once('=') else {
            return Err(format!("{title} says `{entry}` in `with`, where each thing is `name=value`"));
        };
        said.push((name.trim().to_string(), Value::from(value.trim()), true));
    }
    for (name, value, in_with) in said {
        let Some(kind) = kind(&name).filter(|_| !ALWAYS.contains(&name.as_str())) else {
            return Err(format!(
                "{title} says `{name}`, and `{text}` takes nothing called that: {}",
                takes()
            ));
        };
        // What `with` says is anything more the blueprint is handed. A
        // cell of the row is said in its cell, where what writing it takes
        // is asked; and what whoever asks says of itself is said where it
        // is (the stage's row, what the fleet was told: the secret its
        // remote is asked with), by whoever may say it there.
        if in_with && CELLS.contains(&name.as_str()) {
            return Err(format!("{title} says `{name}` in `with`, which is a cell of its own: say it there"));
        }
        if in_with && own.contains_key(&name) && !NOT_HANDED.contains(&name.as_str()) {
            return Err(format!(
                "{title} says `{name}` in `with`, which its stage says of itself: a row says what more its own \
                 blueprint is handed, and not what its stage, or the fleet over it, says"
            ));
        }
        if TIMES.contains(&name.as_str()) {
            let said = blueprint::expr::display(&value);
            blueprint::duration::parse(&said).map_err(|_| {
                format!("{title} says `{name}` is `{said}`, which is no time: write it like `20m` or `1h30m`")
            })?;
        }
        with.insert(name.clone(), as_kind(&name, &value, kind, title)?);
    }
    // What is so of this going through comes before what whoever asks
    // says of itself: both may say something by one name (what an item
    // is called, and what whoever has its column is called).
    // A list is handed to a param that is text as its things joined by a
    // comma, as a row's text is read as a list the other way round, and a
    // number as its words.
    let own = own.iter().filter(|(name, _)| !NOT_HANDED.contains(&name.as_str()));
    for (name, value) in journey.iter().chain(own) {
        if let Some(kind) =
            kind(name).filter(|_| !ALWAYS.contains(&name.as_str()) && !with.contains_key(name))
        {
            let value = match (kind, value) {
                (ValueType::String, Value::Array(all)) => {
                    Value::from(all.iter().map(blueprint::expr::display).collect::<Vec<_>>().join(","))
                }
                // And a number as its words: which coming of an item it is,
                // which a stage says by the number of what brought it.
                (ValueType::String, Value::Number(number)) => Value::from(number.to_string()),
                _ => value.clone(),
            };
            with.insert(name.clone(), value);
        }
    }
    Ok(with)
}

/// What something that is begun is told of a restart, where it is told.
const RESTARTING: &[&str] = &["after_restart", "restarts"];

/// The cells a row may say of something that is begun and heard from.
pub const COUNCIL: &[&str] = &["role", "picks", "seats", "needed"];

/// What something that is begun and heard from is begun with: who looks
/// and how many have to approve, as its row says, and as whoever asks
/// says of itself (`own`) where the row says nothing. A row that says
/// anything else of it is refused in a sentence.
pub fn council(run: &Run, own: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    if let Some(other) = run.cells.keys().find(|cell| !COUNCIL.contains(&cell.as_str())) {
        return Err(format!(
            "the row {} names {}, which is begun and heard from, and says `{other}`: what a row says of it is \
             who looks (`role`, `picks`) and how many (`seats`, `needed`)",
            run.title, run.name
        ));
    }
    if let Some(entry) = run.with.first() {
        return Err(format!(
            "the row {} names {}, which is begun and heard from, and says `{entry}` in `with`: what a row says \
             of it is who looks (`role`, `picks`) and how many (`seats`, `needed`)",
            run.title, run.name
        ));
    }
    let mut with = Map::new();
    for cell in COUNCIL {
        let Some(said) = run.cells.get(*cell).or_else(|| own.get(*cell)) else { continue };
        let kind = if matches!(*cell, "seats" | "needed") { ValueType::Int } else { ValueType::String };
        with.insert(cell.to_string(), as_kind(cell, said, kind, &run.title)?);
    }
    // And what becomes of work in hand at a restart, where whoever asks
    // says that what it begins is told so (`restarting`): one of its own
    // making may take nothing of the kind.
    if own.get("restarting") == Some(&Value::Bool(true)) {
        for word in RESTARTING {
            if let Some(said) = own.get(*word) {
                with.insert(word.to_string(), said.clone());
            }
        }
    }
    Ok(with)
}
