//! What is wrong with a row that says what is done with the items of a
//! column: said in sentences, for whoever arranged it to put right.
//!
//! A row names the column whose items it is for, where an item goes on
//! to and where it is sent back to, a role and how many of it look. None
//! of that is held to anything when the row is written, a cell at a
//! time; what it comes to is looked at each time the row is taken, and
//! what cannot work is said where the work is reported. Which sentences
//! those are was worked out in the text that asks, in the holes of its
//! strings; it is a function here, of what the row says and the columns
//! there are, with a case for each sentence. So is what a row that was
//! taken again came to, held against what it said before ([`changed`]).

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// A row, and what it is held against.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    /// What the row is called, if anything.
    pub title: String,
    /// The column whose items it is for.
    pub column: String,
    /// Where an item goes on to, and where it is sent back to.
    pub pass: String,
    pub fail: String,
    /// The columns there are, where they could be read.
    pub columns: Option<Vec<String>>,
    /// The role, and how many there are of it.
    pub role: String,
    pub agents: i64,
    /// How many look at each item, and how many have to approve.
    pub seats: i64,
    pub needed: i64,
    /// Items are looked at here, or are worked here.
    pub reviews: bool,
    pub works: bool,
}

/// The sentences [`wrong`] can say, by what each is about.
pub const WRONG: &[&str] = &["astray", "short", "idle"];

/// What is wrong with a row, each as what it is about and the sentence:
/// a column it names that is not there (`astray`), more to look or to
/// approve than there are (`short`), nobody to work what is worked here,
/// or nobody in the role of a stage that begins something for each item
/// (`idle`: such a stage begins nothing while its role has nobody, whether
/// or not what it begins asks an agent). Nothing, for a row that
/// can work.
pub fn wrong(row: &Row) -> Result<Vec<(&'static str, String)>, String> {
    if row.column.is_empty() {
        return Err("`column` is the column whose items the row is for, and none is said".into());
    }
    for (name, count) in [("agents", row.agents), ("seats", row.seats), ("needed", row.needed)] {
        if count < 0 {
            return Err(format!("`{name}` is a count, and is not less than none; it is {count}"));
        }
    }
    let called = if row.title.is_empty() { &row.column } else { &row.title };
    let mut said = Vec::new();
    if let Some(columns) = &row.columns {
        let has = |column: &String| columns.contains(column);
        let astray = if !has(&row.column) {
            Some(format!("its own, {}", row.column))
        } else if !row.pass.is_empty() && !has(&row.pass) {
            Some(format!("where a card goes on to, {}", row.pass))
        } else if row.reviews && !has(&row.fail) {
            Some(format!("where a card is sent back to, {}", row.fail))
        } else {
            None
        };
        if let Some(which) = astray {
            said.push((
                "astray",
                format!(
                    "The stage {called} names a column the board has not got (its columns are {}): {which}. Put it \
                     right in the project's stages.",
                    columns.join(", ")
                ),
            ));
        }
    }
    if row.reviews && row.agents > 0 && (row.seats > row.agents || row.needed > row.seats) {
        let how = if row.seats > row.agents {
            format!("has {} look at each card, and its role, {}, has {}", row.seats, row.role, row.agents)
        } else {
            format!("needs {} to approve, and {} look", row.needed, row.seats)
        };
        said.push((
            "short",
            format!(
                "The stage {called} {how}: nothing is reviewed there until that is put right, and a card waits \
                 for a person."
            ),
        ));
    }
    if row.works && row.agents == 0 {
        said.push((
            "idle",
            format!(
                "Nobody in the fleet has the role {}, which works the cards in {}: a card put there is not begun \
                 until somebody has. Give a row of the project's agents that role.",
                row.role, row.column
            ),
        ));
    } else if !row.works && !row.reviews && row.agents == 0 {
        said.push((
            "idle",
            format!(
                "Nobody in the fleet has the role {}, which the stage {called} is for: it begins nothing for a card \
                 in {} until somebody has, whether or not what it begins asks an agent. Give a row of the project's \
                 agents that role, or say a role somebody has.",
                row.role, row.column
            ),
        ));
    }
    Ok(said)
}

