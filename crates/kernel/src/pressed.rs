//! What a column asks of somebody before a thing there is taken up, and
//! what was pressed.
//!
//! A table's rows may each be something a person, or whoever may, does
//! to a thing in one column: a button, called what its row says, which
//! is pressed and kept on the thing (a card is signed, approved, cleared
//! to go). A row that `holds` keeps the thing where it is until it has
//! been pressed: what looks after the column asks first whether
//! anything it holds is still to be pressed, and begins nothing while
//! something is. A row that holds nothing is still a button whose
//! pressing is kept and said.
//!
//! A press is for good, or for one coming of the thing to the column
//! (`again`): sent back and come again, the thing is to be pressed
//! again. Which coming a press is of is the count of the thing's moves
//! when it was made, which tells one coming from the next as it does
//! wherever a coming is told apart.
//!
//! Everything here is a function of what it is handed: the rows, as
//! their table holds them, and what the thing keeps of what was pressed
//! (a list of words, as a record keeps any list). Who reads and writes
//! them, and asks who may, is whoever keeps the thing; what holds a
//! thing back is asked by whatever looks after its column. Nothing here
//! knows what a column, a row or a thing is of.

use serde_json::{Map, Value};

/// Who may press: anybody who may change the thing, or only a person.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Who {
    #[default]
    Anyone,
    Person,
}

impl Who {
    pub fn word(&self) -> &'static str {
        match self {
            Who::Anyone => "anyone",
            Who::Person => "person",
        }
    }
}

/// The words `who` may be, in the order they are offered.
pub const WHO: &[&str] = &["anyone", "person"];

/// One row: something that can be pressed on a thing in a column.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Action {
    /// The row's id.
    pub id: String,
    /// What it is known by on a command line and in a text: what its row
    /// says (`name`), or what it is called, written as a name is.
    pub name: String,
    /// What it is called where it is pressed.
    pub title: String,
    /// The column a thing is to be in to be offered it.
    pub column: String,
    /// Who may press it, and a scope they are to hold beside, where its
    /// row says one.
    pub who: Who,
    pub scope: String,
    /// Whether a thing in the column is kept there until it is pressed.
    pub holds: bool,
    /// Whether a press is for this coming of the thing only.
    pub again: bool,
    /// What pressing it says in the thing's history, where its row says.
    pub says: String,
}

/// A name, as one is written: lowercase letters and digits, with a dash
/// where anything else was. `Signed off` is `signed-off`.
pub fn named(text: &str) -> String {
    let mut out = String::new();
    for letter in text.chars() {
        if letter.is_ascii_alphanumeric() {
            out.push(letter.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

fn cell<'a>(row: &'a Map<String, Value>, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default()
}

fn flag(row: &Map<String, Value>, key: &str) -> bool {
    match row.get(key) {
        Some(Value::Bool(flag)) => *flag,
        Some(Value::String(text)) => matches!(text.trim(), "true" | "yes"),
        _ => false,
    }
}

impl Action {
    /// A row, as a table's rows are read (its cells beside its `id`) or
    /// as a record is (`data` beside its `id`). One that is called
    /// nothing is none: there is nothing to press it by.
    pub fn read(row: &Value) -> Option<Action> {
        let whole = row.as_object()?;
        let cells = whole.get("data").and_then(Value::as_object).unwrap_or(whole);
        let title = cell(cells, "title").to_string();
        let name = match cell(cells, "name") {
            "" => named(&title),
            said => named(said),
        };
        if name.is_empty() {
            return None;
        }
        let who = match cell(cells, "who") {
            "person" => Who::Person,
            _ => Who::Anyone,
        };
        Some(Action {
            id: cell(whole, "id").to_string(),
            title: if title.is_empty() { name.clone() } else { title },
            name,
            column: cell(cells, "column").to_string(),
            who,
            scope: cell(cells, "scope").to_string(),
            holds: flag(cells, "holds"),
            again: flag(cells, "again"),
            says: cell(cells, "says").to_string(),
        })
    }

    /// Who may press it, in words: "a person", "anyone who may change it",
    /// with the scope they hold beside where it says one.
    pub fn who_may(&self) -> String {
        match (self.who, self.scope.is_empty()) {
            (Who::Person, true) => "a person".into(),
            (Who::Person, false) => format!("a person who holds `{}`", self.scope),
            (Who::Anyone, true) => "anyone who may change it".into(),
            (Who::Anyone, false) => format!("whoever holds `{}`", self.scope),
        }
    }
}

/// The rows that are something to press, as they were read.
pub fn rows(rows: &[Value]) -> Vec<Action> {
    rows.iter().filter_map(Action::read).collect()
}

/// The row `word` names: by its name first, then its id, then what it is
/// called, whatever its letters are.
pub fn find<'a>(actions: &'a [Action], word: &str) -> Option<&'a Action> {
    let word = word.trim();
    actions
        .iter()
        .find(|action| action.name == named(word) && !word.is_empty())
        .or_else(|| actions.iter().find(|action| !action.id.is_empty() && action.id == word))
        .or_else(|| actions.iter().find(|action| action.title.eq_ignore_ascii_case(word)))
}

/// One press: which row, of which coming, when and by whom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Press {
    pub action: String,
    pub coming: u64,
    /// When, in milliseconds.
    pub at: i64,
    /// Who, as an actor, and what they are called.
    pub by: String,
    pub name: String,
}

