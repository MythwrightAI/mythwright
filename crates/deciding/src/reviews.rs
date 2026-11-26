//! What an item that is to be looked at is owed, and what a verdict on
//! it comes to: what a text that has items reviewed asks, and does as it
//! is told.
//!
//! What a verdict comes to (the work merged first, the item on or back
//! and with what words, a person told that something is in the way) and
//! whether a review of an item begins were worked out in the text that
//! asks, in the holes of its strings: whether the item is still in its
//! column, whether there are enough to look, how often it has been sent
//! back against how often is allowed, and whether it has a review under
//! way already. Each is a function here, of what the row says, what the
//! text keeps and the event, with a table of cases that reaches every
//! word it can answer.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// What an item said to be due a review is asked of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Owed {
    /// The item, and whether it was found.
    pub item: String,
    pub found: bool,
    /// The column it is in, and the column whose items are looked at.
    pub status: String,
    pub here: String,
    /// How many there are to look, how many look at each, and how many
    /// have to approve.
    pub agents: i64,
    pub seats: i64,
    pub needed: i64,
    /// How often at most an item is sent back before it is left.
    pub most: i64,
    /// How often its row says it was sent back.
    pub rounds: i64,
    /// The items sent back from here, once for each time, as whoever
    /// asks keeps them: each the item, or a mapping that names it, which
    /// says `behind` where it went back only to be brought up to date.
    pub back: Vec<Value>,
    /// The items that have a review under way, or were left, as whoever
    /// asks keeps them: each a mapping that names the item, or the item.
    pub kept: Vec<Value>,
    /// The review of it says itself that another is to begin.
    pub again: bool,
    /// Something its column offers holds it there, and is not pressed
    /// (`crate::pressed`).
    pub held: bool,
    /// The role whose agents look, for the sentence of one nobody looks at.
    pub role: String,
    /// One agent named to review it, by a rung of the project's chain,
    /// where nobody of the role is there to: it reviews as one of the
    /// role would (`std/chain`).
    pub by: String,
}

/// What it comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwedCame {
    /// A review of it begins.
    Review,
    /// It has been sent back as often as is allowed, and is left for a
    /// person: how often, and the sentence that says so.
    Left { rounds: i64, said: String },
    /// It waits for something its column offers to be pressed: nobody is
    /// asked to look meanwhile.
    Held,
    /// Nobody is there to look, or too few: it waits for a person to
    /// review it, and the sentence that says so.
    Person { said: String },
    /// Nothing: it is gone or elsewhere, or it has a review under way, or
    /// was left or said to wait for a person already.
    Nothing,
}

impl OwedCame {
    pub fn word(&self) -> &'static str {
        match self {
            OwedCame::Review => "review",
            OwedCame::Left { .. } => "left",
            OwedCame::Held => "held",
            OwedCame::Person { .. } => "person",
            OwedCame::Nothing => "nothing",
        }
    }
}

/// Every word [`owed`] can answer.
pub const OWED_WORDS: &[&str] = &["review", "left", "held", "person", "nothing"];

/// Whether an entry of what is kept is of this item: a mapping that
/// names it as its `card`, or the item itself.
pub(crate) fn of(entry: &Value, item: &str) -> bool {
    match entry {
        Value::Object(entry) => entry.get("card").and_then(Value::as_str) == Some(item),
        other => other.as_str() == Some(item),
    }
}

/// How many times at most an item's branch is brought up to date with the
/// project's, having no longer merged once it was approved, before it is
/// left for a person: a project's branch that moves on in the way of the
/// same files each time would otherwise have it go round for ever.
pub const BEHIND: i64 = 3;

/// Whether an entry of the items sent back says it went back only to be
/// brought up to date with the project's branch (`behind`).
fn brought(entry: &Value) -> bool {
    matches!(entry.get("behind"), Some(Value::Bool(true)))
        || entry.get("behind").and_then(Value::as_str) == Some("true")
}

