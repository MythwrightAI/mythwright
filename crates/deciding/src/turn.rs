//! What an agent's turn came to, by the word it ended with.
//!
//! An agent given a piece of work ends its turn by choosing one of a few
//! words, and what each comes to is whoever gave it the work's to say
//! (`ends`): each word with the word a thing done with an item ends in
//! (`on`, `back`, `again`, `asks`, `left`; `docs/blueprints.md`, *Writing
//! a segment*). Where cards are worked that is `done=on` and `ask=asks`;
//! where they are planned `propose=on`, `ask=asks` and `leave=left`. So a
//! third word is a word more in the setting, which the agent is offered,
//! and what it comes to is decided here and nowhere in a text. An agent
//! that chooses none of them is taken to have chosen the first.
//!
//! What goes with the word is made here too: the words an item goes back
//! or is left with, and the question where it asks.
//!
//! A turn of several who look at once (a council: `seats` of
//! `agent.task`) ends with what they came to together, which the tally
//! says (`passed`, `failed` or `undecided`), and `ends` then says what
//! each of those comes to: `passed=on failed=back` has an item go on
//! where enough approve and go back, with what was said against it, where
//! they no longer can. What it does not say comes to what it usually
//! does (`passed=on`, `failed=back`, `undecided=left`), so that no
//! outcome is ever taken for another. Those who look are offered the two
//! words they are counted by (`yes`, `no`), and not those of `ends`.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

/// Every word a turn can come to.
pub const CAME_WORDS: &[&str] = &["on", "back", "again", "asks", "left"];

/// What a turn on a piece of work may end with where nobody says.
pub const USUAL: &[&str] = &["done=on", "ask=asks"];

/// What several who look at once may come to together: those of the
/// tally (`deciding::tally`), but for that it waits.
pub const OUTCOMES: &[&str] = &["passed", "failed", "undecided"];

/// What each of those comes to where nobody says.
pub const COUNCIL_USUAL: &[&str] = &["passed=on", "failed=back", "undecided=left"];

/// What an agent that chose to ask and said nothing has asked.
pub const ASKED_NOTHING: &str = "What is wanted here? (It chose to ask, and did not say what.)";

/// The words an agent may end its turn with, in the order it is offered
/// them, each with what it comes to; or, of several who look at once,
/// what they may come to together, each with what that comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ends(Vec<(String, &'static str)>, bool);

impl Ends {
    /// Read from what is said: each `word=what it comes to`, written one
    /// to an entry of the list or several to one, between spaces or commas
    /// (a row of a table says a list as one line).
    pub fn read(said: &[String]) -> Result<Ends, String> {
        let mut ends: Vec<(String, &'static str)> = Vec::new();
        let each = said.iter().flat_map(|entry| entry.split(|c: char| c == ',' || c.is_whitespace()));
        for one in each.filter(|one| !one.is_empty()) {
            let Some((word, comes)) = one.split_once('=').filter(|(word, _)| !word.is_empty()) else {
                return Err(format!(
                    "`{one}` in `ends` says no word and what it comes to: each is `word=what it comes to`, as \
                     `done=on`"
                ));
            };
            let Some(comes) = CAME_WORDS.iter().copied().find(|known| *known == comes) else {
                return Err(format!(
                    "`{one}` in `ends` comes to `{comes}`, which is none of the words a turn comes to: {}",
                    CAME_WORDS.iter().map(|word| format!("`{word}`")).collect::<Vec<_>>().join(", ")
                ));
            };
            if ends.iter().any(|(known, _)| known == word) {
                return Err(format!("`ends` says what `{word}` comes to twice"));
            }
            ends.push((word.to_string(), comes));
        }
        if ends.is_empty() {
            return Err("`ends` says no word the turn may end with, as `done=on`".into());
        }
        Ok(Ends(ends, false))
    }

    /// What several who look at once come to, read from what is said of
    /// it (each `outcome=what it comes to`), with what it usually comes
    /// to for each outcome that is not said; the usual, where nothing is.
    pub fn council(said: &[String]) -> Result<Ends, String> {
        let said_any = said.iter().any(|entry| !entry.trim().is_empty());
        let mut ends = match said_any {
            true => Ends::read(said)?.0,
            false => Vec::new(),
        };
        if let Some((word, _)) = ends.iter().find(|(word, _)| !OUTCOMES.contains(&word.as_str())) {
            return Err(format!(
                "`ends` says what `{word}` comes to, and several who look at once (`seats`) come to one of \
                 `passed`, `failed` or `undecided` together: say what each of those comes to, as \
                 `passed=on failed=back`, and the two words they are counted by with `yes` and `no`"
            ));
        }
        for usual in Ends::read(&COUNCIL_USUAL.iter().map(|one| one.to_string()).collect::<Vec<_>>())?.0 {
            if !ends.iter().any(|(word, _)| *word == usual.0) {
                ends.push(usual);
            }
        }
        Ok(Ends(ends, true))
    }

    /// Whether these are what several who look at once come to.
    pub fn of_a_council(&self) -> bool {
        self.1
    }

    /// What a turn on a piece of work ends with where nobody says.
    pub fn usual() -> Ends {
        Ends::read(&USUAL.iter().map(|one| one.to_string()).collect::<Vec<_>>())
            .expect("the usual words read")
    }

    /// The words, in order: what the agent is offered.
    pub fn words(&self) -> Vec<String> {
        self.0.iter().map(|(word, _)| word.clone()).collect()
    }
}

/// How a turn ended: the item, what its agent chose, who it is, and what
/// it said.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    pub item: String,
    pub decision: String,
    pub by: String,
    pub reply: String,
}

