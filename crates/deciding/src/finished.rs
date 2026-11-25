//! How a piece of work is finished, as whoever gives it says, and what an
//! agent's account of that comes to.
//!
//! Whoever gives an agent a piece of work may say how it is finished: the
//! steps the agent sees to before it says the work is done (`finish`: its
//! tests pass, the docs it changes say what is new), and what somebody
//! else checks after it and so is never the agent's to wait on (`later`:
//! the whole gate, run when the work lands). The agent is told both as a
//! checklist before what it is given ([`told`]), with what the item's own
//! words name of the second said to be checked later, and ends by saying
//! of each step, a line each, whether it is done, waits only on something
//! checked later, or is not done and why ([`said`]). What that comes to
//! is decided here and nowhere in a text ([`checked`]): every step done,
//! or waiting only on something checked later, and the work goes on; some
//! not done, and it is given again once with what is left (nearly done),
//! and after that left for a person saying what is left; none done, and it
//! is left. So a card that says "the gate passes" to an agent told never
//! to run the whole gate is no longer the agent's to reconcile.
//!
//! A turn may be given a time as well (`within` of `agent.task`): past it
//! its program is stopped and the agent asked once, briefly, to say the
//! same of each step, which is read here too (`cut`). One that says
//! nothing even then is left, saying its time ran out.

/// What each step may be said to be, after `STEP <n>:`.
pub const STATES: &[&str] = &["done", "later", "not done"];

/// How a step was said to stand.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    Done,
    /// It waits only on something checked later.
    Later,
    /// Not done, and why, where it said.
    Not(String),
    /// Nothing was said of it.
    Unsaid,
}

/// What a turn on a piece of work whose finishing was said came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// `on`, `again` or `left`.
    pub came: &'static str,
    /// For `left`: the sentence for where the work is reported.
    pub says: String,
    /// For `again`, what is left, which it is told; for `left`, why.
    pub why: String,
    /// For `left`: `unfinished`, or `cut` where its time ran out and it
    /// said nothing of what it had done.
    pub left_as: String,
    /// It was given again because it was nearly done.
    pub nearly: bool,
}

/// The checklist an agent is told before what it is given, with how it
/// ends its turn; nothing where no step is said.
///
/// `card` is the item's own words (its title and what it says), for which
/// of what is checked later it names: such a thing is said to be checked
/// later, and not the agent's to wait on.
pub fn told(item: &str, finish: &[String], later: &[String], card: &str) -> String {
    if finish.is_empty() {
        return String::new();
    }
    let mut told = format!("Before you say {item} is done, see to each of these, in order:\n\n");
    for (at, step) in finish.iter().enumerate() {
        told.push_str(&format!("{}. {}\n", at + 1, step.trim()));
    }
    if !later.is_empty() {
        let later: Vec<&str> = later.iter().map(|one| one.trim()).collect();
        told.push_str(&format!(
            "\nChecked later by somebody else, once the work has left you, and never yours to wait on or to run: \
             {}.\n",
            later.join("; ")
        ));
        let named = named(later.as_slice(), card);
        if !named.is_empty() {
            told.push_str(&format!(
                "What {item} says asks for {}: that part is checked later, as above, and is not yours to wait on. \
                 Do the rest.\n",
                named.join(" and ")
            ));
        }
    }
    told.push_str(
        "\nWhen you end, say of each of them, a line each above your last line: `STEP <number>: done`; `STEP \
         <number>: later`, where it waits only on something checked later; or `STEP <number>: not done: <why>`. \
         A step you say nothing of is taken as not done.\n\n",
    );
    told
}

/// What of `later` the item's own words name: each by what it is called
/// (its words before a comma), where a word of that of four letters or
/// more is a word of the item's.
pub fn named<'a>(later: &[&'a str], card: &str) -> Vec<&'a str> {
    let words = |text: &str| -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| word.len() > 3)
            .map(str::to_lowercase)
            .collect()
    };
    let card = words(card);
    later
        .iter()
        .map(|one| one.split(',').next().unwrap_or_default().trim())
        .filter(|thing| words(thing).iter().any(|word| card.contains(word)))
        .collect()
}

/// What an agent said of each of `count` steps, read from what it said:
/// a line `STEP <n>: done`, `STEP <n>: later` or `STEP <n>: not done[:
/// why]` for each, whatever marks a list or a word's weight are about it.
/// The last said of a step stands; a step said nothing of is unsaid.
pub fn said(reply: &str, count: usize) -> Vec<Said> {
    let mut said = vec![Said::Unsaid; count];
    for line in reply.lines() {
        let line = line.trim().trim_start_matches(['-', '*', '>', ' ']).trim_matches(['*', '`', '_']).trim();
        let Some((label, rest)) = line.split_once(':') else { continue };
        let mut label = label.split_whitespace();
        if !label.next().is_some_and(|word| word.eq_ignore_ascii_case("step")) {
            continue;
        }
        let Some(at) = label.next().and_then(|n| n.trim_matches(['#', '.']).parse::<usize>().ok()) else {
            continue;
        };
        if label.next().is_some() || at == 0 || at > count {
            continue;
        }
        let rest = rest.trim().trim_matches(['*', '`', '_']).trim();
        let (state, why) = match rest.split_once(':') {
            Some((state, why)) => (state.trim(), why.trim()),
            None => (rest, ""),
        };
        let state = state.trim_end_matches(['.', ',', ';']).to_lowercase();
        said[at - 1] = match state.as_str() {
            "done" => Said::Done,
            "later" => Said::Later,
            "not done" => Said::Not(why.trim_end_matches('.').to_string()),
            _ => continue,
        };
    }
    said
}