/// What an item said to be due a review is owed.
///
/// One that has a review under way, or was left for a person when it
/// came, is owed nothing more however often it is said to be due (what
/// looks at the column and the saying of its move may both find it),
/// but where its review says itself that another is to begin. One sent
/// back as often as is allowed is left for a person; otherwise a review
/// begins where there are enough to look and to approve.
pub fn owed(asked: &Owed) -> Result<OwedCame, String> {
    if asked.here.is_empty() {
        return Err("`here` is the column whose items are looked at, and none is said".into());
    }
    for (name, count) in [
        ("agents", asked.agents),
        ("seats", asked.seats),
        ("needed", asked.needed),
        ("most", asked.most),
        ("rounds", asked.rounds),
    ] {
        if count < 0 {
            return Err(format!("`{name}` is a count, and is not less than none; it is {count}"));
        }
    }
    if !asked.found {
        return Ok(OwedCame::Nothing);
    }
    if asked.status.is_empty() {
        return Err("the item was found, and its `status` says no column".into());
    }
    let under_way = asked.kept.iter().any(|entry| of(entry, &asked.item));
    if asked.status != asked.here || (under_way && !asked.again) {
        return Ok(OwedCame::Nothing);
    }
    if asked.held {
        return Ok(OwedCame::Held);
    }
    // Sent back only to be brought up to date with the project's branch,
    // it was not sent back for what was done, and that is not counted
    // with the times it was; but for as many times of its own.
    let (behind, counted): (Vec<&Value>, Vec<&Value>) =
        asked.back.iter().filter(|entry| of(entry, &asked.item)).partition(|entry| brought(entry));
    let rounds = asked.rounds.max(counted.len() as i64);
    let behind = behind.len() as i64;
    Ok(if rounds >= asked.most {
        let said = format!(
            "{} is in {} again after being sent back {rounds} times. Nobody was asked this time: it is left for a \
             person to look at.",
            asked.item, asked.here
        );
        OwedCame::Left { rounds, said }
    } else if behind >= BEHIND {
        let said = format!(
            "{} is in {} again after its branch was brought up to date with the project's {behind} times, having \
             no longer merged once it was approved. Nobody was asked this time: it is left for a person to look \
             at and merge.",
            asked.item, asked.here
        );
        OwedCame::Left { rounds: behind, said }
    } else if !asked.by.is_empty()
        || (asked.agents > 0 && asked.agents >= asked.seats && asked.needed <= asked.seats)
    {
        OwedCame::Review
    } else {
        OwedCame::Person { said: person(asked) }
    })
}

/// The sentence for an item nobody of the fleet is there to look at, or
/// too few: it waits for a person, and why.
fn person(asked: &Owed) -> String {
    let role = match asked.role.is_empty() {
        true => "the role that looks there".to_string(),
        false => format!("the role {}", asked.role),
    };
    let why = match asked.agents {
        0 => format!("nobody in the fleet has {role}"),
        agents if asked.seats > agents => {
            format!("its stage has {} look at each card, and {role} has {agents}", asked.seats)
        }
        _ => format!("its stage needs {} to approve, and {} look", asked.needed, asked.seats),
    };
    format!("{} is in {}, where {why}: it waits for a person to review it.", asked.item, asked.here)
}

/// What a verdict on an item is asked of, by whoever had it reviewed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Verdict {
    /// `passed`, `failed` or `undecided`.
    pub verdict: String,
    pub item: String,
    /// The column it is in, and the column whose items are looked at.
    pub status: String,
    pub here: String,
    /// Where an item goes on to, and where it is sent back to.
    pub pass: String,
    pub fail: String,
    /// What was done is put with the rest before the item goes on.
    pub merge: bool,
    /// What putting it with the rest came to, once that was asked:
    /// `merged`, `nothing`, `conflict`, `stuck`; empty before it is.
    pub merged: String,
    /// Asking that failed outright, and why, or why it is stuck.
    pub failed: bool,
    pub why: String,
    /// What was said of a conflict by what was asked.
    pub conflict: String,
    /// Where it no longer merged: the project's branch is being merged
    /// into the item's where it was worked, and these files conflict.
    pub begun: bool,
    pub files: Vec<String>,
    /// Who decided it, what they said, and how many of how many.
    pub by: String,
    pub words: String,
    pub approved: i64,
    pub against: i64,
    pub seats: i64,
    /// What reviews come to is said where the work is reported.
    pub reports: bool,
}

/// What a verdict comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerdictCame {
    /// What was done is to be put with the rest first; asked again with
    /// what that came to.
    Merge,
    /// The item goes on, with this said as it is moved.
    On(String),
    /// It goes back to whoever worked it, with this said, and is counted.
    Back(String),
    /// Something is in the way of putting it with the rest that a person
    /// has to put right: the sentence that says so.
    Stuck(String),
    /// Nobody approved it and nobody said what is wrong: it is left for a
    /// person, and the sentence that says so.
    Left(String),
    /// Nothing: it is no longer where it was looked at, or there is
    /// nowhere for it to go on to.
    Nothing,
}