/// What it came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Came {
    /// One of [`CAME_WORDS`].
    pub came: &'static str,
    /// The word it was taken to have chosen.
    pub chose: String,
    /// For `back`, the words the item goes back with; for `left`, the
    /// sentence for where the work is reported.
    pub says: String,
    /// For `again` and `left`: why, in words kept with the item.
    pub why: String,
    /// For `asks`: who, and what.
    pub asker: String,
    pub question: String,
}

/// What a turn that ended with this answer comes to, by `ends`.
///
/// `on` goes with nothing: what the agent said is what it said where it
/// reports. The others go with what it said, or where it said nothing,
/// with which word it chose.
pub fn came(answer: &Answer, ends: &Ends) -> Result<Came, String> {
    if answer.item.is_empty() {
        return Err("the `item` the turn was on is not said".into());
    }
    let (chose, comes) =
        ends.0.iter().find(|(word, _)| *word == answer.decision).unwrap_or(&ends.0[0]).clone();
    // Several who look at once came to what they came to together.
    let (who, chose_it) = match (ends.1, answer.by.is_empty()) {
        (true, _) => ("those who looked", "came to"),
        (false, true) => ("its agent", "chose"),
        (false, false) => (answer.by.as_str(), "chose"),
    };
    let reply = answer.reply.trim();
    let or_chose = || match reply.is_empty() {
        true => format!("{who} {chose_it} `{chose}` and said nothing more"),
        false => reply.to_string(),
    };
    let mut came = Came { came: comes, chose: chose.clone(), ..Came::default() };
    match comes {
        "back" => came.says = or_chose(),
        "again" => came.why = or_chose(),
        "asks" => {
            came.asker = answer.by.clone();
            came.question = if reply.is_empty() { ASKED_NOTHING.to_string() } else { answer.reply.clone() };
        }
        "left" => {
            came.says = match reply.is_empty() {
                true => format!("{}: {who} {chose_it} `{chose}`, and it is left for a person", answer.item),
                false => {
                    format!(
                        "{}: {who} {chose_it} `{chose}`, and it is left for a person: {reply}",
                        answer.item
                    )
                }
            };
            came.why = or_chose();
        }
        _ => {}
    }
    Ok(came)
}

