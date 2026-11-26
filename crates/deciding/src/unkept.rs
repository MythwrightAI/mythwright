//! What becomes of work that could not be kept.
//!
//! What an agent changed is kept as a commit (`place.keep`), and a commit
//! may be refused: by something in the work's way, which whoever did the
//! work can put right; or because git has nobody to commit as, which no
//! agent can. Work that was not kept does not go on. It is given to its
//! agent again, told what git said, a number of times at most; after
//! that, or where it is not the agent's to put right, it is left for a
//! person. Which of those, and the sentence that says so where the work
//! is reported, is [`next`]: a function of what keeping answered and of
//! how often the work was given back already. A blueprint asks it as
//! `place.unkept`, and then does what the word says.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use serde_json::{Value, json};

use crate::add;

blueprint::came! {
    /// What becomes of work that was not kept.
    pub enum Came {
        /// It is given to whoever did it again, to put right what is in the way.
        Again => "again",
        /// It is left where it is for a person.
        Left => "left",
        /// Git has nobody to commit as, which is a person's to say: it is left.
        Unnamed => "unnamed",
    }
}

/// What becomes of the work, and what is said of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Next {
    pub came: Came,
    /// What comes next, as a sentence for where the work is reported.
    pub says: String,
}

/// What becomes of work that could not be kept. `unkept` is what git said
/// was in the way of the commit, and nothing where keeping failed some
/// other way; `named`, whether git knows who to commit as; `tries`, how
/// many times the work was given back for this already, and `most`, how
/// many times it may be; `by`, who did it.
pub fn next(unkept: &str, named: bool, tries: i64, most: i64, by: &str) -> Result<Next, String> {
    for (name, many) in [("tries", tries), ("most", most)] {
        if many < 0 {
            return Err(format!("`{name}` is a number of times, and is not less than none; it is {many}"));
        }
    }
    let came = match (named, !unkept.is_empty() && tries < most) {
        (true, true) => Came::Again,
        (true, false) => Came::Left,
        (false, _) => Came::Unnamed,
    };
    let says = match came {
        Came::Again => {
            let by = if by.is_empty() { "Whoever worked on it" } else { by };
            format!("{by} is given the card again, to put that right.")
        }
        Came::Left => match tries {
            0 => "It is left in doing for a person.".to_string(),
            1 => "It is left in doing for a person, having been given back 1 time for it.".to_string(),
            tries => {
                format!("It is left in doing for a person, having been given back {tries} times for it.")
            }
        },
        Came::Unnamed => format!(
            "Git does not know who to commit as, which no agent can put right: {}. The card is left in doing for \
             a person.",
            kernel::git::TELL_WHO
        ),
    };
    Ok(Next { came, says })
}

/// Adds what is asked here to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    add(
        registry,
        ActionSpec::new(
            "place.unkept",
            "Say what becomes of work that could not be kept: given to whoever did it again, or left for a person.",
        )
        .deciding()
        .arg(
            "unkept",
            T::String,
            "what git said was in the way of the commit (`unkept` of `place.keep`); empty where keeping failed \
             some other way",
        )
        .arg("named", T::Bool, "whether git knows who to commit as (`named` of `place.keep`; default true)")
        .arg("tries", T::Int, "how many times the work was given back for this already (default 0)")
        .arg("most", T::Int, "how many times it may be given back (default 0)")
        .arg("by", T::String, "who did the work")
        .output("came", &format!("{}: what becomes of it", Came::words()))
        .output("again", "whether it is given to whoever did it again")
        .output("ends", "the word that is for whatever does this as one of a list of things: `back`, it is given again; `left`")
        .output("says", "what comes next, as a sentence for where the work is reported")
        .idempotent(),
        |_, args| async move {
            let text = |key: &str| args.get(key).and_then(Value::as_str).unwrap_or_default();
            let number = |key: &str| match args.get(key) {
                None | Some(Value::Null) => Ok(0),
                Some(said) => said
                    .as_i64()
                    .ok_or_else(|| ActionError::new(format!("`{key}` is a whole number, and was given {said}"))),
            };
            let named = match args.get("named") {
                None | Some(Value::Null) => true,
                Some(Value::Bool(named)) => *named,
                Some(said) => {
                    return Err(ActionError::new(format!("`named` is true or false, and was given {said}")));
                }
            };
            let next = next(text("unkept"), named, number("tries")?, number("most")?, text("by"))
                .map_err(ActionError::new)?;
            let again = next.came == Came::Again;
            let ends = if again { "back" } else { "left" };
            Ok(json!({ "came": next.came.word(), "again": again, "ends": ends, "says": next.says }))
        },
    );
}
