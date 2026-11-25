//! Two sides of one thing, kept in step.
//!
//! Something that is kept in two places (a row here, and whatever stands
//! for it somewhere else) is changed in either, and what keeps the two in
//! step has to say, each time it looks, what goes which way. That is not
//! worked out from the two as they stand: where they differ, nothing in
//! them says which of them moved. It is worked out against what the two
//! last agreed on, which whoever keeps them in step writes down each time
//! it has brought them together. A side that says what was agreed has not
//! moved; one that says something else has.
//!
//! [`cross`] says that for the fields of the two sides: what goes out,
//! what comes in, and what they agree on once that is done. Where both
//! moved, or nothing was ever agreed, one of them is taken, and which is
//! said by whoever asks and never here. So nothing comes back as an echo:
//! what was sent is what is agreed, and the other side saying it again is
//! a side that has not moved.
//!
//! Whose changes on the other side are taken is said by whoever asks too
//! (`takes`): everyone's; only those of somebody trusted, where the other
//! side as it stands is said to have been written by one or not; or
//! nobody's, so that this side only ever goes out. A change that is not
//! taken for want of trust is held: nothing of these fields crosses
//! either way, so that nothing here is written over what is there and
//! nothing there comes in, until somebody says which of the two stands
//! (`take` or `keep`), which is said here as a word kept beside what the
//! two agreed on.
//!
//! [`owed`] says the same of things that are said and not changed (what
//! was said on one side, each with a name of its own): which of them have
//! not crossed yet, held against the names of those that have.
//!
//! Neither knows what the sides are, or where the other one is.

use serde_json::{Map, Value};

/// A field that says nothing: never said, said as nothing, or emptied.
/// The three are the same nothing on either side.
fn nothing(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.is_empty(),
        Some(_) => false,
    }
}

/// Whether two fields say the same.
fn same(one: Option<&Value>, other: Option<&Value>) -> bool {
    match (nothing(one), nothing(other)) {
        (true, true) => true,
        (false, false) => one == other,
        _ => false,
    }
}

/// Which side is taken where both moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefer {
    Here,
    There,
}

/// Whose changes on the other side are taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    Everyone,
    /// Only where the other side as it stands is said to be trusted.
    Trusted,
    /// Nobody's: this side only goes out.
    Nobody,
}

/// What was said of a change on the other side that is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// Nothing is held.
    Nothing,
    /// A change is held, and nobody has said which side stands.
    Waiting,
    /// Somebody said the other side stands.
    Take,
    /// Somebody said this side stands.
    Keep,
}

/// What looking at two sides came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Came {
    /// Nothing is to cross.
    Same,
    /// Something goes out, and nothing comes in.
    Out,
    /// Something comes in, and nothing goes out.
    In,
    /// Something goes each way.
    Both,
    /// A change on the other side is held, and nothing crosses.
    Held,
}

impl Came {
    pub fn word(self) -> &'static str {
        match self {
            Came::Same => "same",
            Came::Out => "out",
            Came::In => "in",
            Came::Both => "both",
            Came::Held => "held",
        }
    }
}

/// Every word [`cross`] can answer.
pub const CAME: &[&str] = &["same", "out", "in", "both", "held"];

/// What is asked of two sides.
#[derive(Clone, Debug)]
pub struct Sides<'a> {
    pub here: &'a Map<String, Value>,
    pub there: &'a Map<String, Value>,
    /// What the two last agreed on.
    pub last: &'a Map<String, Value>,
    pub prefer: Prefer,
    pub takes: Takes,
    /// Whether the other side as it stands was written by somebody
    /// trusted. It is asked only where `takes` is [`Takes::Trusted`].
    pub trusted: bool,
    pub held: Held,
}

/// What is to cross between two sides.
#[derive(Clone, Debug, PartialEq)]
pub struct Crossing {
    pub came: Came,
    /// What goes out: each field that is to be said there, as it is here.
    pub out: Map<String, Value>,
    /// What comes in: each field that is to be said here, as it is there.
    pub take: Map<String, Value>,
    /// What the two agree on once that is done, for every field of either:
    /// where nothing crossed for a field that differs, what they agreed
    /// on before.
    pub agreed: Map<String, Value>,
    /// Whether a change is held once this is done: what is written down
    /// beside what is agreed, in the same write.
    pub holds: bool,
    /// Whether it came to be held just now, so that it is said once.
    pub newly: bool,
}