/// What giving an item to an agent came to, as whoever did that asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Worked {
    pub item: String,
    /// Whether there was anybody to give it to.
    pub available: bool,
    /// Who it was to be given to by name, where that one is not up.
    pub named: String,
    /// Whether whoever asks named that one itself; otherwise the item did.
    pub told: bool,
    /// Whether that one is an agent the factory does not start, which
    /// takes up what is given to it itself: nobody was asked.
    pub external: bool,
    /// What the agent chose, who it is, and what it said.
    pub decision: String,
    pub by: String,
    pub reply: String,
    /// The words it may end with, each with what it comes to
    /// (`turn::Ends`); `done=on` and `ask=asks` where none are said.
    pub ends: Vec<String>,
    /// Where the item stays where there was nobody to give it to, in
    /// words that follow `it stays`; `in doing` where none are said.
    pub stays: String,
    /// How many looked at it at once, each for itself; none, where one
    /// agent was given it. Where several did, what they came to together
    /// (`decided`, which `ends` says what each comes to: [`Ends::council`])
    /// and what was said (`words`) are its answer.
    pub seats: i64,
    pub decided: String,
    pub words: String,
    /// What its agent's program said it was refused as it went (a tool, a
    /// command), each by a word, where its package says; and what a turn
    /// that ended `on` for all that comes to: `note`, it goes on as its
    /// agent said and what was refused is only said, which is what is
    /// done where nothing is said; `blocked`, it is left for a person as
    /// one whose agent said it could not do what it needed.
    pub refused: Vec<String>,
    pub refused_means: String,
    /// How the work is finished, where whoever gave it says: the steps
    /// its agent sees to before it says it is done (`crate::finished`). A
    /// turn that came to `on` is held to what its agent said of each.
    pub finish: Vec<String>,
    /// How often it has been given again for being nearly done.
    pub given: i64,
    /// Its time ran out, its program was stopped, and what it said is
    /// what it said when asked then what it had done.
    pub cut: bool,
}

/// What a turn whose program was refused something, and whose agent
/// said it went on, may come to (`Worked::refused_means`).
pub const REFUSED_MEANS: &[&str] = &["note", "blocked"];

/// The word that came to, and what goes with it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkedCame {
    /// `left`, there was nobody to give it to; otherwise what the word
    /// its agent ended with comes to (`turn::came`): with the usual
    /// words, `on`, it was worked, and `asks`, it asked in place of
    /// working.
    pub came: &'static str,
    /// For `left`: the sentence for where the work is reported, and why
    /// in words the item's row keeps; for `back`, the words it goes back
    /// with; for `again`, why.
    pub says: String,
    pub why: String,
    pub asker: String,
    pub question: String,
    /// For `left`, which way it came to be left, for whoever is told:
    /// `nobody`, there was nobody to give it to; `external`, it is an
    /// external agent's to take up; or the word its agent ended with that
    /// comes to `left` (`blocked`, it could not do what it needed); or
    /// `unfinished` and `cut` (`crate::finished`).
    pub left_as: String,
    /// For `again`: it is given again because it was nearly done.
    pub nearly: bool,
}

/// Every word [`worked`] can answer: those of a turn.
pub const WORKED_WORDS: &[&str] = CAME_WORDS;

