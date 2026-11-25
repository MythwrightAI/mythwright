//! What comes next for a card that waits on somebody: the rungs of a
//! project's `chain`.
//!
//! A row of the table is one rung: which kinds of wait it is for
//! (`waits`: `review`, `left`, `blocked`, `asked`, `held`, `plan`, and
//! `idle`, a card in a column no stage works, which a rung is for only
//! where it names that column; every kind but `idle` where it says none),
//! held to a column or to cards with a label where it says so, where it
//! stands among the rest (`step`, the lowest first), who is next (`who`)
//! and how long the card may wait there before the next rung is tried too
//! (`after`; never, where it says none).
//!
//! [`next`] is what a wait comes to: begun, the first rung there is for
//! it acts; a time come, the rung it came for acts where the wait still
//! stands. A wait stands while the card has not been moved since it began
//! and is the same kind of wait. What is kept of it is on the card's own
//! row (`rung`: the kind, the count of the card's moves it began at, and
//! how many rungs it has reached). With no rung for it nothing is done:
//! a chain with no row adds nothing of its own to what `tells` tells.
//! [`read`] says what of the chain is asked before anything is run: whether
//! it has a rung at all, and which columns a rung names for a card nobody
//! works.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::add;

blueprint::came! {
    /// What a wait comes to.
    pub enum Came {
        /// A rung acts: whoever it names is told.
        Acts => "acts",
        /// Nothing: no rung is for it, it no longer stands, or it was
        /// begun already.
        Nothing => "nothing",
    }
}

/// Every kind of wait a rung can be for.
pub const KINDS: &[&str] = &["review", "left", "blocked", "asked", "held", "plan", "idle"];

/// One rung, as its row says it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rung {
    pub title: String,
    pub step: i64,
    pub who: String,
    /// What it does: `tell` whoever it names; `look`, the agent it names
    /// (`agent:<name>`, or one of a role, `role:<role>`) looks at the card
    /// and decides what becomes of it; `review`, the agent it names
    /// reviews the card as its stage would have one of its role review it.
    pub does: String,
    /// What an agent that looks or reviews is told, in the row's words.
    pub told: String,
    /// How long, as written (`2h`); empty for never.
    pub after: String,
}

/// Every way a rung acts.
pub const DOES: &[&str] = &["tell", "look", "review"];

/// What a wait is asked of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    /// The rows of the chain, as they were read.
    pub rows: Vec<Value>,
    /// What said there is a wait (`card.left`, `card.asked`, `card.waits`,
    /// `card.planned`, `board.item.held`, `board.item.moved`), and what it
    /// carried; for a time that came, what the wait was begun with.
    pub topic: String,
    pub said: Map<String, Value>,
    /// The card, its column and its labels.
    pub card: String,
    pub column: String,
    pub labels: Vec<String>,
    /// How many times it has been moved (the count its `moved` keeps).
    pub moves: i64,
    /// What its row keeps of where on the chain it is: the kind, the
    /// count of moves it began at, how many rungs it has reached; empty
    /// where it waits on nothing.
    pub kept: Vec<Value>,
    /// For a time that came: which rung it came for.
    pub due: Option<i64>,
    pub project: String,
    /// The agents that worked the card, by name: a rung that names one of
    /// them to look at it or review it is passed over, since an agent
    /// never judges its own work.
    pub worked: Vec<String>,
}

/// What it comes to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
    pub came: Option<Came>,
    /// Which kind of wait it is.
    pub kind: String,
    /// The rung that acts, and the sentence it tells.
    pub rung: Rung,
    pub says: String,
    /// What the card's row keeps from now.
    pub keep: Vec<Value>,
    /// The next rung, and after how long it is to be asked for with
    /// `due`, where there is one and the rung that acts says a time.
    pub next: i64,
    pub after: String,
    /// Whether there is a rung after this one at all: an agent that looks
    /// may have whoever is next told now.
    pub further: bool,
}