/// What of `here` and `there` is to cross, each field held against what
/// the two last agreed on (`last`). A field of one side that the other
/// does not say is one it says nothing in. Where a field moved on both
/// sides, to different things, or was never agreed and differs, the side
/// `prefer` names is taken. What would come in is taken only from whom
/// `takes` says, and what somebody said of a change that was held
/// (`held`) is done whatever moved.
pub fn cross(asked: &Sides<'_>) -> Crossing {
    let Sides { here, there, last, .. } = *asked;
    let mut names: Vec<&String> = here.keys().collect();
    names.extend(there.keys().filter(|name| !here.contains_key(*name)));
    let said = |value: Option<&Value>| value.cloned().unwrap_or(Value::Null);
    // Which way each field that differs would go, with nothing held
    // back: outward, or inward.
    let outward = |name: &String| {
        let (ours, theirs, was) = (here.get(name), there.get(name), last.get(name));
        // Agreed before on what one of them still says: the other moved.
        // Never agreed, or both moved: whoever asked says which is taken.
        let agreed_before = last.contains_key(name);
        match (agreed_before && same(ours, was), agreed_before && same(theirs, was)) {
            (false, true) => true,
            (true, false) => false,
            _ => asked.prefer == Prefer::Here,
        }
    };
    let differing: Vec<&String> =
        names.iter().copied().filter(|name| !same(here.get(*name), there.get(*name))).collect();
    let comes_in = differing.iter().any(|name| !outward(name));
    // A change that is held, now or still: nothing crosses, and what
    // was agreed stands.
    let untrusted = asked.takes == Takes::Trusted && !asked.trusted && comes_in;
    let holding = match asked.held {
        Held::Waiting => !differing.is_empty(),
        Held::Nothing => untrusted,
        Held::Take | Held::Keep => false,
    };
    let (mut out, mut take, mut agreed) = (Map::new(), Map::new(), Map::new());
    for name in names {
        let (ours, theirs) = (here.get(name), there.get(name));
        if same(ours, theirs) {
            agreed.insert(name.clone(), said(ours));
            continue;
        }
        let goes = match asked.held {
            _ if holding => None,
            Held::Take => Some(false),
            Held::Keep => Some(true),
            // Nobody's changes are taken: what moved there alone is
            // neither taken nor written over.
            _ if asked.takes == Takes::Nobody && !outward(name) => None,
            _ => Some(outward(name)),
        };
        match goes {
            Some(true) => {
                out.insert(name.clone(), said(ours));
                agreed.insert(name.clone(), said(ours));
            }
            Some(false) => {
                take.insert(name.clone(), said(theirs));
                agreed.insert(name.clone(), said(theirs));
            }
            None => {
                agreed.insert(name.clone(), said(last.get(name)));
            }
        }
    }
    let came = match (holding, out.is_empty(), take.is_empty()) {
        (true, _, _) => Came::Held,
        (false, true, true) => Came::Same,
        (false, false, true) => Came::Out,
        (false, true, false) => Came::In,
        (false, false, false) => Came::Both,
    };
    Crossing { came, out, take, agreed, holds: holding, newly: holding && asked.held == Held::Nothing }
}

