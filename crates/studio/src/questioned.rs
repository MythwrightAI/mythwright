//! A question asked on a record, said on it as something said there.
//!
//! What is asked of a person (an agent that cannot tell what is wanted,
//! a contest whose judging leaves it to somebody) is said on the record
//! it is about, among what else was said there (`record.ask`): by whoever
//! asked, where that is an agent (its line of the record's history is the
//! agent's, with the blueprint that said it after it), and otherwise by
//! the factory's bot, and naming with `@` each it is put to. Being named
//! is what tells them ([`talk::commented`]): a line in their box, where
//! they may read the record, which opens it at the question. Whom is
//! said as a telling says it (`applier`, an actor, a handle), and found
//! as a telling finds them ([`talk::Channels::whom`]); somebody with no
//! box is not named, and said to be passed over.
//!
//! It is the same done twice: what is said keeps which saying it is (the
//! step's own words, `once`, or else the step that said it), and a step
//! done again after a stop looks for that before it says anything.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, Registry};
use serde_json::{Value, json};

use crate::Studio;
use crate::actions::{add, text, through};
use crate::board::Through;
use crate::records::Saying;

/// The word each actor named is named by after `@`, where there is one:
/// a person by what their account is called, an agent by its name.
async fn handle_of(studio: &Studio, actor: &str) -> Option<String> {
    let (kind, name) = actor.split_once(':')?;
    let word = match kind {
        "agent" => name.to_string(),
        "user" => people::handle::handle(&studio.identity.holder(actor).await.ok()??.name),
        _ => return None,
    };
    (!word.is_empty()).then_some(word)
}

/// Whose words a question is, as an actor: an agent, where whoever asked
/// is one (by its actor, or by its name where an agent of that name is
/// there); nobody, so that the factory's bot says it, otherwise.
async fn asker_of(studio: &Studio, asker: &str) -> Option<String> {
    let asker = asker.trim();
    let actor = match asker.split_once(':') {
        Some(("agent", name)) if !name.is_empty() => asker.to_string(),
        Some(_) => return None,
        None if asker.is_empty() => return None,
        None => format!("agent:{asker}"),
    };
    studio.identity.holder(&actor).await.ok().flatten().map(|_| actor)
}

/// Which saying a step's is: its own words, or the step that said it.
fn saying_of(cx: &ActionCx, once: Option<String>) -> String {
    match once {
        Some(words) => format!("once {words}"),
        None => format!("{} {} {}", cx.run(), cx.step(), cx.doing()),
    }
}

pub(crate) fn register(registry: &mut Registry, studio: &Studio) {
    add(
        registry,
        studio,
        ActionSpec::new(
            "record.ask",
            "Ask a question on a record: said on it by whoever asked, naming with @ each it is put to, so that \
             being named tells them in their inbox. Needs `board:write` for a card, `records:write` for any other.",
        )
        .example(kernel::asked::RECORD_ASK)
        .required("id", T::String, "the record it is asked on")
        .required("says", T::String, "the question, in the words it was asked in")
        .arg(
            "to",
            T::List,
            "whom it is put to, each named with @ before the question: `applier`, whoever applied this; an actor \
             (`user:<id>`, `agent:<name>`); or what somebody with an account, or an agent, is called after `@`. \
             One with no inbox is not named. Nobody, where it holds none",
        )
        .arg(
            "asker",
            T::String,
            "who asked: an agent by its name or its actor says it, with this blueprint after it; anybody else, or \
             nothing, and the factory's bot says it",
        )
        .arg(
            "otherwise",
            T::String,
            "what is said after the question where it names nobody, so that a question asked of nobody says so",
        )
        .arg(
            "once",
            T::String,
            "which asking it is, in words of the step's own: a run begun again with the same words, or this step \
             done again, says nothing a second time",
        )
        .output("named", "each actor named, who is told where they may read the record")
        .output("passed", "each the words named who has no inbox, and was not named")
        .output("nobody", "whether it named nobody")
        .idempotent(),
        |studio, cx, args| async move {
            let id = text(&args, "id").unwrap_or_default();
            let says = text(&args, "says").unwrap_or_default();
            if says.trim().is_empty() {
                return Err(ActionError::new("a question says something: `says` is empty"));
            }
            let to: Vec<String> = match args.get("to") {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Array(words)) => words.iter().map(blueprint::expr::display).collect(),
                Some(Value::String(one)) if one.trim().is_empty() => Vec::new(),
                Some(Value::String(one)) => vec![one.clone()],
                Some(other) => {
                    return Err(ActionError::new(format!("`to` is a list of whom it is put to; it was given {other}")));
                }
            };
            let came = studio.channels.whom(cx.principal(), &to).await?;
            let mut named = Vec::new();
            let mut words = Vec::new();
            let mut passed = came.passed;
            for actor in came.told {
                match handle_of(&studio, &actor).await {
                    Some(word) => {
                        if !words.contains(&word) {
                            words.push(word);
                        }
                        named.push(actor);
                    }
                    None => passed.push(actor),
                }
            }
            let nobody = named.is_empty();
            let mut body = words.iter().map(|word| format!("@{word} ")).collect::<String>() + says.trim();
            if let Some(otherwise) = text(&args, "otherwise").filter(|_| nobody) {
                body = format!("{body}\n\n{}", otherwise.trim());
            }
            let own = text(&args, "once");
            let looks = own.is_some() || cx.again();
            let once = saying_of(&cx, own);
            let asker = text(&args, "asker").unwrap_or_default();
            let decided = asker_of(&studio, &asker).await;
            let through = Through { decided, ..through(&cx) };
            if !(looks && studio.records.said_once(&id, &once).await?) {
                let saying =
                    Saying { body: &body, public: false, anchor: None, once: Some(&once), asker: Some(&asker), parent: None, answered: false };
                studio.records.say_on(&id, saying, cx.principal(), Some(&through)).await?;
            }
            Ok(json!({ "named": named, "passed": passed, "nobody": nobody }))
        },
    );
}