impl VerdictCame {
    pub fn word(&self) -> &'static str {
        match self {
            VerdictCame::Merge => "merge",
            VerdictCame::On(_) => "on",
            VerdictCame::Back(_) => "back",
            VerdictCame::Stuck(_) => "stuck",
            VerdictCame::Left(_) => "left",
            VerdictCame::Nothing => "nothing",
        }
    }

    /// What goes with the word: what is said as the item is moved, or
    /// the sentence for where the work is reported.
    pub fn words(&self) -> &str {
        match self {
            VerdictCame::On(said)
            | VerdictCame::Back(said)
            | VerdictCame::Stuck(said)
            | VerdictCame::Left(said) => said,
            VerdictCame::Merge | VerdictCame::Nothing => "",
        }
    }
}

/// Every word [`verdict`] can answer.
pub const VERDICT_WORDS: &[&str] = &["merge", "on", "back", "stuck", "left", "nothing"];

/// What a verdict on an item comes to, and what is said of it where the
/// work is reported, if its row says verdicts are said there (nothing,
/// otherwise).
pub fn verdict(asked: &Verdict) -> Result<(VerdictCame, String), String> {
    if asked.here.is_empty() {
        return Err("`here` is the column whose items are looked at, and none is said".into());
    }
    for (name, count) in [("approved", asked.approved), ("against", asked.against), ("seats", asked.seats)] {
        if count < 0 {
            return Err(format!("`{name}` is a count, and is not less than none; it is {count}"));
        }
    }
    let item = &asked.item;
    let several = asked.seats > 1;
    let said = if asked.words.is_empty() { ".".to_string() } else { format!(": {}", asked.words) };
    let there = asked.status == asked.here;
    match asked.verdict.as_str() {
        "passed" => {
            let told = if asked.reports {
                let who = if several {
                    format!(", {} of {}", asked.approved, asked.seats)
                } else {
                    format!(" by {}", asked.by)
                };
                format!("{item} was approved{who}{said}")
            } else {
                String::new()
            };
            let of = if several { format!(", {} of {}", asked.approved, asked.seats) } else { String::new() };
            let on = |merged: bool| {
                let was = if merged { format!("; lane/{item} was merged") } else { String::new() };
                VerdictCame::On(format!("approved by {}{of}{was}", asked.by))
            };
            let came = if asked.pass.is_empty() || !there {
                VerdictCame::Nothing
            } else if !asked.merge {
                on(false)
            } else if asked.failed {
                stuck(asked, "the merge failed")
            } else {
                match asked.merged.as_str() {
                    "" => VerdictCame::Merge,
                    "merged" => on(true),
                    "nothing" => on(false),
                    "conflict" if asked.begun => VerdictCame::Back(behind(asked)),
                    "conflict" if !asked.why.is_empty() => stuck(asked, ""),
                    "conflict" => VerdictCame::Back(format!(
                        "It was approved, and its branch lane/{item} no longer merges into the project's branch as \
                         that is now. Merge the project's branch into yours, put right what conflicts, and hand it \
                         in again. What git said: {}",
                        asked.conflict
                    )),
                    "stuck" => stuck(asked, "the merge failed"),
                    other => {
                        return Err(format!(
                            "`merged` is what putting the work with the rest came to (`merged`, `nothing`, \
                             `conflict` or `stuck`), and is `{other}`"
                        ));
                    }
                }
            };
            Ok((came, told))
        }
        "failed" => {
            let told = if asked.reports && there {
                let of = if several {
                    format!(", {} of {} against", asked.against, asked.seats)
                } else {
                    String::new()
                };
                format!("{item} was sent back to {}{of}{said}", asked.fail)
            } else {
                String::new()
            };
            let came = if there { VerdictCame::Back(asked.words.clone()) } else { VerdictCame::Nothing };
            Ok((came, told))
        }
        "undecided" => {
            let why = if asked.words.is_empty() { String::new() } else { format!(" ({})", asked.words) };
            let left = format!(
                "{item}: those who looked at it neither approved it nor sent it back{why}. It is left in {} for a \
                 person.",
                asked.here
            );
            Ok((VerdictCame::Left(left), String::new()))
        }
        other => Err(format!("`verdict` is `passed`, `failed` or `undecided`, and is `{other}`")),
    }
}