/// What giving an item to an agent came to: what the word its agent
/// ended with comes to (with the usual words, it was worked and goes on,
/// or its agent asked something in place of working), or there was
/// nobody to give it to and it is left where it is, with the sentence
/// that says who was not there.
pub fn worked(asked: &Worked) -> Result<WorkedCame, String> {
    if asked.item.is_empty() {
        return Err("the `item` that was given to an agent is not said".into());
    }
    let means = if asked.refused_means.is_empty() { "note" } else { asked.refused_means.as_str() };
    if !REFUSED_MEANS.contains(&means) {
        return Err(format!(
            "`refused_means` is what a turn whose program was refused something comes to where its agent went \
             on, `note` or `blocked`; it was given `{means}`"
        ));
    }
    let ends = match (asked.seats > 0, asked.ends.is_empty()) {
        (true, _) => Ends::council(&asked.ends)?,
        (false, true) => Ends::usual(),
        (false, false) => Ends::read(&asked.ends)?,
    };
    let stays = if asked.stays.is_empty() { "in doing" } else { asked.stays.as_str() };
    let left = |who: String| WorkedCame {
        came: "left",
        says: format!("{}: {who}; it stays {stays}", asked.item),
        why: "there was nobody to give it to".into(),
        left_as: "nobody".into(),
        ..WorkedCame::default()
    };
    Ok(match (asked.available, asked.named.as_str(), asked.told) {
        // Named, and one the factory does not start: the item is that
        // one's to take up, and is left in hand for it.
        (_, name, _) if !name.is_empty() && asked.external => WorkedCame {
            came: "left",
            says: format!(
                "{}: it is given to {name}, an external agent, which takes up its own cards and is asked nothing \
                 here; it stays {stays}",
                asked.item
            ),
            why: format!("it is given to {name}, an external agent, which takes it up itself"),
            left_as: "external".into(),
            ..WorkedCame::default()
        },
        (_, name, true) if !name.is_empty() => {
            left(format!("{name}, whom its row names, is not an agent that is up"))
        }
        (_, name, false) if !name.is_empty() => {
            left(format!("{name}, whom the card is given to, is not an agent that is up"))
        }
        (false, _, _) => left("no agent is available to take this card".into()),
        (true, _, _) => {
            let answer = match asked.seats > 0 {
                true => Answer {
                    item: asked.item.clone(),
                    decision: asked.decided.clone(),
                    by: String::new(),
                    reply: asked.words.clone(),
                },
                false => Answer {
                    item: asked.item.clone(),
                    decision: asked.decision.clone(),
                    by: asked.by.clone(),
                    reply: asked.reply.clone(),
                },
            };
            let came = came(&answer, &ends)?;
            if came.came == "on" && means == "blocked" && !asked.refused.is_empty() && asked.seats == 0 {
                let refused = asked.refused.join(", ");
                let reply = asked.reply.trim();
                let said = if reply.is_empty() { String::new() } else { format!(": {reply}") };
                return Ok(WorkedCame {
                    came: "left",
                    says: format!(
                        "{}: its program was refused {refused}, and it is left for a person{said}",
                        asked.item
                    ),
                    why: format!("its program was refused {refused}{said}"),
                    left_as: "blocked".into(),
                    ..WorkedCame::default()
                });
            }
            // How it is finished, where that was said, or what a turn
            // whose time ran out said when asked.
            let checked = match (asked.seats > 0, came.came, asked.cut && asked.decision.is_empty()) {
                (true, ..) => None,
                (false, _, true) if asked.finish.is_empty() => {
                    Some(crate::finished::cut(&asked.item, &asked.by, &asked.reply, asked.given))
                }
                (false, "on", _) => crate::finished::checked(
                    &asked.item,
                    &asked.by,
                    &asked.finish,
                    &asked.reply,
                    asked.given,
                    asked.cut,
                ),
                _ => None,
            };
            if let Some(checked) = checked {
                return Ok(WorkedCame {
                    came: checked.came,
                    says: checked.says,
                    why: checked.why,
                    left_as: checked.left_as,
                    nearly: checked.nearly,
                    ..WorkedCame::default()
                });
            }
            let left_as = if came.came == "left" { came.chose.clone() } else { String::new() };
            WorkedCame {
                came: came.came,
                says: came.says,
                why: came.why,
                asker: came.asker,
                question: came.question,
                left_as,
                nearly: false,
            }
        }
    })
}

/// A list that was handed, each of its entries as a word; none, where
/// nothing was.
pub(crate) fn listed(args: &kernel::Fields, key: &str) -> Vec<String> {
    let entry = |entry: &Value| entry.as_str().map(str::to_string).unwrap_or_else(|| entry.to_string());
    match args.get(key) {
        Some(Value::Array(list)) => list.iter().map(entry).collect(),
        Some(Value::String(text)) => vec![text.clone()],
        _ => Vec::new(),
    }
}

/// What is said of `ends`, or the usual where nothing is: of a turn, or
/// where several look at once (`seats`), of what they come to together.
pub(crate) fn ends_of(args: &kernel::Fields) -> Result<Ends, String> {
    let said = listed(args, "ends");
    match (seats(args) > 0, said.is_empty()) {
        (true, _) => Ends::council(&said),
        (false, true) => Ok(Ends::usual()),
        (false, false) => Ends::read(&said),
    }
}