/// What a thing keeps of what was pressed on it: the latest press of
/// each row, and the coming it was last said to be held at, so that it
/// is said once for each coming however often it is looked at.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pressed {
    pub presses: Vec<Press>,
    pub held: Option<u64>,
}

impl Pressed {
    /// What a thing's list says. A word of it that is not one of these
    /// is passed over: a list somebody wrote by hand is read as far as
    /// it can be.
    pub fn read(kept: &[String]) -> Pressed {
        let mut pressed = Pressed::default();
        for line in kept {
            let words: Vec<&str> = line.splitn(6, ' ').collect();
            match words.as_slice() {
                ["pressed", action, coming, at, by, name @ ..] => {
                    if let (Ok(coming), Ok(at)) = (coming.parse(), at.parse()) {
                        pressed.presses.retain(|press| press.action != *action);
                        let (action, by) = (action.to_string(), by.to_string());
                        let name = name.first().map_or(String::new(), |name| name.to_string());
                        pressed.presses.push(Press { action, coming, at, by, name });
                    }
                }
                ["held", coming] => pressed.held = coming.parse().ok().or(pressed.held),
                _ => {}
            }
        }
        pressed
    }

    /// As a thing keeps it.
    pub fn kept(&self) -> Vec<String> {
        let mut kept: Vec<String> = self
            .presses
            .iter()
            .map(|press| {
                let said = format!("pressed {} {} {} {}", press.action, press.coming, press.at, press.by);
                if press.name.is_empty() { said } else { format!("{said} {}", press.name) }
            })
            .collect();
        if let Some(coming) = self.held {
            kept.push(format!("held {coming}"));
        }
        kept
    }

    /// The press of `action` that stands at `coming`: its latest, where
    /// a press is for good, and only one of this coming where it is for
    /// one coming at a time.
    pub fn standing(&self, action: &Action, coming: u64) -> Option<&Press> {
        self.presses
            .iter()
            .find(|press| press.action == action.name && (!action.again || press.coming == coming))
    }

    /// `action` pressed at `coming` by `by`, called `name`; nothing where
    /// it stands pressed already, which is the same pressed twice.
    pub fn press(&self, action: &Action, coming: u64, (by, name): (&str, &str), at: i64) -> Option<Pressed> {
        if self.standing(action, coming).is_some() {
            return None;
        }
        let mut pressed = self.clone();
        pressed.presses.retain(|press| press.action != action.name);
        let (by, name) = (by.to_string(), name.trim().to_string());
        pressed.presses.push(Press { action: action.name.clone(), coming, at, by, name });
        Some(pressed)
    }

    /// The press of `action` taken back: only one made at this coming,
    /// before the thing has gone anywhere since.
    pub fn take_back(&self, action: &Action, coming: u64) -> Result<Pressed, String> {
        match self.presses.iter().find(|press| press.action == action.name) {
            Some(press) if press.coming == coming => {
                let mut pressed = self.clone();
                pressed.presses.retain(|press| press.action != action.name);
                Ok(pressed)
            }
            Some(_) => Err(format!(
                "{} was pressed when it was last in {}, and is taken back only where it is still where it was \
                 pressed",
                action.title, action.column
            )),
            None => Err(format!("{} is not pressed, so there is nothing to take back", action.title)),
        }
    }

    /// Said to be held at `coming`; nothing where it was said already.
    pub fn hold(&self, coming: u64) -> Option<Pressed> {
        (self.held != Some(coming)).then(|| Pressed { held: Some(coming), ..self.clone() })
    }
}

/// The rows of `column` that keep a thing there and stand unpressed at
/// `coming`, in the order of the rows: what it waits for.
pub fn waits<'a>(actions: &'a [Action], column: &str, pressed: &Pressed, coming: u64) -> Vec<&'a Action> {
    actions
        .iter()
        .filter(|action| {
            action.holds && action.column == column && pressed.standing(action, coming).is_none()
        })
        .collect()
}