/// Which kind of wait what was said is.
pub fn kind(topic: &str, said: &Map<String, Value>) -> Result<String, String> {
    let word = |key: &str| said.get(key).and_then(Value::as_str).unwrap_or_default();
    Ok(match topic {
        "card.left" if word("as") == "blocked" => "blocked",
        "card.left" => "left",
        "card.asked" => "asked",
        "card.planned" => "plan",
        "board.item.held" => "held",
        "board.item.moved" => "idle",
        "card.waits" if KINDS.contains(&word("kind")) => word("kind"),
        _ => {
            return Err(format!(
                "`{topic}` says no wait a rung is for: a wait is said as `card.left`, `card.asked`, \
                 `card.waits`, `card.planned`, `board.item.held`, or a card's move to a column nobody works"
            ));
        }
    }
    .to_string())
}

fn text(row: &Map<String, Value>, key: &str) -> String {
    row.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

fn words(row: &Map<String, Value>, key: &str) -> Vec<String> {
    let said: Vec<String> = match row.get(key) {
        Some(Value::Array(words)) => words.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(one)) => one.split(',').map(str::to_string).collect(),
        _ => Vec::new(),
    };
    said.into_iter().map(|word| word.trim().to_string()).filter(|word| !word.is_empty()).collect()
}

/// A rung with what it is held to: the kinds, the column, the labels.
type HeldTo = (Rung, Vec<String>, String, Vec<String>);

/// Each row as a rung, with what it is held to. A row that names nobody
/// is no rung.
fn rows(rows: &[Value]) -> Result<Vec<HeldTo>, String> {
    let mut found = Vec::new();
    for row in rows {
        let Some(row) = row.as_object() else {
            return Err(format!("`rows` is the rows of the chain, each a row; one of them is {row}"));
        };
        let waits = words(row, "waits");
        if let Some(bad) = waits.iter().find(|word| !KINDS.contains(&word.as_str())) {
            return Err(format!(
                "a rung of the chain waits for `{bad}`, which is no kind of wait: {}",
                KINDS.iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>().join(", ")
            ));
        }
        let after = text(row, "after");
        if !after.is_empty() {
            blueprint::duration::parse(&after)
                .map_err(|why| format!("a rung of the chain says to wait `{after}`: {why}"))?;
        }
        let who = text(row, "who");
        if who.is_empty() {
            continue;
        }
        let does = match text(row, "does") {
            does if does.is_empty() => "tell".to_string(),
            does => does,
        };
        if !DOES.contains(&does.as_str()) {
            return Err(format!(
                "a rung of the chain does `{does}`, and a rung does `tell`, `look` or `review`"
            ));
        }
        let agent = who.strip_prefix("agent:").is_some_and(|name| !name.is_empty());
        let role = who.strip_prefix("role:").is_some_and(|role| !role.is_empty());
        if does == "look" && !agent && !role {
            return Err(format!(
                "a rung of the chain that looks names an agent, as `agent:<name>`, or one of a role, as \
                 `role:<role>`; it names `{who}`"
            ));
        }
        if does == "review" && (!agent || waits.iter().any(|kind| kind != "review") || waits.is_empty()) {
            return Err(format!(
                "a rung of the chain that reviews names one agent, as `agent:<name>`, and waits for `review` \
                 alone; it names `{who}` and waits for {}",
                if waits.is_empty() { "every kind".to_string() } else { waits.join(", ") }
            ));
        }
        let step = match row.get("step") {
            Some(Value::Number(step)) => step.as_i64().or_else(|| step.as_f64().map(|step| step as i64)),
            _ => None,
        };
        let told = text(row, "told");
        let rung = Rung { title: text(row, "title"), step: step.unwrap_or(0), who, does, told, after };
        found.push((rung, waits, text(row, "column"), words(row, "labelled")));
    }
    found.sort_by_key(|(rung, ..)| rung.step);
    Ok(found)
}

/// What is asked of the chain before anything is run for a card: whether
/// it has a rung, and which columns a rung names for a card in a column
/// nobody works.
pub fn read(all: &[Value]) -> Result<(bool, Vec<String>), String> {
    let found = rows(all)?;
    let mut idle: Vec<String> = Vec::new();
    for (_, waits, column, _) in &found {
        if waits.iter().any(|kind| kind == "idle") && !column.is_empty() && !idle.contains(column) {
            idle.push(column.clone());
        }
    }
    Ok((!found.is_empty(), idle))
}