/// The same asked as an action is handed it, refusing what makes no
/// sense: for whoever holds this to a table of cases.
pub fn told(args: &Map<String, Value>) -> Result<Crossing, String> {
    let side = |key: &str, needed: bool| match args.get(key) {
        Some(Value::Object(side)) => Ok(side.clone()),
        None | Some(Value::Null) if !needed => Ok(Map::new()),
        None | Some(Value::Null) => {
            Err(format!("`{key}` is one of the two sides, each field by its name, and none is said"))
        }
        Some(other) => Err(format!("`{key}` is fields by their names, and is {other}")),
    };
    let word = |key: &str| args.get(key).and_then(Value::as_str).filter(|word| !word.is_empty());
    let prefer = match word("prefer") {
        Some("here") => Prefer::Here,
        Some("there") => Prefer::There,
        Some(other) => {
            return Err(format!(
                "`prefer` is which side is taken where both moved, `here` or `there`, and is `{other}`"
            ));
        }
        None => {
            return Err(
                "`prefer` is which side is taken where both moved, `here` or `there`, and neither is said"
                    .into(),
            );
        }
    };
    let takes = match word("takes") {
        None | Some("everyone") => Takes::Everyone,
        Some("trusted") => Takes::Trusted,
        Some("nobody") => Takes::Nobody,
        Some(other) => {
            return Err(format!(
                "`takes` is whose changes on the other side are taken, `everyone`, `trusted` or `nobody`, and is \
                 `{other}`"
            ));
        }
    };
    let held = match word("held") {
        None => Held::Nothing,
        Some("waiting") => Held::Waiting,
        Some("take") => Held::Take,
        Some("keep") => Held::Keep,
        Some(other) => {
            return Err(format!(
                "`held` is what was said of a change that is held: nothing, `waiting`, `take` to have the other \
                 side stand or `keep` to have this one; it is `{other}`"
            ));
        }
    };
    let trusted = match args.get("trusted") {
        Some(Value::Bool(trusted)) => *trusted,
        None | Some(Value::Null) if takes != Takes::Trusted => true,
        None | Some(Value::Null) => {
            return Err(
                "`trusted` is whether the other side as it stands was written by somebody trusted, which \
                        taking only what such a one changed goes by, and it is not said"
                    .into(),
            );
        }
        Some(other) => return Err(format!("`trusted` is yes or no, and is {other}")),
    };
    let (here, there, last) = (side("here", true)?, side("there", true)?, side("last", false)?);
    Ok(cross(&Sides { here: &here, there: &there, last: &last, prefer, takes, trusted, held }))
}

/// Every word [`owed`] can answer.
pub const OWED: &[&str] = &["some", "none"];

/// Which things have not crossed yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Owed {
    /// Each thing that has not crossed, in the order it was handed, with
    /// `kept` beside what it says: the names of all that have crossed
    /// once it has, which is what is written down as it does.
    pub those: Vec<Value>,
    /// Their names, in that order.
    pub names: Vec<Value>,
    /// The names of all that have crossed once each of those has.
    pub kept: Vec<Value>,
}

impl Owed {
    pub fn word(&self) -> &'static str {
        if self.those.is_empty() { "none" } else { "some" }
    }
}

/// Which of `things` have not crossed: those whose name (what each says
/// in `by`) is not among `had`, nor among `also`, and that do not say
/// `true` in `except`, where one is named. A name is counted once.
pub fn owed(
    things: &[Value],
    by: &str,
    had: &[Value],
    also: &[Value],
    except: Option<&str>,
) -> Result<Owed, String> {
    let mut kept: Vec<Value> = Vec::new();
    for name in had {
        if !kept.contains(name) {
            kept.push(name.clone());
        }
    }
    let (mut those, mut names) = (Vec::new(), Vec::new());
    for thing in things {
        let Some(says) = thing.as_object() else {
            return Err(format!("each of `things` is fields by their names, and one is {thing}"));
        };
        let name = match says.get(by) {
            None | Some(Value::Null) => {
                return Err(format!("each of `things` says its name in `{by}`, and one says none: {thing}"));
            }
            Some(Value::String(text)) if text.is_empty() => {
                return Err(format!("each of `things` says its name in `{by}`, and one says none: {thing}"));
            }
            Some(name) => name,
        };
        let left_out = except.is_some_and(|field| says.get(field) == Some(&Value::Bool(true)));
        if left_out || kept.contains(name) || also.contains(name) {
            continue;
        }
        kept.push(name.clone());
        let mut with = says.clone();
        with.insert("kept".into(), Value::Array(kept.clone()));
        those.push(Value::Object(with));
        names.push(name.clone());
    }
    Ok(Owed { those, names, kept })
}

/// The same asked as an action is handed it.
pub fn owed_told(args: &Map<String, Value>) -> Result<Owed, String> {
    let list = |key: &str| match args.get(key) {
        Some(Value::Array(items)) => Ok(items.clone()),
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(other) => Err(format!("`{key}` is a list, and is {other}")),
    };
    let word = |key: &str| args.get(key).and_then(Value::as_str).filter(|word| !word.is_empty());
    owed(&list("things")?, word("by").unwrap_or("id"), &list("had")?, &list("also")?, word("except"))
}
