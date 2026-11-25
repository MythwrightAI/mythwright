//! What giving a field a value takes, as scopes, every one of which is
//! asked of whoever gives it (`takes` of a field, and of one of its
//! values).
//!
//! One scope was all a field could say, and a value that is two things
//! at once was guarded as one of them: `with` of a fleet's `segments` is
//! a param of a blueprint run as whoever applied the fleet, which may be
//! a command (`shell`) and may be the name of a secret the factory keeps
//! (`secrets:use`). So a field says several in a list
//! (`takes: [shell, secrets:use]`), and a value the same way
//! (`takes: { on: [shell, secrets:use] }`). One scope is written, kept
//! and handed out as a word, as it always was, so that a table declared
//! before reads as it did; several as a list.

use kernel::{Error, Principal, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// The scopes giving something takes, each once and in the order written.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Takes(Vec<String>);

impl Takes {
    /// The scopes, in the order they were written.
    pub fn scopes(&self) -> &[String] {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether it takes `scope`, written as it is written here.
    pub fn has(&self, scope: &str) -> bool {
        self.0.iter().any(|one| one == scope)
    }

    /// Those of its scopes `by` does not hold.
    pub fn lacking<'a>(&'a self, by: &Principal) -> Vec<&'a str> {
        self.0.iter().map(String::as_str).filter(|scope| !by.can(scope)).collect()
    }

    /// Reads it as a declaration writes it: a scope, or a list of them,
    /// of any value or of the one `value` names. Nothing, an empty word
    /// and an empty list say nothing. Anything else (`true`, a number) is
    /// refused: passed over, it was a guard somebody meant that was not
    /// there.
    pub(crate) fn read(name: &str, value: Option<&str>, said: Option<&Value>) -> Result<Takes> {
        let neither = |one: &Value, among: bool| {
            let one = blueprint::expr::display(one);
            let (giving, like) = match value {
                None => (
                    "giving it a value".to_string(),
                    "`takes: shell` or `takes: [shell, secrets:use]`, or for a choice or a flag those of each \
                     value, as in `takes: { on: shell }`"
                        .to_string(),
                ),
                Some(value) => (
                    format!("giving it `{value}`"),
                    format!("`takes: {{ {value}: shell }}` or `takes: {{ {value}: [shell, secrets:use] }}`"),
                ),
            };
            let said = if among {
                format!("`{one}` among what {giving} takes")
            } else {
                format!("that {giving} takes `{one}`")
            };
            Error::invalid(format!(
                "the field `{name}` says {said}, which is no scope: what {giving} takes is a scope or a list of \
                 them, as in {like}"
            ))
        };
        let words: Vec<String> = match said {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::String(one)) => vec![one.trim().to_string()],
            Some(Value::Array(several)) => several
                .iter()
                .filter(|one| !one.is_null())
                .map(|one| one.as_str().map(|one| one.trim().to_string()).ok_or_else(|| neither(one, true)))
                .collect::<Result<_>>()?,
            Some(other) => return Err(neither(other, false)),
        };
        let mut takes = Vec::new();
        for scope in words.into_iter().filter(|scope| !scope.is_empty()) {
            if scope == crate::fields::EACH {
                return Err(Error::invalid(format!(
                    "the field `{name}` says `{scope}` among the scopes giving it a value takes; `takes: {scope}` \
                     is said alone, of a list of scopes, where each one written takes holding it"
                )));
            }
            crate::declared::held_by_somebody(&scope)
                .map_err(|err| Error::invalid(format!("the field `{name}` takes {}", err.message)))?;
            if !takes.contains(&scope) {
                takes.push(scope);
            }
        }
        Ok(Takes(takes))
    }

    /// The rest of a sentence that refuses `by`, where they lack any of
    /// its scopes: what it takes, and of that what they do not hold.
    /// Nothing where they hold all of it.
    pub(crate) fn refused(&self, by: &Principal) -> Option<String> {
        let lacking = self.lacking(by);
        if lacking.is_empty() {
            return None;
        }
        let all = listed(self.0.iter().map(String::as_str));
        Some(if lacking.len() == self.0.len() {
            format!("takes {all}, which {} does not hold", by.name)
        } else {
            format!("takes {all}, and {} does not hold {}", by.name, listed(lacking))
        })
    }
}

/// Scopes as a sentence says them: "`a`", "`a` and `b`", "`a`, `b` and `c`".
pub(crate) fn listed<'a>(scopes: impl IntoIterator<Item = &'a str>) -> String {
    let quoted: Vec<String> = scopes.into_iter().map(|scope| format!("`{scope}`")).collect();
    match quoted.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, before)) => format!("{} and {last}", before.join(", ")),
    }
}

impl Serialize for Takes {
    fn serialize<S: Serializer>(&self, to: S) -> std::result::Result<S::Ok, S::Error> {
        match self.0.as_slice() {
            [one] => to.serialize_str(one),
            several => several.serialize(to),
        }
    }
}

impl<'de> Deserialize<'de> for Takes {
    fn deserialize<D: Deserializer<'de>>(from: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Kept {
            One(String),
            Several(Vec<String>),
        }
        Ok(Takes(match Option::<Kept>::deserialize(from)? {
            None => Vec::new(),
            Some(Kept::One(one)) => vec![one],
            Some(Kept::Several(several)) => several,
        }))
    }
}