/// How many files that conflict are named in what whoever works an item
/// is told; git names the rest.
const NAMED: usize = 20;

/// What whoever works an item is told where its branch no longer merged,
/// and the project's branch is being merged into it for them.
fn behind(asked: &Verdict) -> String {
    let item = &asked.item;
    let mut files = asked.files.iter().take(NAMED).map(String::as_str).collect::<Vec<_>>().join(", ");
    if asked.files.len() > NAMED {
        files = format!(
            "{files} and {} more (`git diff --name-only --diff-filter=U` lists them all)",
            asked.files.len() - NAMED
        );
    }
    let said = if asked.conflict.trim().is_empty() {
        String::new()
    } else {
        format!(" What git said: {}", asked.conflict.trim())
    };
    format!(
        "It was approved, and its branch lane/{item} no longer merges into the project's branch as that is now. \
         The project's branch is being merged into yours where you work, and that merge is under way for you: \
         what conflicts is marked in {files}, as git marks it. Put each right, keeping what both sides meant, and \
         leave no marker. Change files and nothing else: do not run git to commit, merge or call the merge off. \
         What you leave is committed as the end of that merge, and it is looked at again.{said}"
    )
}

/// The sentence for a merge something is in the way of.
fn stuck(asked: &Verdict, otherwise: &str) -> VerdictCame {
    let why = if asked.why.is_empty() { otherwise } else { &asked.why };
    VerdictCame::Stuck(format!(
        "{} was approved by {}, and its branch lane/{} could not be merged into the project's: {why}. It stays in \
         {}; when what is in the way is put right, merge it and move it on.",
        asked.item, asked.by, asked.item, asked.here
    ))
}