/// The sentence a rung tells, by the kind of wait.
fn says(asked: &Asked, kind: &str) -> Result<String, String> {
    if kind == "idle" {
        return Ok(format!(
            "{} is in {}, where nobody works its cards: it waits for a person.",
            asked.card, asked.column
        ));
    }
    let mut said = asked.said.clone();
    said.entry("card").or_insert_with(|| json!(asked.card));
    let of = match kind {
        // What a card left that says why, or one that waits, is told as.
        "blocked" | "left" => "left",
        "review" => "review",
        other => other,
    };
    Ok(crate::told::whom(&[], of, &said, "", &asked.project)?.says)
}

/// A count, written as a number or as the text a list of a row keeps.
fn number(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

/// What a wait comes to.
pub fn next(asked: &Asked) -> Result<Answer, String> {
    let kind = kind(&asked.topic, &asked.said)?;
    let nothing = Answer { came: Some(Came::Nothing), kind: kind.clone(), ..Answer::default() };
    let rungs: Vec<Rung> = rows(&asked.rows)?
        .into_iter()
        .filter(|(_, waits, column, labelled)| {
            // A column nobody works is waited on only where a rung names it.
            let of_kind = match kind.as_str() {
                "idle" => waits.iter().any(|word| word == "idle") && !column.is_empty(),
                kind => waits.is_empty() || waits.iter().any(|word| word == kind),
            };
            let here = column.is_empty() || *column == asked.column;
            let labels = labelled.is_empty() || labelled.iter().any(|label| asked.labels.contains(label));
            of_kind && here && labels
        })
        .map(|(rung, ..)| rung)
        .collect();
    let kept_kind = asked.kept.first().and_then(Value::as_str).unwrap_or_default();
    let kept_moves = asked.kept.get(1).and_then(number);
    let reached = asked.kept.get(2).and_then(number).unwrap_or(0);
    let same = kept_kind == kind && kept_moves == Some(asked.moves);
    let at = match asked.due {
        // Said again of the same coming: begun already.
        None if same => return Ok(nothing),
        None => 0,
        // A time come for a wait that no longer stands, or for another
        // rung than the one it has come to.
        Some(due) if !same || due != reached => return Ok(nothing),
        Some(due) => due,
    };
    let rung_at = |at: i64| usize::try_from(at).ok().and_then(|at| rungs.get(at)).cloned();
    // A rung that names an agent that worked the card to judge it is
    // passed over to the next; where there is none after it, whoever
    // applied the fleet is told so in its place.
    let mut at = at;
    let mut over: Option<String> = None;
    while let Some(rung) = rung_at(at).filter(|rung| judges_own(rung, &asked.worked)) {
        over = Some(rung.who.trim_start_matches("agent:").to_string());
        at += 1;
    }
    let rung = match (rung_at(at), over) {
        (Some(rung), _) => rung,
        (None, None) => return Ok(nothing),
        (None, Some(agent)) => {
            let says = format!(
                "{} {agent} is next for it on the chain, and worked it: an agent does not judge its own work, so a \
                 person is to.",
                says(asked, &kind)?
            );
            let rung = Rung {
                title: "applier".into(),
                who: "applier".into(),
                does: "tell".into(),
                ..Rung::default()
            };
            let keep = vec![json!(kind), json!(asked.moves), json!(at + 1)];
            return Ok(Answer { came: Some(Came::Acts), kind, rung, says, keep, ..Answer::default() });
        }
    };
    let keep = vec![json!(kind), json!(asked.moves), json!(at + 1)];
    let further = usize::try_from(at + 1).is_ok_and(|next| next < rungs.len());
    let after = match rung.after.is_empty() || !further {
        true => String::new(),
        false => rung.after.clone(),
    };
    let next = if further { at + 1 } else { 0 };
    let says = says(asked, &kind)?;
    Ok(Answer { came: Some(Came::Acts), kind, rung, says, keep, next, after, further })
}

/// Whether a rung names an agent that worked the card to look at it or
/// review it.
fn judges_own(rung: &Rung, worked: &[String]) -> bool {
    let agent = rung.who.strip_prefix("agent:").unwrap_or_default();
    rung.does != "tell" && !agent.is_empty() && worked.iter().any(|one| one == agent)
}

/// The words an agent that looks may end with, by the kind of wait, each
/// with what it comes to as a turn (`turn.came`): `back`, back to be worked
/// with what it says; `ask`, a person is asked; `up`, the next rung now;
/// `stay`, it waits as it is; and where the card waits on a question,
/// `answer`, the question answered with what it says.
pub fn looks_with(kind: &str) -> Vec<String> {
    let mut words = vec!["stay=on", "back=back", "ask=asks", "up=left"];
    if kind == "asked" {
        words.push("answer=again");
    }
    words.into_iter().map(str::to_string).collect()
}

blueprint::came! {
    /// What an agent that looked decided comes to.
    pub enum Decided {
        /// The card goes back to be worked, as the agent decided.
        Back => "back",
        /// A person is asked, in the agent's words.
        Ask => "ask",
        /// The next rung is told now.
        Up => "up",
        /// The question asked on the card is answered.
        Answer => "answer",
        /// It waits as it is.
        Stay => "stay",
        /// Nothing: the card was moved while the agent looked, or is gone.
        Nothing => "nothing",
    }
}

/// What an agent that looked at a waiting card is asked of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Looked {
    pub item: String,
    pub kind: String,
    /// What its turn came to (`turn.came` of the words of [`words`]), and
    /// what it said: `says` for `back`, `why` for `again` and `left`,
    /// `question` for `asks`.
    pub came: String,
    pub says: String,
    pub why: String,
    pub question: String,
    /// The agent.
    pub agent: String,
    /// Whether it holds what moving a card takes.
    pub may_move: bool,
    /// The card as it now is: whether it was found, its column, the
    /// column it was taken from to be worked, who asked on it, and the
    /// count of its moves, against the count as the rung began.
    pub found: bool,
    pub status: String,
    pub taken: String,
    pub asker: String,
    pub moves: i64,
    pub began: i64,
    /// How long the rung says, and whether there is a rung after it.
    pub after: String,
    pub further: bool,
    /// The agents that worked the card: one of the role that looked, and
    /// worked it, does not judge its own work.
    pub worked: Vec<String>,
}

