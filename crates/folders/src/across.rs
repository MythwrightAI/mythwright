//! What every page of a factory says across its top where it is not the
//! factory somebody would take it for: a factory started from a
//! copy of another says so, with its profile where that is not the usual
//! one, `Profile: preview. A copy of prod taken 2026-11-23 14:05 UTC: what
//! is done here stays here.`; and one told what to say there as it was
//! started (`--banner`) says that instead, copy or not. A factory that is
//! neither says nothing there. It is said until whoever manages the
//! factory says it may stop, and said again where it is told something new
//! to say.
//!
//! What is kept is the factory's own, in the namespace of what it keeps of
//! itself, which no copy takes: a copy of a factory that was told to stop
//! saying it says it all the same, and one of a factory that was told what
//! to say says what it is.

use journal::Journal;
use kernel::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::arrival::{Arrived, REMEMBERED};

/// Where a factory keeps what it says across every page and whether it is to stop.
pub const KEPT: (&str, &str) = ("factory", "across");
/// The most letters it is told to say.
pub const LONGEST: usize = 200;

/// What a factory keeps of it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kept {
    /// Whoever manages it said it may stop.
    #[serde(default)]
    pub quiet: bool,
    /// What it was told to say in place of what it would.
    #[serde(default)]
    pub says: Option<String>,
}

/// What a page says across its top, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Across {
    /// `said`, what it was told to say; `copy`, it is a copy of another.
    pub kind: &'static str,
    /// What it was told to say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub says: Option<String>,
    /// Of a copy, the factory it is a copy of by its profile, and the
    /// device it was on where it had none; and when the copy was taken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub made: Option<i64>,
    /// This factory's profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The line in words, with the time in UTC, as a terminal says it.
    pub words: String,
}

/// What a factory says across every page: what it was told to, else that it
/// is a copy, with its profile; nothing where it was told it may stop, or has
/// neither to say.
pub fn across(kept: &Kept, arrived: Option<&Arrived>, profile: Option<&str>) -> Option<Across> {
    if kept.quiet {
        return None;
    }
    let profile = profile.filter(|name| !name.is_empty()).map(str::to_string);
    let blank = |kind| Across {
        kind,
        says: None,
        of: None,
        device: None,
        made: None,
        profile: profile.clone(),
        words: String::new(),
    };
    if let Some(says) = kept.says.as_ref().filter(|says| !says.trim().is_empty()) {
        return Some(Across { says: Some(says.clone()), words: says.clone(), ..blank("said") });
    }
    if let Some(arrived) = arrived {
        let (of, device) = match &arrived.from.profile {
            Some(name) => (Some(name.clone()), None),
            None => (None, Some(arrived.from.device.clone())),
        };
        let called = of.clone().unwrap_or_else(|| format!("the factory on {}", arrived.from.device));
        let named = profile.as_ref().map(|name| format!("Profile: {name}. ")).unwrap_or_default();
        let words = format!(
            "{named}A copy of {called} taken {}: what is done here stays here.",
            crate::arrival::when(arrived.made)
        );
        return Some(Across { of, device, made: Some(arrived.made), words, ..blank("copy") });
    }
    None
}

/// What `written` comes to as what a factory is told to say, or why not:
/// nothing where it is empty.
pub fn told(written: &str) -> Result<Option<String>> {
    let says = written.trim();
    let letters = says.chars().count();
    if says.chars().any(char::is_control) || letters > LONGEST {
        return Err(kernel::Error::invalid(format!(
            "what is said across every page is one line of {LONGEST} letters at most, and this is {letters} letters{}",
            if says.chars().any(char::is_control) { " over more than one line" } else { "" }
        )));
    }
    Ok((!says.is_empty()).then(|| says.to_string()))
}

/// What the factory on `journal` keeps of it.
pub async fn kept(journal: &Journal) -> Kept {
    let kept = journal.kv_get(KEPT.0, KEPT.1).await.ok().flatten();
    kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default()
}

/// What every page of the factory on `journal`, called `profile`, says across its top.
pub async fn read(journal: &Journal, profile: Option<&str>) -> Option<Across> {
    let arrived = journal.kv_get(REMEMBERED.0, REMEMBERED.1).await.ok().flatten();
    let arrived: Option<Arrived> = arrived.and_then(|arrived| serde_json::from_value(arrived).ok());
    across(&kept(journal).await, arrived.as_ref(), profile)
}

/// Keeps what it is now to say: told something new to say, it says it again
/// though it was told it may stop.
pub async fn keep(journal: &Journal, kept: &Kept) -> Result<()> {
    journal.kv_set(KEPT.0, KEPT.1, &json!(kept)).await
}
