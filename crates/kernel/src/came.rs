//! How the package that came with this installation differs from the copy
//! of its name that is installed, as a function of the two manifests.
//!
//! It is by what each says and not by how it is written: both are read as
//! the language reads them, and what is compared is what was read. All it
//! says is that they differ, and where. Which of the two is the earlier is
//! not something a manifest says: a package has no version, and nothing is
//! kept of what was installed but the copy itself, so a copy its owner
//! changed and a copy of what an earlier program shipped read the same
//! here.

use serde_json::{Value, json};

/// How many names a clause says before it says how many more there are.
const NAMED: usize = 6;

/// Where the one that came differs from the installed copy, each a place
/// in the manifest written as its keys with dots between.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Differs {
    /// What the one that came says and the copy does not.
    pub besides: Vec<String>,
    /// What both say, each another way.
    pub another: Vec<String>,
    /// What the copy says and the one that came does not.
    pub gone: Vec<String>,
}

/// Where `came` differs from `installed`, each a manifest as it was read.
/// Mappings are gone into; anything else (a list, a blueprint's text) is
/// one thing, the same or not.
pub fn differs(installed: &Value, came: &Value) -> Differs {
    let mut found = Differs::default();
    walk(installed, came, "", &mut found);
    found
}

fn walk(installed: &Value, came: &Value, at: &str, found: &mut Differs) {
    let (Value::Object(was), Value::Object(now)) = (installed, came) else {
        if installed != came {
            found.another.push(at.to_string());
        }
        return;
    };
    let under = |key: &str| if at.is_empty() { key.to_string() } else { format!("{at}.{key}") };
    for (key, value) in now {
        match was.get(key) {
            Some(before) => walk(before, value, &under(key), found),
            None => found.besides.push(under(key)),
        }
    }
    found.gone.extend(was.keys().filter(|key| !now.contains_key(*key)).map(|key| under(key)));
}

impl Differs {
    /// Whether the two say the same.
    pub fn is_empty(&self) -> bool {
        self.besides.is_empty() && self.another.is_empty() && self.gone.is_empty()
    }

    /// What is said of it after "the one that came" or "it", in a
    /// sentence's worth of clauses: `says x besides; says something else
    /// of y; does not say z`. Nothing for two that say the same.
    pub fn in_words(&self) -> String {
        let clauses = [
            (&self.besides, "says ", " besides"),
            (&self.another, "says something else of ", ""),
            (&self.gone, "does not say ", ""),
        ];
        let said: Vec<String> = clauses
            .into_iter()
            .filter(|(places, ..)| !places.is_empty())
            .map(|(places, before, after)| format!("{before}{}{after}", named(places)))
            .collect();
        said.join("; ")
    }

    /// As it is handed out: the three lists and the words.
    pub fn to_json(&self) -> Value {
        json!({
            "besides": self.besides, "another": self.another, "gone": self.gone, "words": self.in_words(),
        })
    }
}

/// The places by name, the first few and then how many more.
fn named(places: &[String]) -> String {
    let mut names: Vec<String> = places.iter().take(NAMED).map(|place| format!("`{place}`")).collect();
    if places.len() > NAMED {
        names.push(format!("{} more", places.len() - NAMED));
    }
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} and {last}", rest.join(", ")),
        _ => names.concat(),
    }
}