/// What it comes to that `by` said a piece of work was done, where how it
/// is finished was said (`finish`), by what it said of each step (`reply`,
/// read by [`said`]); how often it has been given again for being nearly
/// done (`given`); and whether its time ran out and this is what it said
/// when asked (`cut`). `None` where no step was said: there is nothing to
/// check.
pub fn checked(
    item: &str,
    by: &str,
    finish: &[String],
    reply: &str,
    given: i64,
    cut: bool,
) -> Option<Checked> {
    if finish.is_empty() {
        return None;
    }
    let who = if by.is_empty() { "its agent" } else { by };
    let said = said(reply, finish.len());
    let as_left = |left_as: &str, says: String, why: String| Checked {
        came: "left",
        says: format!("{item}: {says}, and it is left for a person"),
        why,
        left_as: left_as.into(),
        nearly: false,
    };
    if cut && reply.trim().is_empty() {
        return Some(as_left(
            "cut",
            format!("the time {who} was given ran out, and asked then it said nothing of what it had done"),
            "its time ran out, and it said nothing of what it had done".into(),
        ));
    }
    let open: Vec<String> = finish
        .iter()
        .zip(&said)
        .enumerate()
        .filter_map(|(at, (step, said))| match said {
            Said::Done | Said::Later => None,
            Said::Not(why) if !why.is_empty() => {
                Some(format!("{}. {} (not done: {why})", at + 1, step.trim()))
            }
            Said::Not(_) => Some(format!("{}. {} (not done)", at + 1, step.trim())),
            Said::Unsaid => Some(format!("{}. {} (it said nothing of it)", at + 1, step.trim())),
        })
        .collect();
    if open.is_empty() {
        return Some(Checked { came: "on", ..Checked::default() });
    }
    let left = open.join("; ");
    let time = if cut { "its time ran out, and " } else { "" };
    if said.iter().all(|one| matches!(one, Said::Not(_))) {
        return Some(as_left(
            "unfinished",
            format!("{time}{who} said none of how it is finished is done: {left}"),
            format!("none of how it is finished is done: {left}"),
        ));
    }
    if given < 1 {
        return Some(Checked {
            came: "again",
            why: format!("{time}not all of how it is finished is done; what is left: {left}"),
            nearly: true,
            ..Checked::default()
        });
    }
    Some(as_left(
        "unfinished",
        format!("{time}{who} was given it again once for what was left, and this is left still: {left}"),
        format!("what is left of how it is finished: {left}"),
    ))
}

/// What a turn whose time ran out comes to where no step was said and
/// its agent, asked then, chose no word: what it said is what is left,
/// given again once and then left; and where it said nothing, left.
pub fn cut(item: &str, by: &str, reply: &str, given: i64) -> Checked {
    let who = if by.is_empty() { "its agent" } else { by };
    let reply = reply.trim();
    let left = |says: String, why: String, left_as: &str| Checked {
        came: "left",
        says: format!("{item}: {says}, and it is left for a person"),
        why,
        left_as: left_as.into(),
        nearly: false,
    };
    match (reply.is_empty(), given < 1) {
        (true, _) => left(
            format!("the time {who} was given ran out, and asked then it said nothing of what it had done"),
            "its time ran out, and it said nothing of what it had done".into(),
            "cut",
        ),
        (false, true) => Checked {
            came: "again",
            why: format!("its time ran out before it was done; what it said is left: {reply}"),
            nearly: true,
            ..Checked::default()
        },
        (false, false) => left(
            format!("the time {who} was given ran out again after it was given it again once: {reply}"),
            format!("its time ran out twice; what it said is left: {reply}"),
            "unfinished",
        ),
    }
}

/// What an agent whose time ran out is asked, once and briefly, in the
/// same conversation: to do no more, and to say what is done and what is
/// left, of each step where they were said (`checklist`, from [`told`]).
pub fn after(item: &str, checklist: &str) -> String {
    let said = match checklist.is_empty() {
        true => "Say briefly what of it you have done and what is left.".to_string(),
        false => format!("Say briefly what you have done and what is left, as you were told:\n\n{checklist}"),
    };
    format!(
        "The time you were given for {item} ran out, and your program was stopped. Do no more work on it now. {said}"
    )
    .trim_end()
    .to_string()
}