/// How many look at once, each for itself: none, where it is not said.
pub(crate) fn seats(args: &kernel::Fields) -> i64 {
    match args.get("seats") {
        Some(Value::Number(number)) => {
            number.as_i64().or_else(|| number.as_f64().map(|n| n as i64)).unwrap_or(0)
        }
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let said = |args: &kernel::Fields, key: &str| {
        args.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
    };
    add(
        registry,
        ActionSpec::new(
            "turn.came",
            "Say what an agent's turn came to by the word it ended with, and what goes with it; and the words it \
             may end with, to offer it.",
        )
        .deciding()
        .required("item", T::String, "the item the turn is on")
        .arg("ends", T::List, "the words the agent may end with, each as `word=what it comes to` (`on`, `back`, `again`, `asks` or `left`); one it does not choose is taken for the first; `done=on` and `ask=asks`, if not said. With `seats`: what those who look come to together (`passed`, `failed`, `undecided`), each as `outcome=what it comes to`; one not said comes to `passed=on`, `failed=back` or `undecided=left`")
        .arg("seats", T::Int, "how many look at once, each for itself (`seats` of `agent.task`); none, if 0 or not said")
        .arg("yes", T::String, "with `seats`: the word those who look choose for the work (default `approve`)")
        .arg("no", T::String, "with `seats`: the word they choose against it (default `send_back`)")
        .arg("decision", T::String, "what the agent chose; with none, what one that chose none of them comes to. With `seats`: what those who look came to (`came` of `agent.task`)")
        .arg("by", T::String, "the agent")
        .arg("reply", T::String, "what it said; with `seats`, what they said (`words` of `agent.task`)")
        .arg("finish", T::List, "how the work is finished: each step its agent sees to before it says it is done, such as `its tests pass`; none, if not said")
        .arg("later", T::List, "with `finish`: what somebody else checks after the agent, and so is never its to wait on, such as `the whole gate, run when the work lands`")
        .arg("text", T::String, "with `finish`: the item's own words, for what of `later` they name")
        .arg("asking", T::String, "which asking of the item this is, in a word of whoever asks (`asking` of `agent.task`)")
        .arg("given", T::Int, "how often it has been given again for being nearly done")
        .output("options", "the words the agent may end with, in order, to offer it; with `seats`, the two each who looks is counted by")
        .output("checklist", "with `finish`: what the agent is told before what it is given, with how it ends its turn")
        .output("asking", "the word this turn is asked by: `asking`, and after it which time it is given again where it is")
        .output("cut", "the word it is asked by, where its time ran out, what it has done")
        .output("after", "what it is asked where its time ran out: to do no more, and say what is done and what is left")
        .output("came", "`on`, `back`, `again`, `asks` or `left`")
        .output("chose", "the word it was taken to have chosen")
        .output("says", "for `back`: the words it goes back with; for `left`: the sentence for where the work is reported")
        .output("why", "for `again` and `left`: why, in words kept with the item")
        .output("asker", "for `asks`: who asked")
        .output("question", "for `asks`: what")
        .idempotent(),
        move |_, args| async move {
            let ends = ends_of(&args).map_err(ActionError::new)?;
            let answer = Answer {
                item: said(&args, "item"),
                decision: said(&args, "decision"),
                by: said(&args, "by"),
                reply: said(&args, "reply"),
            };
            let came = came(&answer, &ends).map_err(ActionError::new)?;
            let options = match ends.of_a_council() {
                true => vec![
                    Some(said(&args, "yes")).filter(|word| !word.is_empty()).unwrap_or_else(|| crate::tally::YES.into()),
                    Some(said(&args, "no")).filter(|word| !word.is_empty()).unwrap_or_else(|| crate::tally::NO.into()),
                ],
                false => ends.words(),
            };
            // Those who look at once are counted by two words, and are
            // not told how the work is finished: that is whoever did it's.
            let finish = if ends.of_a_council() { Vec::new() } else { listed(&args, "finish") };
            let checklist = crate::finished::told(&answer.item, &finish, &listed(&args, "later"), &said(&args, "text"));
            let (asking, given) = (said(&args, "asking"), args.get("given").and_then(Value::as_i64).unwrap_or(0));
            let asking = match (asking.is_empty(), given > 0) {
                (false, true) => format!("{asking}:again{given}"),
                _ => asking,
            };
            let cut = if asking.is_empty() { String::new() } else { format!("{asking}:cut") };
            Ok(json!({
                "options": options, "came": came.came, "chose": came.chose, "says": came.says,
                "why": came.why, "asker": came.asker, "question": came.question,
                "after": crate::finished::after(&answer.item, &checklist),
                "checklist": checklist, "asking": asking, "cut": cut,
            }))
        },
    );
}