/// A row that was taken as it now says, held against what it said
/// before: how items are looked at, and by how many.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Taken {
    /// The column whose items it is for.
    pub column: String,
    /// How items were looked at before, and how they are now, each as
    /// one word whoever asks puts together the same way both times.
    pub was: String,
    pub now: String,
    /// How many look at each item now, and how many have to approve;
    /// more than half of those who look, where none is said.
    pub seats: i64,
    pub needed: i64,
    /// How many there are of the role now, and how many there were.
    pub agents: i64,
    pub had: i64,
    /// What is under way is begun again as now arranged, and is not
    /// left to finish as it began.
    pub restart: bool,
}

/// What a row that was taken came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Changed {
    /// Items are looked at as they were: nothing is to be said.
    Same,
    /// They are looked at another way from now. What is under way is to
    /// be told; and where there were and are some to look, the sentence
    /// that says so where the work is reported.
    Arranged { told: Option<String> },
}

impl Changed {
    pub fn word(&self) -> &'static str {
        match self {
            Changed::Same => "same",
            Changed::Arranged { .. } => "arranged",
        }
    }
}

/// Every word [`changed`] can answer.
pub const CHANGED_WORDS: &[&str] = &["same", "arranged"];

/// What a row that was taken as it now says came to: whether items are
/// looked at another way from now, and the sentence that says how, said
/// only where somebody was looking before and somebody is now (a role
/// that had nobody, or has nobody, says what it has another way).
pub fn changed(taken: &Taken) -> Result<Changed, String> {
    if taken.column.is_empty() {
        return Err("`column` is the column whose items the row is for, and none is said".into());
    }
    if taken.now.is_empty() {
        return Err("`now` is how items are looked at from now, and nothing is said".into());
    }
    for (name, count) in
        [("seats", taken.seats), ("needed", taken.needed), ("agents", taken.agents), ("had", taken.had)]
    {
        if count < 0 {
            return Err(format!("`{name}` is a count, and is not less than none; it is {count}"));
        }
    }
    if taken.was == taken.now {
        return Ok(Changed::Same);
    }
    if taken.agents == 0 || taken.had == 0 {
        return Ok(Changed::Arranged { told: None });
    }
    let council = if taken.seats == 1 {
        "a council of one".to_string()
    } else {
        let needed = if taken.needed > 0 { taken.needed } else { taken.seats / 2 + 1 };
        format!("a council of {}, and passes when {needed} approve", taken.seats)
    };
    let under_way = if taken.restart {
        "Reviews under way are begun again that way."
    } else {
        "Reviews in session finish as they began."
    };
    Ok(Changed::Arranged {
        told: Some(format!("From now each card in {} is looked at by {council}. {under_way}", taken.column)),
    })
}

/// How much is done at once with the items of a column, as what gives
/// them their turns has said: that it gives none for now, or how many
/// it gives from now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Paced {
    /// What was said: that it gives no place for now (`pool.paused`, or
    /// `paused`), or that it gives places, and how many (`pool.open`).
    pub topic: String,
    pub column: String,
    /// Items are looked at here, and not worked.
    pub reviews: bool,
    /// Why none is given for now, and how many are given from now.
    pub why: String,
    pub limit: String,
}

