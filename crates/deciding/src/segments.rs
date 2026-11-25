//! What is done with an item where it is, as a list of small things done
//! one after another, and what comes next.
//!
//! Whoever looks after the items of a column does a usual few things with
//! each (a review, say), and a table of the project may say more: a row
//! names something to be done (`does`, a blueprint), whose column's list it
//! is in is whoever asks to say, and where in it (`order`). The rows are changes to the
//! usual list and not the list: a row at an order between two usual things
//! goes between them, a row at the order of a usual thing stands in for
//! it, and one there that names nothing takes it out. A row may be held to
//! one item, to items with a label or to items that are part of something;
//! of two rows at one order, the one held to more stands.
//!
//! Each thing in the list is a *segment*: a blueprint that is handed the
//! item and what it declares of what was said, and that ends in a word.
//! [`next`] says, from where an item is in the list and the word the
//! segment there came to, what comes next: another segment is run, the
//! item goes on, it goes back, it is left for a person, or nothing, the
//! item having gone on or been moved meanwhile. [`fitted`] says what a
//! segment is handed, by what it declares. [`ran`] says what a command's
//! ending comes to, [`recorded`] what is kept of it as a row of a table,
//! and [`wrong`] what is wrong with a list as written.
//! Each is a function of what it is handed, with a table of cases that
//! reaches every word it can answer; none knows what an item is, or what
//! the column is for.
//!
//! What a segment under way was begun with is what was answered here when
//! it was begun, which whoever asked keeps and carries on: a list that
//! changes while a segment is under way changes what comes after it.
//!
//! A list is gone through one of two ways. Whoever asks may say each
//! time where the item is and what the segment there came to; or it may
//! go through the whole of it in one piece of work that keeps nothing,
//! asking once in full and then handing each asking what the one before
//! answered (`was`) and what the segment gave out (`gave`), as a round
//! does for an item in hand (by `std/through`, which asks, calls what it
//! is told and calls itself with what that answered). There an answer
//! that is not another thing to run is answered again as it was, so that
//! an asking made once the list has ended says what the whole came to,
//! and what a segment gives out beside its word is handed on to those
//! after it. Each asking says which thing of the going through it calls
//! (`nth`), and the one that would call more than whoever asks does at
//! once (`most_done`) leaves the item for a person instead ([`beyond`]).

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::add;

mod fitted;
mod gave;
mod most;
mod recorded;
mod run_in;
mod wrong;

pub use fitted::{COUNCIL, NOT_HANDED, as_kind, coming, council, fitted};
use gave::WORDS_GIVEN;
pub use gave::{Gave, RAN_WORDS, Ran, gave, ran};
pub use most::beyond;
pub use recorded::{LAST_LINES, MOST_PRINTED, RESULTS, Recorded, recorded};
pub use run_in::{IN_WORDS, RunIn, run_in};
pub use wrong::{GOES_THROUGH, goes_through, wrong};

/// Every word [`next`] can answer.
pub const WORDS: &[&str] = &["run", "on", "back", "asks", "left", "nothing"];

/// How often a segment is given again where nobody says.
const MOST: i64 = 2;

/// One of the things usually done with an item where it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usual {
    pub order: i64,
    /// The blueprint that does it.
    pub does: String,
    /// It is begun and heard from, and not called and waited for: what
    /// begins it is whoever asks, with what its own row says.
    pub spawned: bool,
}

/// A row of the table, as it was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Said {
    pub id: String,
    pub title: String,
    /// Whose list it is in, by what that one is called.
    pub order: i64,
    /// The blueprint it names; nothing, to take out what is usual there.
    pub does: String,
    /// What it says that a segment may be handed, cell by cell.
    pub cells: Map<String, Value>,
    /// And anything else, each `name=value`.
    pub with: Vec<String>,
    /// How often its segment is given again at most.
    pub rounds: i64,
    /// What it is held to: one item, items with a label, items that are
    /// part of something.
    pub item: String,
    pub labelled: String,
    pub part_of: String,
}