/// What it comes to: the word, where the card goes, what goes with it,
/// what a person is asked, and after how long the next rung is asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Decision {
    pub decided: Option<Decided>,
    pub to: String,
    pub note: String,
    pub asks: String,
    pub later: bool,
    pub after: String,
}

/// What an agent that looked at a waiting card decided comes to.
pub fn decided(asked: &Looked) -> Result<Decision, String> {
    let agent = if asked.agent.is_empty() { "its agent" } else { asked.agent.as_str() };
    let further = |after: &str| (asked.further && !after.is_empty(), after.to_string());
    let stay = |decided| {
        let (later, after) = further(&asked.after);
        Decision { decided: Some(decided), later, after, ..Decision::default() }
    };
    // Moved, or gone, while it looked: it is whoever moved it's to say.
    if !asked.found || asked.moves != asked.began {
        return Ok(Decision { decided: Some(Decided::Nothing), ..Decision::default() });
    }
    let said = |words: &str| words.trim().to_string();
    if !asked.agent.is_empty() && asked.worked.contains(&asked.agent) {
        return Ok(Decision {
            asks: format!(
                "{}: {agent} looked at it and worked it, and an agent does not judge its own work: a person is to.",
                asked.item
            ),
            ..stay(Decided::Ask)
        });
    }
    Ok(match asked.came.as_str() {
        "on" => stay(Decided::Stay),
        "left" => {
            let (later, after) = further("0s");
            Decision { decided: Some(Decided::Up), later, after, ..Decision::default() }
        }
        "asks" => Decision {
            asks: format!("{}: {agent} asks: {}", asked.item, said(&asked.question)),
            ..stay(Decided::Ask)
        },
        "again" if asked.kind == "asked" && !asked.asker.is_empty() => {
            Decision { note: said(&asked.why), ..stay(Decided::Answer) }
        }
        // An answer where nobody asked anything is a person's to give.
        "again" => Decision {
            asks: format!(
                "{}: {agent} would answer what was asked, and nothing is: {}",
                asked.item,
                said(&asked.why)
            ),
            ..stay(Decided::Ask)
        },
        "back" if asked.taken.is_empty() || asked.taken == asked.status => Decision {
            asks: format!(
                "{}: {agent} would send it back to be worked, and it was taken from nowhere it can go back to: {}",
                asked.item,
                said(&asked.says)
            ),
            ..stay(Decided::Ask)
        },
        "back" if !asked.may_move => Decision {
            asks: format!(
                "{}: {agent} would send it back to {}, and may not move a card itself: {}",
                asked.item,
                asked.taken,
                said(&asked.says)
            ),
            ..stay(Decided::Ask)
        },
        "back" => Decision {
            decided: Some(Decided::Back),
            to: asked.taken.clone(),
            note: said(&asked.says),
            ..Decision::default()
        },
        other => {
            return Err(format!(
                "`came` is what the agent's turn came to, `on`, `back`, `asks`, `left` or `again`; it was given `{other}`"
            ));
        }
    })
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "chain.read",
            "Say what of a chain of rungs is asked before anything is run: whether it has a rung, and which columns \
             a rung names for a card nobody works.",
        )
        .deciding()
        .required("rows", T::List, "the rows of the chain, as they were read (`rows` of `rows.read`)")
        .output("rungs", "whether any row names somebody")
        .output("idle", "the columns a rung that waits for `idle` names")
        .idempotent(),
        |_, args| async move {
            let rows = args.get("rows").and_then(Value::as_array).cloned().unwrap_or_default();
            let (rungs, idle) = read(&rows).map_err(ActionError::new)?;
            Ok(json!({ "rungs": rungs, "idle": idle }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "chain.next",
            "Say who is next for an item that waits on somebody, by the rungs of a chain, and the sentence they are \
             told; and after how long the next rung is to be asked for.",
        )
        .deciding()
        .required("rows", T::List, "the rows of the chain, as they were read")
        .required("topic", T::String, "what said there is a wait: `card.left`, `card.asked`, `card.waits`, `card.planned`, `board.item.held`, or `board.item.moved` for one in a column nobody works")
        .arg("said", T::Any, "what that carried, whole")
        .required("item", T::String, "the item")
        .arg("column", T::String, "the column it is in")
        .arg("labels", T::List, "the labels it holds")
        .arg("moved", T::List, "how its row says it came to be where it is: the count of its moves is the fourth")
        .arg("kept", T::List, "what its row keeps of where on the chain it is")
        .arg("due", T::Int, "for a time that came: which rung it came for; none, or below nought, for a wait that is begun")
        .arg("project", T::String, "the project it is in")
        .arg("worked", T::List, "the agents that worked it, by name (`worked` of `record.said`): a rung that names one of them to judge it is passed over")
        .output("came", &Came::words())
        .output("kind", &format!("which kind of wait: {}", KINDS.iter().map(|kind| format!("`{kind}`")).collect::<Vec<_>>().join(", ")))
        .output("who", "for `acts`: whom the rung tells, as its row says them")
        .output("title", "for `acts`: what the rung is called")
        .output("says", "for `acts`: the sentence")
        .output("keep", "for `acts`: what the item's row keeps from now of where on the chain it is")
        .output("later", "whether the next rung is to be asked for after a while")
        .output("next", "which rung that is")
        .output("after", "after how long, as written")
        .output("further", "whether there is a rung after this one")
        .output("does", "for `acts`: `tell`, `look` or `review`")
        .output("told", "for `acts`: what the rung's row says to an agent that looks or reviews")
        .output("agent", "for `look` and `review`: the agent the rung names, by its name")
        .output("role", "for `look`: or the role one of whose agents looks")
        .output("words", "for `look`: the words the agent may end with, each with what it comes to as a turn")
        .output("moves", "the count of the item's moves as it waits, for what is decided to look at it first")
        .output("column", "the column it waits in")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let list = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let moves = list("moved").get(3).and_then(number);
            let said = match args.get("said") {
                None | Some(Value::Null) => Map::new(),
                Some(Value::Object(said)) => said.clone(),
                Some(other) => {
                    return Err(ActionError::new(format!(
                        "`said` is what an event carried, by name; it was given {other}"
                    )));
                }
            };
            let asked = Asked {
                rows: list("rows"),
                topic: text("topic"),
                said,
                card: text("item"),
                column: text("column"),
                labels: list("labels").iter().map(blueprint::expr::display).collect(),
                moves: moves.unwrap_or(0),
                kept: list("kept"),
                // A wait that is begun says none, or one below nought.
                due: args.get("due").and_then(Value::as_i64).filter(|due| *due >= 0),
                project: text("project"),
                worked: list("worked").iter().filter_map(Value::as_str).map(str::to_string).collect(),
            };
            let answer = next(&asked).map_err(ActionError::new)?;
            let who = &answer.rung.who;
            let named = |prefix: &str| who.strip_prefix(prefix).unwrap_or_default().to_string();
            Ok(json!({
                "came": answer.came.map(|came| came.word()).unwrap_or("nothing"),
                "kind": answer.kind, "who": [who], "title": answer.rung.title, "says": answer.says,
                "keep": answer.keep, "later": !answer.after.is_empty(), "next": answer.next, "after": answer.after,
                "further": answer.further, "does": answer.rung.does, "told": answer.rung.told,
                "agent": named("agent:"), "role": named("role:"), "words": looks_with(&answer.kind),
                "moves": asked.moves, "column": asked.column,
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "chain.decided",
            "Say what the word an agent that looked at a waiting item decided comes to: back to be worked, a \
             person asked, the next rung told now, the question on it answered, or it waits as it is.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("kind", T::String, "which kind of wait it is")
        .arg(
            "came",
            T::String,
            "what the agent's turn came to (`came` of `std/turn`): `on`, `back`, `asks`, `left` or `again`",
        )
        .arg("says", T::String, "for `back`: what it said")
        .arg("why", T::String, "for `again`: what it said")
        .arg("question", T::String, "for `asks`: what it asked")
        .arg("agent", T::String, "the agent, by its name")
        .arg("may_move", T::Bool, "whether the agent itself holds what moving an item takes")
        .arg("found", T::Bool, "whether the item was found, as it now is")
        .arg("status", T::String, "the column it is in now")
        .arg("taken", T::String, "the column it was taken from to be worked")
        .arg("asker", T::String, "who asked what it waits on, where it waits on a question")
        .arg(
            "moved",
            T::List,
            "how its row says it came to be where it is, now: the count of its moves is the fourth",
        )
        .arg("moves", T::Int, "the count of its moves as the agent was asked")
        .arg("after", T::String, "how long the rung says the item may wait at it")
        .arg("further", T::Bool, "whether there is a rung after it")
        .arg(
            "worked",
            T::List,
            "the agents that worked it, by name: one that looked and worked it does not judge it",
        )
        .output("decided", &Decided::words())
        .output("to", "for `back`: the column it goes back to")
        .output("note", "for `back`: what its worker is told; for `answer`: the answer")
        .output("asks", "for `ask`: what the person is asked")
        .output("later", "whether the next rung is to be asked for after a while")
        .output("after", "after how long: none for `up`")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let moved = args.get("moved").and_then(Value::as_array).cloned().unwrap_or_default();
            let looked = Looked {
                item: text("item"),
                kind: text("kind"),
                came: text("came"),
                says: text("says"),
                why: text("why"),
                question: text("question"),
                agent: text("agent"),
                may_move: flag("may_move"),
                found: flag("found"),
                status: text("status"),
                taken: text("taken"),
                asker: text("asker"),
                moves: moved.get(3).and_then(number).unwrap_or(0),
                began: args.get("moves").and_then(number).unwrap_or(0),
                after: text("after"),
                further: flag("further"),
                worked: args
                    .get("worked")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            };
            let decision = decided(&looked).map_err(ActionError::new)?;
            Ok(json!({
                "decided": decision.decided.map(|decided| decided.word()).unwrap_or("nothing"),
                "to": decision.to, "note": decision.note, "asks": decision.asks, "later": decision.later,
                "after": decision.after,
            }))
        },
    );
}