/// The sentence that says so where the work is reported.
pub fn paced(asked: &Paced) -> Result<String, String> {
    let (column, why, limit) = (&asked.column, &asked.why, &asked.limit);
    match (asked.topic.rsplit('.').next().unwrap_or_default(), asked.reviews) {
        ("paused", true) => Ok(format!(
            "No new review is being started in {column} for now: {why}. What is under way goes on."
        )),
        ("paused", false) => {
            Ok(format!("No new work is being started for now: {why}. What is under way goes on."))
        }
        ("open", true) => Ok(format!("Cards in {column} are reviewed {limit} at a time from now.")),
        ("open", false) => Ok(format!("Cards in {column} are worked {limit} at a time from now.")),
        (other, _) => Err(format!(
            "`topic` is what was said of how much is done at once, `pool.paused` or `pool.open`; it is `{other}`"
        )),
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "column.paced",
            "Say, in a sentence for where the work is reported, how much is done at once with the items of a column from now: nothing new for now and why, or so many at a time.",
        )
        .deciding()
        .required("topic", T::String, "what was said: `pool.paused` or `pool.open`")
        .arg("column", T::String, "the column whose items they are")
        .arg("said", T::Object, "what was said, whole, in place of saying each thing of it: `why`, `limit`, and `pool`, what gives the turns is called")
        .arg("under", T::String, "with `columns`: what the turns of a column are given by is called this, a `/`, and the column")
        .arg("columns", T::List, "the columns there are, where which of them it is said of is to be found by `pool`")
        .arg("looked", T::List, "those of them whose items are looked at, and not worked")
        .arg("reviews", T::Bool, "items are looked at here, and not worked")
        .arg("why", T::String, "why nothing new is started for now")
        .arg("limit", T::Any, "how many are done at a time from now")
        .output("says", "the sentence; empty where it was said of no column there is")
        .idempotent(),
        move |_, args| async move {
            let args = crate::begun::with(&args, "said", &[("why", "why"), ("limit", "limit"), ("pool", "pool")]);
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let words = |key: &str| -> Vec<String> {
                let said = args.get(key).and_then(Value::as_array);
                said.map(|all| all.iter().map(blueprint::expr::display).collect()).unwrap_or_default()
            };
            // Which column it is said of, where that is to be found: the
            // one whose turns are given by what was named.
            let (under, pool) = (text("under"), text("pool"));
            let found = words("columns").into_iter().find(|column| format!("{under}/{column}") == pool);
            let column = match (args.contains_key("columns"), found) {
                (true, Some(column)) => column,
                // Said of something that gives no column here its turns.
                (true, None) => return Ok(json!({ "says": "" })),
                (false, _) => text("column"),
            };
            let looked = words("looked").contains(&column);
            let asked = Paced {
                topic: text("topic"),
                reviews: args.get("reviews").and_then(Value::as_bool).unwrap_or(looked),
                column,
                why: text("why"),
                limit: args.get("limit").map(blueprint::expr::display).unwrap_or_default(),
            };
            Ok(json!({ "says": paced(&asked).map_err(ActionError::new)? }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "column.changed",
            "Say what a row that was taken as it now says came to, held against what it said before: whether the \
             items of its column are looked at another way from now, and the sentence that says how.",
        )
        .deciding()
        .required("column", T::String, "the column whose items it is for")
        .arg("was", T::String, "how items were looked at before, as one word")
        .required("now", T::String, "how they are looked at now, put together the same way")
        .arg("seats", T::Int, "how many look at each item now")
        .arg("needed", T::Int, "how many have to approve; more than half of those who look, if 0")
        .arg("agents", T::Int, "how many there are of the role now")
        .arg("had", T::Int, "how many there were")
        .arg("restart", T::Bool, "what is under way is begun again as now arranged")
        .arg("taken", T::Object, "what was said as the row was taken, whole, in place of saying each thing of it: `was`, `seats`, `needed`, `agents`, `had`, and `in_flight`, which is `restart` where it says so")
        .output("came", "`arranged`, they are looked at another way from now; or `same`")
        .output("told", "for `arranged`, where some were looking and some are: the sentence; else empty")
        .idempotent(),
        move |_, args| async move {
            let args = crate::begun::with(
                &args,
                "taken",
                &[("was", "was"), ("seats", "seats"), ("needed", "needed"), ("agents", "agents"), ("had", "had")],
            );
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let count = |key: &str| args.get(key).and_then(Value::as_i64).unwrap_or(0);
            let again = args.get("taken").is_some_and(|taken| taken["in_flight"] == "restart");
            let taken = Taken {
                column: text("column"),
                was: text("was"),
                now: text("now"),
                seats: count("seats"),
                needed: count("needed"),
                agents: count("agents"),
                had: count("had"),
                restart: args.get("restart").and_then(Value::as_bool).unwrap_or(again),
            };
            let came = changed(&taken).map_err(ActionError::new)?;
            let told = match &came {
                Changed::Arranged { told: Some(told) } => told.clone(),
                _ => String::new(),
            };
            Ok(json!({ "came": came.word(), "told": told }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "column.wrong",
            "Say what is wrong with a row that says what is done with the items of a column, each in a sentence: \
             a column it names that is not there, more to look or to approve than there are, nobody to work.",
        )
        .deciding()
        .arg("title", T::String, "what the row is called")
        .required("column", T::String, "the column whose items it is for")
        .arg("pass", T::String, "where an item goes on to")
        .arg("fail", T::String, "where an item is sent back to")
        .arg("found", T::Bool, "whether the columns there are could be read")
        .arg("columns", T::List, "the columns there are")
        .arg("role", T::String, "the role that does it")
        .arg("agents", T::Int, "how many there are of the role")
        .arg("seats", T::Int, "how many look at each item")
        .arg("needed", T::Int, "how many have to approve")
        .arg("reviews", T::Bool, "items are looked at here")
        .arg("works", T::Bool, "items are worked here")
        .arg("rows", T::List, "the rows of a table that lay more into what is done here, as `segments.next` takes them")
        .arg("usual", T::List, "what is usually done here, as `segments.next` takes it")
        .arg("of", T::Object, "which of those rows are this row's list, as `segments.next` takes it")
        .arg("row", T::Object, "the row, whole, in place of saying each thing of it: `title`, `pass`, `fail`, `role`, `agents`, `seats`, `needed`")
        .output("said", "what is wrong, a sentence each; none where the row can work. What is wrong with the rows of its list comes after what is wrong with the row itself")
        .output("about", "what each sentence about the row itself is about, in order: `astray`, `short`, `idle`")
        .idempotent(),
        move |_, args| async move {
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let columns = args.get("columns").and_then(Value::as_array).map(|columns| {
                columns.iter().filter_map(|column| column.as_str().map(str::to_string)).collect::<Vec<_>>()
            });
            let handed = crate::begun::with(
                &args,
                "row",
                &[
                    ("title", "title"),
                    ("pass", "pass"),
                    ("fail", "fail"),
                    ("role", "role"),
                    ("agents", "agents"),
                    ("seats", "seats"),
                    ("needed", "needed"),
                ],
            );
            let text = |key: &str| handed.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let count = |key: &str| handed.get(key).and_then(Value::as_i64).unwrap_or(0);
            let row = Row {
                title: text("title"),
                column: text("column"),
                pass: text("pass"),
                fail: text("fail"),
                columns: columns.filter(|_| flag("found")),
                role: text("role"),
                agents: count("agents"),
                seats: count("seats"),
                needed: count("needed"),
                reviews: flag("reviews"),
                works: flag("works"),
            };
            let found = wrong(&row).map_err(ActionError::new)?;
            let (about, mut said): (Vec<&str>, Vec<String>) = found.into_iter().unzip();
            // And what is wrong with the rows that lay more into what is done here.
            let (rows, usual) = (crate::segments::rows(&args), crate::segments::usual(&args));
            said.extend(crate::segments::wrong(&rows, &usual, &row.column, true));
            Ok(json!({ "said": said, "about": about }))
        },
    );
}