impl Said {
    /// A row out of what `rows.read` gives.
    pub fn read(row: &Value) -> Said {
        let text = |key: &str| match &row[key] {
            Value::String(text) => text.trim().to_string(),
            Value::Null => String::new(),
            other => other.to_string(),
        };
        let number = |key: &str| match &row[key] {
            Value::Number(number) => {
                number.as_i64().or_else(|| number.as_f64().map(|n| n as i64)).unwrap_or(0)
            }
            Value::String(text) => text.trim().parse().unwrap_or(0),
            _ => 0,
        };
        let cells = CELLS
            .iter()
            .filter(|cell| match &row[**cell] {
                Value::Null => false,
                Value::String(text) => !text.trim().is_empty(),
                Value::Array(all) => all.iter().any(|one| !blueprint::expr::display(one).trim().is_empty()),
                _ => true,
            })
            .map(|cell| (cell.to_string(), row[*cell].clone()))
            .collect();
        let with = row["with"]
            .as_array()
            .map(|all| {
                all.iter().map(blueprint::expr::display).filter(|one| !one.trim().is_empty()).collect()
            })
            .unwrap_or_default();
        Said {
            id: text("id"),
            title: text("title"),
            order: number("order"),
            does: text("does"),
            cells,
            with,
            rounds: number("rounds"),
            item: text("item"),
            labelled: text("labelled"),
            part_of: text("part_of"),
        }
    }

    /// How much it is held to, and whether it is this item's at all.
    fn held(&self, asked: &Asked) -> Option<usize> {
        let holds = [
            (&self.item, self.item == asked.item),
            (&self.labelled, asked.labels.contains(&self.labelled)),
            (&self.part_of, self.part_of == asked.part_of),
        ];
        let said: Vec<bool> = holds.iter().filter(|(word, _)| !word.is_empty()).map(|(_, is)| *is).collect();
        said.iter().all(|is| *is).then_some(said.len())
    }
}

/// The cells of a row that a segment may be handed, where it declares
/// a param of that name.
pub const CELLS: &[&str] =
    &["role", "picks", "seats", "needed", "ends", "yes", "no", "command", "record_to", "warn_after"];

/// The cells of a row that say a time (`20m`), which a segment is handed
/// only where it is one.
pub const TIMES: &[&str] = &["warn_after"];

/// What [`next`] is asked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asked {
    pub rows: Vec<Said>,
    pub usual: Vec<Usual>,
    pub item: String,
    pub labels: Vec<String>,
    pub part_of: String,
    /// Whether the item was found, the column it is in, and the column
    /// whose items these are.
    pub found: bool,
    pub status: String,
    pub here: String,
    /// The column an item is in while it is in hand, where whoever asks
    /// takes items out of `here` to do these things with them: an item
    /// there is still this list's.
    pub doing: String,
    /// How many times the item has been moved, now; and how many it had
    /// been when this going through the list began, where that was said.
    pub moved: i64,
    pub moves: i64,
    /// The segment at `row` and `order` is to be begun, or the first of
    /// the list where neither is said; otherwise it has ended, with `came`.
    pub begin: bool,
    pub row: String,
    pub order: i64,
    /// What the segment there came to: `on` (and so is nothing), `back`,
    /// `asks`, `again`, `left`. And whether it could not be called at all.
    pub came: String,
    pub failed: bool,
    pub why: String,
    pub says: String,
    pub asker: String,
    pub question: String,
    /// Whether whoever asks can take up what a segment asks.
    pub asking: bool,
    /// How often the segment there has been given again, and how often
    /// one is at most where its row says nothing.
    pub tries: i64,
    pub most: i64,
    /// How often the item has been given to whoever works it again, and
    /// how often it is at most, where whoever asks counts that: one that
    /// is sent back as often is left for a person.
    pub given: i64,
    pub most_given: i64,
    /// Which thing of this going through the list would be called by
    /// this asking, from 1; how many at most are called in one going
    /// through, where whoever asks says (none, if 0); and the record that
    /// says so, for the sentence that says how to raise it.
    pub nth: i64,
    pub most_done: i64,
    pub most_of: String,
}

/// A segment that is to be run.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    /// The row that names it; none, for one of the usual.
    pub row: String,
    pub title: String,
    pub order: i64,
    pub name: String,
    pub spawned: bool,
    pub tries: i64,
    pub cells: Map<String, Value>,
    pub with: Vec<String>,
}

/// What comes next.
#[derive(Clone, Debug, PartialEq)]
pub enum Came {
    Run(Run),
    /// Nothing is left of the list: the item goes on.
    On {
        says: String,
    },
    /// It goes back, with what was said.
    Back {
        says: String,
    },
    /// The segment asked something, and whoever asks takes that up.
    Asks {
        asker: String,
        question: String,
    },
    /// It is left for a person, and this says why.
    Left {
        says: String,
    },
    /// The item is gone, has gone on, or was moved since this began.
    Nothing,
}