/// Whether somebody of `kind` (what their actor says before its colon:
/// `user`, `key`, `agent`, ...) who does or does not hold the row's
/// scope may press it, with why not in a sentence for them.
pub fn may(action: &Action, kind: &str, holds_scope: bool) -> Result<(), String> {
    if action.who == Who::Person && kind == "agent" {
        return Err(format!("{} is pressed by a person, and an agent may not press it", action.title));
    }
    if !action.scope.is_empty() && !holds_scope {
        return Err(format!("{} is pressed by {}, which you do not hold", action.title, action.who_may()));
    }
    Ok(())
}

/// What pressing says in the thing's history: the row's own words, or
/// that it was pressed; and that it was taken back.
pub fn did(action: &Action, taken_back: bool) -> String {
    match (taken_back, action.says.is_empty()) {
        (true, _) => format!("took back {}", action.title),
        (false, true) => format!("pressed {}", action.title),
        (false, false) => action.says.clone(),
    }
}

/// The sentence that says what a held thing waits for.
pub fn held_says(item: &str, column: &str, waits: &[&Action]) -> String {
    let each: Vec<String> =
        waits.iter().map(|action| format!("{}, which {} presses", action.title, action.who_may())).collect();
    let all = match each.as_slice() {
        [] => return format!("{item} waits for nothing in {column}."),
        [one] => one.clone(),
        [first @ .., last] => format!("{}; and {last}", first.join("; ")),
    };
    let those = if waits.len() == 1 { "that is" } else { "each is" };
    format!("{item} waits in {column} for {all}. It is taken up when {those} pressed.")
}

/// Why a press is refused: a sentence, and what kind of refusal it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// What was named is nothing the thing is offered.
    Unoffered(String),
    /// It is offered elsewhere, or is not pressed to be taken back.
    Unfit(String),
    /// Whoever presses may not.
    Denied(String),
}

/// A press asked of one thing: which, where it is and which time it came
/// there; the word that names the row and whether it is taken back; who
/// presses (the kind of actor, the actor and the name) and when.
pub struct Asked<'a> {
    pub item: &'a str,
    pub column: &'a str,
    pub coming: u64,
    pub word: &'a str,
    pub back: bool,
    pub by: (&'a str, &'a str, &'a str),
    pub at: i64,
}

/// What a press comes to.
#[derive(Clone, Debug, PartialEq)]
pub struct Pressing<'a> {
    pub action: &'a Action,
    /// What the thing keeps now; nothing where nothing is to be written
    /// (pressed already).
    pub now: Option<Pressed>,
    /// It was held, and this was the last of what held it.
    pub released: bool,
    /// What still holds it, by what each is called.
    pub waits: Vec<String>,
}

/// What pressing comes to for a thing that keeps `was`, among `actions`:
/// the row found by its word, held to the thing's column and to who may
/// (`holds`, whether whoever presses holds a scope).
pub fn pressing<'a>(
    actions: &'a [Action],
    was: &Pressed,
    asked: Asked<'_>,
    holds: impl Fn(&str) -> bool,
) -> Result<Pressing<'a>, Refused> {
    let Asked { item, column, coming, word, back, by: (kind, actor, name), at } = asked;
    let Some(action) = find(actions, word) else {
        let offered: Vec<&str> =
            actions.iter().filter(|one| one.column == column).map(|one| one.title.as_str()).collect();
        return Err(Refused::Unoffered(match offered.as_slice() {
            [] => format!(
                "nothing is pressed on {item} in {column}: no row of what can be pressed is of that column"
            ),
            offered => format!(
                "`{word}` is not something {item} is offered; in {column} it is offered {}",
                offered.join(", ")
            ),
        }));
    };
    if action.column != column {
        let there = if action.column.is_empty() { "no column" } else { action.column.as_str() };
        return Err(Refused::Unfit(format!(
            "{} is pressed on a card in {there}, and {item} is in {column}",
            action.title
        )));
    }
    may(action, kind, action.scope.is_empty() || holds(&action.scope)).map_err(Refused::Denied)?;
    let titles = |pressed: &Pressed| {
        waits(actions, column, pressed, coming).iter().map(|one| one.title.clone()).collect()
    };
    let before: Vec<String> = titles(was);
    let now = match back {
        true => Some(was.take_back(action, coming).map_err(Refused::Unfit)?),
        false => was.press(action, coming, (actor, name), at),
    };
    let waits: Vec<String> = now.as_ref().map_or(before.clone(), titles);
    let released = now.is_some() && !back && !before.is_empty() && waits.is_empty();
    Ok(Pressing { action, now, released, waits })
}

/// What a thing that keeps `pressed` is held by at `coming`, and what it
/// keeps once that is said; nothing where nothing holds it or it was
/// said already for this coming.
pub fn holding<'a>(
    actions: &'a [Action],
    column: &str,
    pressed: &Pressed,
    coming: u64,
) -> Option<(Pressed, Vec<&'a Action>)> {
    let left = waits(actions, column, pressed, coming);
    pressed.hold(coming).filter(|_| !left.is_empty()).map(|now| (now, left))
}