/// A review that is over having said nothing of how it came out (it was
/// stopped, or it failed, or it kept nothing and went with the daemon):
/// the item whose review it is, as whoever asks kept it, and (asked
/// again with how the item now is) whether it waits for a person in the
/// column, which is so where it is there still and this was the review
/// kept for it. One another review
/// took the place of (begun again as reviews are now arranged) is no
/// longer kept under this one's name, and one whose item has gone on is
/// waiting for nobody here.
pub fn over(kept: &[Value], review: &str, found: bool, status: &str, here: &str) -> (String, Option<String>) {
    let entry = kept.iter().find(|entry| !review.is_empty() && entry["review"].as_str() == Some(review));
    let item = entry.and_then(|entry| entry["card"].as_str()).unwrap_or_default().to_string();
    let waits = !item.is_empty() && found && !here.is_empty() && status == here;
    let said = waits.then(|| {
        format!(
            "{item} is in {here}, and its review ended with nothing said of how it came out (it was stopped, or \
             it failed): it waits for a person to review it."
        )
    });
    (item, said)
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "review.owed",
            "Say what an item said to be due a review is owed: a review of it begins, it is left for a person \
             having been sent back as often as is allowed, or nothing.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .required("found", T::Bool, "whether it was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are looked at")
        .arg("agents", T::Int, "how many there are to look")
        .arg("seats", T::Int, "how many look at each item")
        .arg("needed", T::Int, "how many have to approve")
        .arg("most", T::Int, "how often at most an item is sent back before it is left")
        .arg("rounds", T::Int, "how often its row says it was sent back")
        .arg("back", T::List, "the items sent back from here, once for each time, as whoever asks keeps them; one that says `behind` went back only to be brought up to date with the project's branch, and is counted apart")
        .arg("kept", T::List, "the items that have a review under way or were left, as whoever asks keeps them")
        .arg("again", T::Bool, "its review says itself that another is to begin")
        .arg("by", T::String, "one agent a rung of the project's chain names to review it, where nobody of the role is there to; only the review that ships is handed one")
        .output("came", "`review`, one begins; `left`, it is left for a person; `held`, it waits for something its column offers to be pressed; `person`, nobody is there to look or too few, and it waits for a person to review it; `nothing`")
        .arg("pressed", T::List, "what the item keeps of what was pressed on it; what can be pressed is the `actions` of `own`")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .output("hold", "for `held`: which coming of it this is (`at`), what is still to be pressed (`waits`) and the sentence (`says`)")
        .output("rounds", "for `left`: how often it was sent back")
        .output("said", "for `left` and `person`: the sentence that says so")
        .arg("row", T::String, "where in the things done here its looking is, where whoever says it is due said: the row")
        .arg("order", T::Int, "and the order")
        .output("row", "where its looking is among the things done here: as was said, or as whoever asks kept it for the looking that says another is to begin")
        .output("order", "and the order; 0, where it is the first of them")
        .arg("rows", T::List, "the rows of a table that lay more into the things done here, as `segments.next` takes them")
        .arg("usual", T::List, "what is usually done here, as `segments.next` takes it")
        .arg("of", T::Object, "which rows are this list's, as `segments.next` takes it")
        .arg("labels", T::List, "the labels the item holds")
        .arg("part_of", T::String, "what it is part of")
        .arg("own", T::Object, "what whoever asks says of itself: who looks (`role`, `picks`) and how many (`seats`, `needed`); and, where they are not said by name, how many there are to look (`agents`) and how often at most an item is sent back (`rounds`); and what can be pressed on an item, of every column (`actions`)")
        .arg("said", T::Object, "what was said as the item was said to be due, whole, in place of saying each thing of it: `again`, `row`, `order`")
        .output("lists", "something is called before the looking, or nothing is begun: the list is gone through from `row` and `order`")
        .output("usual", "only what is usual comes after the looking")
        .output("with", "who looks and how many have to approve, as the row of this looking says and as whoever asks says where it does not")
        .idempotent(),
        move |_, args| async move {
            let own =
                [("agents", "agents"), ("seats", "seats"), ("needed", "needed"), ("most", "rounds"), ("role", "role")];
            let args = crate::begun::with(&args, "own", &own);
            let args = crate::begun::with(&args, "said", &[("again", "again"), ("row", "row"), ("order", "order")]);
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let count = |key: &str| args.get(key).and_then(Value::as_i64).unwrap_or(0);
            let list = |key: &str| args.get(key).and_then(Value::as_array).cloned().unwrap_or_default();
            let asked = Owed {
                item: text("item"),
                found: flag("found"),
                status: text("status"),
                here: text("here"),
                agents: count("agents"),
                seats: count("seats"),
                needed: count("needed"),
                most: count("most"),
                rounds: count("rounds"),
                back: list("back"),
                kept: list("kept"),
                again: flag("again"),
                held: false,
                role: text("role"),
                by: text("by"),
            };
            // An agent can be named only to the review that ships, which
            // its stage tells what becomes of its agents at a restart: a
            // project's own is given what it always was, and nothing more.
            let ships = args.get("own").is_some_and(|own| own["restarting"] == true);
            let asked = Owed { by: if ships { asked.by } else { String::new() }, ..asked };
            if !ships && !text("by").is_empty() {
                return Ok(json!({ "came": "nothing", "rounds": 0, "said": "" }));
            }
            let hold = crate::pressed::of(&args, &asked.item, &asked.here);
            let asked = Owed { held: hold.is_some(), ..asked };
            let came = owed(&asked).map_err(ActionError::new)?;
            let (rounds, said) = match &came {
                OwedCame::Left { rounds, said } => (*rounds, said.clone()),
                OwedCame::Person { said } => (0, said.clone()),
                _ => (0, String::new()),
            };
            // A looking that says another is to begin is begun where it
            // was, which whoever asks kept with it.
            let was = asked.kept.iter().find(|entry| of(entry, &asked.item)).filter(|_| asked.again);
            let (mut row, mut order) = (text("row"), count("order"));
            if let (Some(was), true) = (was, row.is_empty() && order == 0) {
                row = was["row"].as_str().unwrap_or_default().to_string();
                order = was["order"].as_i64().unwrap_or(0);
            }
            // Which of the things done here the looking is, and whether
            // anything is called before it for this item.
            let mut first = crate::begun::first_of(&args, &asked.item, &row, order);
            // Named by a rung, one agent looks and decides alone, of
            // whatever role it is: the stage's has nobody.
            if !asked.by.is_empty() {
                first.with.insert("agent".into(), json!(asked.by));
                first.with.insert("role".into(), json!(""));
                first.with.insert("seats".into(), json!(1));
                first.with.insert("needed".into(), json!(1));
            }
            Ok(json!({
                "came": came.word(), "rounds": rounds, "said": said, "row": first.row, "order": first.order,
                "lists": first.lists, "usual": first.usual, "with": first.with,
                "hold": hold.map(|hold| hold.value()).unwrap_or_default(),
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "review.verdict",
            "Say what a verdict on an item comes to: what was done is put with the rest first, the item goes on, \
             it goes back, something is in the way that a person has to put right, it is left for a person, or \
             nothing; and what is said of it where the work is reported.",
        )
        .deciding()
        .required("verdict", T::String, "`passed`, `failed` or `undecided`")
        .required("item", T::String, "the item")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items are looked at")
        .arg("pass", T::String, "where an item goes on to")
        .arg("fail", T::String, "where an item is sent back to")
        .arg("merge", T::Bool, "what was done is put with the rest before the item goes on")
        .arg("merged", T::String, "what putting it with the rest came to, once asked; empty before")
        .arg("failed", T::Bool, "asking that failed outright")
        .arg("why", T::String, "why it failed, or why it is stuck")
        .arg("conflict", T::String, "what was said of a conflict")
        .arg("begun", T::Bool, "where it no longer merged: the project's branch is being merged into the item's where it was worked, for whoever works it")
        .arg("files", T::List, "and the files that conflict there")
        .arg("by", T::String, "who decided it")
        .arg("words", T::String, "what they said")
        .arg("approved", T::Int, "how many approved")
        .arg("against", T::Int, "how many sent it back")
        .arg("seats", T::Int, "how many looked")
        .arg("reports", T::Bool, "what reviews come to is said where the work is reported")
        .output("came", "`merge`, `on`, `back`, `stuck`, `left` or `nothing`")
        .output("says", "for `on` and `back`: what is said as the item is moved; for `stuck` and `left`: the sentence")
        .output("told", "what is said of the verdict where the work is reported; empty where nothing is")
        .output("word", "the same as the word a thing done with an item ends with: `on`, `back`, `left` (for `stuck` too), `nothing`; empty for `merge`")
        .output("behind", "for `back`: it goes back only to be brought up to date with the project's branch, which is no verdict on it and is not counted among the times it was sent back")
        .idempotent(),
        move |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
            let count = |key: &str| args.get(key).and_then(Value::as_i64).unwrap_or(0);
            let asked = Verdict {
                verdict: text("verdict"),
                item: text("item"),
                status: text("status"),
                here: text("here"),
                pass: text("pass"),
                fail: text("fail"),
                merge: flag("merge"),
                merged: text("merged"),
                failed: flag("failed"),
                why: text("why"),
                conflict: text("conflict"),
                begun: flag("begun"),
                files: args
                    .get("files")
                    .and_then(Value::as_array)
                    .map(|files| files.iter().filter_map(Value::as_str).map(str::to_string).collect())
                    .unwrap_or_default(),
                by: text("by"),
                words: text("words"),
                approved: count("approved"),
                against: count("against"),
                seats: args.get("seats").and_then(Value::as_i64).unwrap_or(1),
                reports: flag("reports"),
            };
            let (came, told) = verdict(&asked).map_err(ActionError::new)?;
            let word = crate::decided::word(&came);
            let behind = matches!(came, VerdictCame::Back(_)) && asked.merged == "conflict";
            Ok(json!({ "came": came.word(), "says": came.words(), "told": told, "word": word, "behind": behind }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "review.over",
            "Say, of a review that is over having said nothing of how it came out, whether its item now waits for \
             a person to review it, and the sentence that says so.",
        )
        .deciding()
        .arg("kept", T::List, "the items that have a review under way, as whoever asks keeps them: each a mapping with the `card` and the `review`")
        .required("review", T::String, "what the review that is over was called")
        .arg("found", T::Bool, "whether its item was found")
        .arg("status", T::String, "the column the item is in")
        .required("here", T::String, "the column whose items are looked at")
        .output("item", "the item whose review it was, as whoever asks kept it; empty where none is kept under that name, another having taken its place")
        .output("waits", "with `found` and `status`: whether the item waits for a person to review it, being here still")
        .output("said", "the sentence that says so")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
            let kept = args.get("kept").and_then(Value::as_array).cloned().unwrap_or_default();
            let found = args.get("found").and_then(Value::as_bool).unwrap_or(false);
            let (item, said) = over(&kept, &text("review"), found, &text("status"), &text("here"));
            Ok(json!({ "item": item, "waits": said.is_some(), "said": said.unwrap_or_default() }))
        },
    );
}