impl Came {
    pub fn word(&self) -> &'static str {
        match self {
            Came::Run(_) => "run",
            Came::On { .. } => "on",
            Came::Back { .. } => "back",
            Came::Asks { .. } => "asks",
            Came::Left { .. } => "left",
            Came::Nothing => "nothing",
        }
    }
}

/// One thing in the list as it is laid out.
#[derive(Clone, Debug, PartialEq)]
struct Entry {
    row: String,
    title: String,
    order: i64,
    name: String,
    spawned: bool,
    rounds: i64,
    cells: Map<String, Value>,
    with: Vec<String>,
}

impl Entry {
    fn run(&self, tries: i64) -> Run {
        Run {
            row: self.row.clone(),
            title: self.title.clone(),
            order: self.order,
            name: self.name.clone(),
            spawned: self.spawned,
            tries,
            cells: self.cells.clone(),
            with: self.with.clone(),
        }
    }
}

/// The usual list with the rows that are this item's laid into it, in order.
fn laid(asked: &Asked) -> Vec<Entry> {
    let mut list: Vec<Entry> = asked
        .usual
        .iter()
        .map(|usual| Entry {
            row: String::new(),
            title: usual.does.clone(),
            order: usual.order,
            name: usual.does.clone(),
            spawned: usual.spawned,
            rounds: 0,
            cells: Map::new(),
            with: Vec::new(),
        })
        .collect();
    // Of the rows at one order, the one held to most; of those, the first read.
    let mut stand: Vec<(usize, &Said)> = Vec::new();
    for row in &asked.rows {
        let Some(held) = row.held(asked) else { continue };
        match stand.iter_mut().find(|(_, other)| other.order == row.order) {
            Some(there) if there.0 < held => *there = (held, row),
            Some(_) => {}
            None => stand.push((held, row)),
        }
    }
    for (_, row) in stand {
        list.retain(|entry| entry.order != row.order);
        if row.does.is_empty() {
            continue;
        }
        list.push(Entry {
            row: row.id.clone(),
            title: row.title.clone(),
            order: row.order,
            name: row.does.clone(),
            // A row that names what is usually begun and heard from here
            // is one more of those, begun with what the row says.
            spawned: asked.usual.iter().any(|usual| usual.spawned && usual.does == row.does),
            rounds: row.rounds,
            cells: row.cells.clone(),
            with: row.with.clone(),
        });
    }
    list.sort_by_key(|entry| entry.order);
    list
}

/// Where the item is in the list: by the row, and by its order where the
/// row names nothing there any more.
fn at(list: &[Entry], asked: &Asked) -> Option<usize> {
    let by_row = list.iter().position(|entry| !asked.row.is_empty() && entry.row == asked.row);
    by_row.or_else(|| list.iter().position(|entry| entry.order == asked.order))
}

/// What comes after where the item is.
fn after<'a>(list: &'a [Entry], asked: &Asked) -> Option<&'a Entry> {
    match at(list, asked) {
        Some(here) => list.get(here + 1),
        None => list.iter().find(|entry| entry.order > asked.order),
    }
}

/// Whether everything after `run` in the list is what is usual, no row
/// having laid anything there or taken anything out: whoever asks may
/// then do what is usual after it by itself.
pub fn rest_is_usual(asked: &Asked, run: &Run) -> bool {
    let rows_after =
        asked.rows.iter().filter(|row| row.held(asked).is_some()).any(|row| row.order > run.order);
    !rows_after
}

/// The list from where an asking begins (`row` and `order`, or the first
/// of it where neither is said), each thing as it would be run: for
/// whoever begins and hears from something itself, and has to know
/// whether anything is called before it.
pub fn ahead(asked: &Asked) -> Vec<Run> {
    let list = laid(asked);
    let from = match (asked.row.is_empty(), asked.order) {
        (true, 0) => 0,
        _ => at(&list, asked)
            .or_else(|| list.iter().position(|entry| entry.order > asked.order))
            .unwrap_or(list.len()),
    };
    list[from..].iter().map(|entry| entry.run(0)).collect()
}

