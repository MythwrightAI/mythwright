//! A run that waits for a person to say it may go on (`approval.ask`), as plain values: what the step
//! asked, how long it waits, who is told, who may decide it, and the
//! words it is said in. It is an ask of its own kind
//! ([`crate::approvals::Kind::Go`]) beside a change of a record's cells,
//! kept in the same table and decided at the same door; what is decided
//! of it is decided here, with a table of cases in the kernel's suite.
//!
//! Who may approve is one rule ([`may`]): a person, never an agent;
//! holding what the run's next stage takes (`blueprints:run`); and, where
//! the environment the next stage is to be under says who approves what
//! goes under it (`approve`), one of those. Whoever started the run may
//! approve it only where that list names them: so somebody running their
//! own deploy is not asked to approve themselves, and with no list nobody waves
//! their own run through.

use serde_json::{Value, json};

use crate::approvals::{LASTS_MS, Status};

/// What a step asked to be let go on with, as an ask keeps it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Asking {
    /// What is to go on, in the blueprint's words.
    pub says: String,
    /// The blueprint whose run waits, and its instance.
    pub blueprint: String,
    pub instance: String,
    /// The environment whose rule says who approves; nothing, where there is none.
    pub environment: String,
    /// Addresses to look at before deciding, each a page on the web.
    pub links: Vec<String>,
    /// Until when it waits, in milliseconds.
    pub until: i64,
}

impl Asking {
    /// As an ask keeps it (its `cells`).
    pub fn kept(&self) -> Value {
        json!({
            "says": self.says, "blueprint": self.blueprint, "instance": self.instance,
            "environment": self.environment, "links": self.links, "until": self.until,
        })
    }

    /// Read back; what is not there is nothing.
    pub fn read(kept: &Value) -> Asking {
        let text = |key: &str| kept[key].as_str().unwrap_or_default().to_string();
        Asking {
            says: text("says"),
            blueprint: text("blueprint"),
            instance: text("instance"),
            environment: text("environment"),
            links: kept["links"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
            until: kept["until"].as_i64().unwrap_or_default(),
        }
    }
}

/// Until when an ask made at `at` waits: as long as its step is given
/// (`timeout`), and seven days where it says nothing, as a change asked
/// for waits.
pub fn until(at: i64, limit_ms: Option<u64>) -> i64 {
    at.saturating_add(limit_ms.map_or(LASTS_MS, |limit| limit.min(i64::MAX as u64) as i64))
}

/// How an ask kept as `status`, waiting until `until`, stands at `now`:
/// one that has waited its time is expired, and nobody approves it after.
pub fn stands(status: Status, until: i64, now: i64) -> Status {
    match status {
        Status::Waiting if now >= until => Status::Expired,
        other => other,
    }
}

/// Somebody deciding whether a run may go on.
#[derive(Clone, Copy, Debug)]
pub struct Deciding<'a> {
    /// Who, as an actor (`user:<id>`; a key is its maker's account).
    pub by: &'a str,
    /// Whether they are a person: an agent, a device or the system is not.
    pub person: bool,
    /// Whether they hold what the run's next stage takes (`blueprints:run`).
    pub holds: bool,
    /// Who started the run, as an actor.
    pub starter: &'a str,
    /// The environment's rule, where the ask names one that says who
    /// approves: those people, as actors. Nothing, or an empty list,
    /// where it says nobody.
    pub rule: Option<&'a [String]>,
    /// The environment's name, for saying so.
    pub environment: &'a str,
}

/// What approving or declining takes beside being one of those [`may`] lets.
pub const TAKES: &str = "blueprints:run";

/// Whether `deciding` may approve or decline: why not, in a sentence.
pub fn may(deciding: &Deciding<'_>) -> Result<(), String> {
    if !deciding.person {
        return Err("a run is let go on by a person, never by an agent".into());
    }
    let rule = deciding.rule.filter(|rule| !rule.is_empty());
    match rule {
        Some(rule) if !rule.iter().any(|one| one == deciding.by) => {
            return Err(format!(
                "what goes under `{}` is approved by those its rule names, and you are not one of them",
                deciding.environment
            ));
        }
        None if deciding.by == deciding.starter => {
            return Err(
                "you started this run, and somebody else lets it go on: only an environment whose rule names you lets you approve your own"
                    .into(),
            );
        }
        _ => {}
    }
    if !deciding.holds {
        return Err(format!("letting a run go on takes `{TAKES}`, which you do not hold"));
    }
    Ok(())
}

/// Whether a run begun for a row that runs as whoever applied what keeps
/// it (a row of a project's triggers: `std/pipeline`) waits for a
/// go-ahead from one of those the environment it is under names as
/// approving (`rule`) before it starts there: unless everybody it is begun
/// for is one of them (`for_whom`: whoever applied what runs it, whoever
/// last wrote the row's environment, and whoever asked for it by hand),
/// and never where the rule names nobody. So a row written by somebody the
/// rule does not name never runs there on the standing of whoever applied
/// the triggers alone. Somebody not known (nothing: a row written before
/// who wrote it was kept) is nobody the rule names.
pub fn first(rule: &[String], for_whom: &[&str]) -> bool {
    !rule.is_empty() && for_whom.iter().any(|one| !rule.iter().any(|named| named == one))
}

/// Whom an ask is told to: of `could` (those [`may`] lets, as actors),
/// those `to` names, and all of them where it names none of them.
pub fn told(could: &[String], to: &[String]) -> Vec<String> {
    let named: Vec<String> = could.iter().filter(|one| to.contains(one)).cloned().collect();
    if named.is_empty() { could.to_vec() } else { named }
}

/// The words of the line somebody who may decide is told by.
pub fn line(asker: &str, asking: &Asking) -> String {
    let under = match asking.environment.as_str() {
        "" => String::new(),
        environment => format!(" under {environment}"),
    };
    let says = asking.says.trim();
    let says = if says.is_empty() { String::new() } else { format!(": {says}") };
    format!("{asker}'s run of {} waits for your go-ahead{under}{says}", asking.blueprint)
}

/// What a run that waited is told came of it, in a sentence: `by` is
/// whoever decided, by name, and `why` what they said.
pub fn decided(status: Status, blueprint: &str, by: &str, why: &str) -> String {
    let because = if why.trim().is_empty() { String::new() } else { format!(": {}", why.trim()) };
    match status {
        Status::Waiting => format!("The run of {blueprint} waits for somebody to let it go on."),
        Status::Approved => format!("{by} let the run of {blueprint} go on."),
        Status::Declined => format!("{by} declined to let the run of {blueprint} go on{because}."),
        Status::Expired => {
            format!("Nobody let the run of {blueprint} go on in the time it waits, and it goes no further.")
        }
        Status::Withdrawn => {
            format!("The run of {blueprint} no longer waits: its ask was withdrawn{because}.")
        }
        Status::Stale => format!("The run of {blueprint} no longer waits for this ask."),
    }
}