/// A segment that is begun: called, where this going through has not
/// called as many as it calls at most already, and otherwise the item is
/// left for a person ([`beyond`]). One that whoever asks begins and hears
/// from is not called here, and is not counted.
fn begun(entry: &Entry, asked: &Asked, tries: i64) -> Came {
    // One that goes through the list itself is not run inside it.
    if GOES_THROUGH.contains(&entry.name.as_str()) && !entry.row.is_empty() {
        return Came::Left { says: format!("{}: {}", asked.item, goes_through(&entry.title, &entry.name)) };
    }
    if !entry.spawned && asked.most_done > 0 && asked.nth > asked.most_done {
        let says = beyond(&asked.item, &asked.here, &entry.title, asked.most_done, &asked.most_of);
        return Came::Left { says };
    }
    Came::Run(entry.run(tries))
}

/// What comes next for an item, from where it is in the list and what the
/// segment there came to.
pub fn next(asked: &Asked) -> Result<Came, String> {
    let in_hand = !asked.doing.is_empty() && asked.status == asked.doing;
    if !asked.found || (asked.status != asked.here && !in_hand) {
        return Ok(Came::Nothing);
    }
    if asked.moves != 0 && asked.moves != asked.moved {
        return Ok(Came::Nothing);
    }
    let list = laid(asked);
    let on = || Came::On { says: asked.says.clone() };
    if asked.begin {
        let entry = match (asked.row.is_empty(), asked.order) {
            (true, 0) => list.first(),
            _ => at(&list, asked).and_then(|here| list.get(here)).or_else(|| after(&list, asked)),
        };
        return Ok(entry.map_or_else(on, |entry| begun(entry, asked, asked.tries)));
    }
    let here = at(&list, asked).and_then(|here| list.get(here));
    let title = here.map_or(asked.row.as_str(), |entry| entry.title.as_str());
    if asked.failed {
        let why = if asked.why.is_empty() { "it could not be called" } else { asked.why.as_str() };
        return Ok(Came::Left { says: format!("{}: {title} could not be done: {why}", asked.item) });
    }
    match asked.came.as_str() {
        "" | "on" => Ok(after(&list, asked).map_or_else(on, |entry| begun(entry, asked, 0))),
        "back" if asked.most_given > 0 && asked.given >= asked.most_given => {
            let times =
                if asked.given == 1 { "1 time".to_string() } else { format!("{} times", asked.given) };
            let says = format!(
                "{}: {title} sent it back, and it has been given again {times} already: it is left for a person. {}",
                asked.item, asked.says
            );
            Ok(Came::Left { says: says.trim_end().to_string() })
        }
        "back" => Ok(Came::Back { says: asked.says.clone() }),
        "left" => Ok(Came::Left { says: asked.says.clone() }),
        // What was to be done found the item gone from under it.
        "nothing" => Ok(Came::Nothing),
        "asks" if asked.asking => {
            Ok(Came::Asks { asker: asked.asker.clone(), question: asked.question.clone() })
        }
        "asks" => {
            let says =
                format!("{}: {title} asks, and it is left for a person: {}", asked.item, asked.question);
            Ok(Came::Left { says })
        }
        "again" => {
            let Some(entry) = here else {
                return Ok(after(&list, asked).map_or_else(on, |entry| begun(entry, asked, 0)));
            };
            let most = [entry.rounds, asked.most, MOST].into_iter().find(|most| *most > 0).unwrap_or(MOST);
            if asked.tries >= most {
                let why = if asked.why.is_empty() { String::new() } else { format!(": {}", asked.why) };
                let says = format!(
                    "{}: {title} was given again {most} times and is left for a person{why}",
                    asked.item
                );
                return Ok(Came::Left { says });
            }
            Ok(begun(entry, asked, asked.tries + 1))
        }
        other => Err(format!(
            "{title} came to `{other}` for {}, which is none of the words a segment ends with: `on`, `back`, \
             `asks`, `again`, `left`, `nothing`",
            asked.item
        )),
    }
}

/// What every segment is handed by whoever calls it, and no row says.
pub const ALWAYS: &[&str] = &["project", "item"];

pub(crate) fn text(args: &kernel::Fields, key: &str) -> String {
    args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

pub(crate) fn flag(args: &kernel::Fields, key: &str) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(crate) fn number(args: &kernel::Fields, key: &str) -> i64 {
    match args.get(key) {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// The rows that are this list's: those that say, cell for cell, what
/// whoever asks says a row of its own says (`of`). A row that says
/// nothing where whoever asks says nothing either is nobody's.
pub(crate) fn rows(args: &kernel::Fields) -> Vec<Said> {
    let of = mapping(args, "of");
    let own = |row: &&Value| {
        of.iter().all(|(cell, is)| !is.as_str().is_some_and(str::is_empty) && row[cell.as_str()] == *is)
    };
    let all = args.get("rows").and_then(Value::as_array);
    all.map(|rows| rows.iter().filter(own).map(Said::read).collect()).unwrap_or_default()
}

pub(crate) fn usual(args: &kernel::Fields) -> Vec<Usual> {
    let one = |said: &Value| Usual {
        order: said["order"].as_i64().unwrap_or(0),
        does: said["does"].as_str().unwrap_or_default().to_string(),
        spawned: said["spawned"].as_bool().unwrap_or(false),
    };
    // One that says it is done only where something is so (`only`) is
    // left out where that is not.
    let stands = |said: &&Value| said["only"] != false && said["only"] != "false";
    let all = args.get("usual").and_then(Value::as_array);
    all.map(|all| all.iter().filter(stands).map(one).collect()).unwrap_or_default()
}

pub(crate) fn mapping(args: &kernel::Fields, key: &str) -> Map<String, Value> {
    args.get(key).and_then(Value::as_object).cloned().unwrap_or_default()
}

/// What a blueprint declares that it takes, or why that cannot be read.
fn declared(source: &str) -> Result<Vec<(String, ValueType)>, String> {
    let checked = blueprint::check(source, &blueprint::NoCatalog);
    match checked.blueprint {
        Some(read) => Ok(read.params.iter().map(|param| (param.name.clone(), param.ty)).collect()),
        None => {
            let wrong: Vec<String> = checked.errors().map(|problem| problem.message.clone()).collect();
            Err(format!("does not read as a blueprint: {}", wrong.join("; ")))
        }
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    use ValueType as T;
    // What is begun for an item by whoever goes through such a list, and what that came to.
    crate::begun::register(registry);
    add(
        registry,
        ActionSpec::new(
            "segments.next",
            "Say what comes next for an item among the things done with it where it is: the usual ones, with what \
             the rows of a table add, stand in for and take out. Another is run, with what it is handed; the item \
             goes on, goes back or is left for a person; or nothing, it having gone on or been moved.",
        )
        .deciding()
        .arg("rows", T::List, "the rows of the table, as `rows.read` gives them")
        .arg("usual", T::List, "what is usually done, each with its `order`, the blueprint that `does` it, and whether it is `spawned`")
        .arg("of", T::Object, "which rows are this list's: the cells a row of it says, and what each says")
        .required("item", T::String, "the item")
        .arg("labels", T::List, "the labels it holds")
        .arg("part_of", T::String, "what it is part of")
        .required("found", T::Bool, "whether the item was found")
        .arg("status", T::String, "the column it is in")
        .required("here", T::String, "the column whose items these are")
        .arg("moved", T::List, "how its row says it came to be where it is")
        .arg("moves", T::Int, "how many times it had been moved when this began, where that was said")
        .arg("begin", T::Bool, "the one at `row` and `order` is to be begun, or the first where neither is said")
        .arg("row", T::String, "the row of the one the item is at")
        .arg("order", T::Int, "and its order")
        .arg("came", T::String, "what that one came to: `on`, `back`, `asks`, `again`, `left`; `on`, if not said")
        .arg("failed", T::Bool, "it could not be called at all")
        .arg("why", T::String, "why not, or why it is to be given again")
        .arg("says", T::String, "what it said with the word")
        .arg("asker", T::String, "who asked, where it asks")
        .arg("question", T::String, "and what")
        .arg("asking", T::Bool, "whether whoever asks takes up what a segment asks")
        .arg("tries", T::Int, "how often the one there has been given again")
        .arg("most", T::Int, "how often one is at most, where its row says nothing")
        .arg("own", T::Object, "what whoever asks says of itself, handed on where the segment declares it")
        .arg("journey", T::Object, "what is so of this going through the list, handed on the same way")
        .arg("doing", T::String, "the column an item is in while it is in hand, where items are taken out of `here` for this")
        .arg("given", T::Int, "how often the item has been given to whoever works it again")
        .arg("most_given", T::Int, "how often at most: one sent back as often is left for a person; no limit, if 0 or not said")
        .arg("answer", T::Bool, "with `begin`: what the item asked was answered, and it begins where it asked (`at`)")
        .arg("at", T::List, "where the item was in the list when it asked: the row and the order, as `at` was given")
        .arg("was", T::Object, "what this answered the time before in the same going through the list; one that ran something says what it came to with `gave`, and one that ran nothing is answered again as it was")
        .arg("gave", T::Object, "with `was`: what the segment that was run gave out")
        .arg("error", T::String, "with `was`: why it failed, where it did")
        .arg("last", T::Bool, "nothing more can be run in this going through the list")
        .arg("most_done", T::Int, "how many things at most are called in one going through the list: the asking that would call one more leaves the item for a person; no limit, if 0 or not said")
        .arg("most_of", T::String, "the record that says that, for the sentence that says how to raise it")
        .arg("through", T::Bool, "all that is asked here is answered back (`asked`), for the askings after this one in the same going through the list, which then say only what came of what was run")
        .output("came", "`run`, `on`, `back`, `asks`, `left` or `nothing`")
        .output("word", "the same, but `review` for one that is begun and heard from by whoever asks")
        .output("calls", "for `run`: it is called and waited for")
        .output("spawns", "for `run`: it is begun by whoever asks, as that one usually begins it")
        .output("tells", "whoever asked for one to be begun has something to say on: it is called, or there is none to begin")
        .output("passes", "the word to say on where there is none to begin")
        .output("name", "for `run`: the blueprint")
        .output("row", "for `run`: the row that names it; empty for one of the usual")
        .output("order", "for `run`: its order")
        .output("title", "for `run`: what it is called")
        .output("tries", "for `run`: how often it has been given again")
        .output("with", "for `run`: what it is handed beside the item")
        .output("moves", "how many times the item has been moved, for whatever comes next to be held to")
        .output("says", "what is said with the word")
        .output("asker", "for `asks`: who asked")
        .output("question", "for `asks`: what")
        .output("why", "why, where the segment said: for whoever keeps that with the item")
        .output("at", "where the item is in the list: the row and the order of the one that is run, or that said the word")
        .output("journey", "what is so of this going through the list from here, with what the segment that ended handed on")
        .output("back", "for `back`: it goes back for what a segment said of it, and not because what was done could not be kept")
        .output("usual", "for one that is begun by whoever asks: only what is usual comes after it")
        .output("asked", "with `through`: all that was asked, for the next asking to be handed as part of `was`")
        .output("nth", "which thing of this going through the list this asking calls, or would have, from 1")
        .idempotent(),
        move |cx, args| async move {
            // Answered before in this going through the list, and nothing
            // was run: it is answered again as it was.
            let was = mapping(&args, "was");
            let ran = !was.is_empty();
            // Looked at again as the going through ends (`last`, with where
            // it now is): an item that is gone, has left the column or was
            // moved since this began is no longer this going through's.
            if ran && flag(&args, "last") && args.contains_key("status") {
                let count = |moved: Option<&Value>| {
                    moved.and_then(Value::as_array).and_then(|moved| moved.get(3)).map_or(0, |count| {
                        count.as_i64().or_else(|| count.as_str().and_then(|text| text.parse().ok())).unwrap_or(0)
                    })
                };
                let began = was.get("moves").and_then(Value::as_i64).unwrap_or(0);
                let there = flag(&args, "found") && text(&args, "status") == text(&args, "here");
                if !there || (began != 0 && count(args.get("moved")) != began) {
                    return Ok(json!({
                        "came": "nothing", "word": "nothing", "calls": false, "spawns": false, "tells": false,
                        "passes": "", "says": "", "with": {}, "row": "", "order": 0, "title": "", "at": ["", 0],
                        "journey": was.get("journey"),
                    }));
                }
            }
            if ran && !was.get("calls").and_then(Value::as_bool).unwrap_or(false) {
                return Ok(Value::Object(was));
            }
            // What the first asking of this going through the list was
            // handed is what this one is, but for what it says itself.
            let mut args = args;
            if let Some(Value::Object(first)) = was.get("asked") {
                for (name, said) in first {
                    if !args.contains_key(name) {
                        args.insert(name.clone(), said.clone());
                    }
                }
            }
            let through = match flag(&args, "through") {
                true => {
                    let mut all: Map<String, Value> =
                        args.iter().map(|(name, said)| (name.clone(), said.clone())).collect();
                    for said_here in ["was", "gave", "error", "last"] {
                        all.remove(said_here);
                    }
                    Value::Object(all)
                }
                false => Value::Null,
            };
            let out = mapping(&args, "gave");
            let ended = gave(&out);
            let before = |key: &str| was.get(key).cloned().unwrap_or(Value::Null);
            let mut journey = match was.get("journey").and_then(Value::as_object) {
                Some(journey) if ran => journey.clone(),
                _ => mapping(&args, "journey"),
            };
            for (name, value) in out {
                let says = !matches!(&value, Value::Null) && value.as_str() != Some("");
                if says && !WORDS_GIVEN.contains(&name.as_str()) {
                    journey.insert(name, value);
                }
            }
            let begins_at = args.get("at").and_then(Value::as_array).filter(|_| flag(&args, "answer"));
            let of_row = |at: usize| begins_at.and_then(|at_| at_.get(at)).cloned().unwrap_or(Value::Null);
            let moved = args.get("moved").and_then(Value::as_array).and_then(|moved| moved.get(3)).map_or(0, |count| {
                count.as_i64().or_else(|| count.as_str().and_then(|text| text.parse().ok())).unwrap_or(0)
            });
            let labels = args.get("labels").and_then(Value::as_array);
            let asked = Asked {
                rows: rows(&args),
                usual: usual(&args),
                item: text(&args, "item"),
                labels: labels.map(|all| all.iter().map(blueprint::expr::display).collect()).unwrap_or_default(),
                part_of: text(&args, "part_of"),
                found: flag(&args, "found"),
                status: text(&args, "status"),
                here: text(&args, "here"),
                moved,
                moves: number(&args, "moves"),
                doing: text(&args, "doing"),
                given: number(&args, "given"),
                most_given: number(&args, "most_given"),
                begin: flag(&args, "begin"),
                row: text(&args, "row"),
                order: number(&args, "order"),
                came: text(&args, "came"),
                failed: flag(&args, "failed"),
                why: text(&args, "why"),
                says: text(&args, "says"),
                asker: text(&args, "asker"),
                question: text(&args, "question"),
                asking: flag(&args, "asking"),
                tries: number(&args, "tries"),
                most: number(&args, "most"),
                // The first asking of a going through calls its first
                // thing; each after it is handed the one before (`was`).
                nth: if ran { was.get("nth").and_then(Value::as_i64).unwrap_or(0) + 1 } else { 1 },
                most_done: number(&args, "most_done"),
                most_of: text(&args, "most_of"),
            };
            // One that was run in this going through the list has ended:
            // where the item is, and what that came to, are what was
            // answered then and what the segment gave out.
            let asked = match ran {
                true => Asked {
                    begin: false,
                    row: before("row").as_str().unwrap_or_default().to_string(),
                    order: before("order").as_i64().unwrap_or(0),
                    tries: before("tries").as_i64().unwrap_or(0),
                    came: ended.came.clone(),
                    says: ended.says.clone(),
                    asker: ended.asker.clone(),
                    question: ended.question.clone(),
                    why: if flag(&args, "failed") { text(&args, "error") } else { ended.why.clone() },
                    ..asked
                },
                false if begins_at.is_some() => Asked {
                    row: of_row(0).as_str().unwrap_or_default().to_string(),
                    order: of_row(1).as_i64().or_else(|| of_row(1).as_str().and_then(|n| n.parse().ok())).unwrap_or(0),
                    ..asked
                },
                false => asked,
            };
            let mut came = next(&asked).map_err(ActionError::new)?;
            if flag(&args, "last")
                && let Came::Run(run) = &came
                && !run.spawned
            {
                let says = format!(
                    "{}: there is more to do with it in {} than is gone through at once, and {} was not begun: it is \
                     left for a person. Take a row of `segments` away, or put two of them in one blueprint",
                    asked.item, asked.here, run.title
                );
                came = Came::Left { says };
            }
            let (mut with, mut usual) = (Map::new(), true);
            if let Came::Run(run) = &came
                && !run.spawned
            {
                // Read as whoever asks may run it: a project's own file as
                // it was vouched for, and never one nobody vouched for.
                let source = cx.engine().text_within(cx.instance(), cx.principal(), &run.name, cx.project()).await;
                let fit = match source {
                    None => Err(format!(
                        "there is no blueprint called `{}` that may be run here: one of the project's own is \
                         run as it was saved through the factory or applied by whoever applied the fleet",
                        run.name
                    )),
                    // What the item holds is so of this going through too.
                    Some(source) => declared(&source).map_err(|why| format!("`{}` {why}", run.name)).and_then(
                        |declared| {
                            let coming = coming(&journey, &asked.labels, &asked.here);
                            fitted(run, &mapping(&args, "own"), &coming, &declared)
                        },
                    ),
                };
                match fit {
                    Ok(fit) => with = fit,
                    Err(why) => came = Came::Left { says: format!("{}: {} was not run: {why}", asked.item, run.title) },
                }
            }
            // One that is begun and heard from is begun by whoever asks,
            // with who looks and how many have to approve as its row says,
            // and as whoever asks says of itself where the row says nothing.
            if let Came::Run(run) = &came
                && run.spawned
            {
                match council(run, &mapping(&args, "own")) {
                    Ok(settings) => {
                        usual = rest_is_usual(&asked, run);
                        with = settings;
                    }
                    Err(why) => came = Came::Left { says: format!("{}: {why}", asked.item) },
                }
            }
            let run = match &came {
                Came::Run(run) => run.clone(),
                _ => Run::default(),
            };
            let (says, asker, question) = match &came {
                Came::On { says } | Came::Back { says } | Came::Left { says } => {
                    (says.clone(), String::new(), String::new())
                }
                Came::Asks { asker, question } => (String::new(), asker.clone(), question.clone()),
                _ => (String::new(), String::new(), String::new()),
            };
            let runs = matches!(came, Came::Run(_));
            let calls = runs && !run.spawned;
            let ends = !runs && !matches!(came, Came::Nothing);
            // Where the item is: at the one that is run, or at the one
            // that said the word.
            let at = match runs {
                true => json!([run.row, run.order]),
                false => json!([asked.row, asked.order]),
            };
            Ok(json!({
                "came": came.word(), "calls": calls, "spawns": runs && run.spawned, "tells": calls || ends,
                "word": if runs && run.spawned { "review" } else { came.word() },
                "passes": if ends { came.word() } else { "" },
                "name": run.name, "row": run.row, "order": run.order, "title": run.title, "tries": run.tries,
                "with": with, "moves": asked.moved, "says": says, "asker": asker, "question": question,
                "why": if asked.why.is_empty() { says.clone() } else { asked.why.clone() }, "at": at,
                "back": matches!(came, Came::Back { .. }) && journey.get("unkept") != Some(&Value::Bool(true)),
                "journey": journey, "asked": through, "usual": usual, "nth": asked.nth,
            }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "segments.ran",
            "Say what a command that was run for an item comes to: the item goes on, goes back with what was \
             printed, or is left for a person.",
        )
        .deciding()
        .required("item", T::String, "the item")
        .arg("title", T::String, "what the thing done is called")
        .arg("command", T::String, "the command, as it was said")
        .required("ok", T::Bool, "whether it went well")
        .arg("fails", T::String, "what becomes of the item where it did not: `back` (if not said) or `left`")
        .arg("printed", T::String, "what it printed")
        .arg("also", T::String, "and what it printed where errors go")
        .arg("why", T::String, "why it could not be run, where it could not")
        .arg("unrun", T::String, "why it was not run at all, where it was not: it is left for a person")
        .output("came", "`on`, `back` or `left`")
        .output("says", "what is said with the word")
        .idempotent(),
        move |_, args| async move {
            let asked = Ran {
                item: text(&args, "item"),
                title: text(&args, "title"),
                run: text(&args, "command"),
                ok: flag(&args, "ok"),
                fails: text(&args, "fails"),
                printed: [text(&args, "printed"), text(&args, "also")]
                    .into_iter()
                    .filter(|said| !said.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n"),
                why: text(&args, "why"),
                unrun: text(&args, "unrun"),
            };
            let (came, says) = ran(&asked).map_err(ActionError::new)?;
            Ok(json!({ "came": came, "says": says }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "segments.wrong",
            "Say what is wrong with the rows of a list of things done with the items of a column, each in a \
             sentence: what a row says that will never be done as it is written.",
        )
        .deciding()
        .arg("rows", T::List, "the rows of the table, as `rows.read` gives them")
        .arg("usual", T::List, "what is usually done, as `segments.next` is handed it")
        .arg("of", T::Object, "which rows are this list's, as `segments.next` is handed it")
        .required("here", T::String, "the column whose items these are")
        .arg("lists", T::Bool, "whether whoever asks goes through such a list at all")
        .output("said", "what is wrong, a sentence each; none where every row can be done")
        .idempotent(),
        move |_, args| async move {
            let said = wrong(&rows(&args), &usual(&args), &text(&args, "here"), flag(&args, "lists"));
            Ok(json!({ "said": said }))
        },
    );
    run_in::register(registry);
    recorded::register(registry);
}
