//! How a running tree comes by new texts, and how one that is taken lands.
//!
//! Different places want different things of the same factory. Where work
//! is being done for somebody, nothing that runs should move until a
//! person says so; where somebody is writing blueprints, having to say so
//! for every change is in the way. So how texts are come by is a setting
//! of the factory, said by its owner, with two parts:
//!
//! - **How texts are come by** ([`Come`]). `held`: a tree goes on with
//!   what it reached when somebody applied it, until somebody applies
//!   saying to take the texts as they now are ([`crate::pinned`]), and a
//!   project's own file is run as it was vouched for
//!   ([`crate::vouched`]). `follow`: a text that changes is taken as it
//!   changes, by everything running that reaches it, with nobody asked.
//!   Where nothing was said, it is as the texts themselves are: what came
//!   with the program is held, and a file somebody writes where the
//!   factory runs (the blueprints of a checkout it is run from) is
//!   followed, so that a factory run from its checkout follows unless it
//!   is told otherwise. A project's own are followed only where somebody
//!   said `follow`, for the reason below.
//! - **How a text that is taken lands on what is running** ([`Land`]).
//!   `in_place`: by the one rule (`conductor`'s `update`), which carries an
//!   instance over or has it take the text where it is, and runs nothing
//!   again where it need not. `again`: its steps run again.
//!
//! **`follow` gives something up, and is said by somebody who may.** A
//! blueprint runs with the authority of whoever applied it, and a
//! project's own are files that whoever may write the project's files can
//! change, an agent among them. Held, such a file is run only as somebody
//! who holds what the instance runs with vouched for it. Followed, it is
//! taken as it changes: whoever turned `follow` on has vouched, once and
//! for the factory, for every file as it comes to be. So turning it on
//! takes what taking a wall away takes (`shell`), is said as an event,
//! and vouches with what whoever said it held then and no more
//! ([`Texts::vouch_saved`]): an instance that runs with more than that
//! goes on with its file as it was vouched for.
//!
//! A text that does not check is not taken. What runs it goes on with the
//! last that did, and says so once, beside itself and as an event
//! (`instance.text_left`), until the text checks or is what it runs.
//!
//! The setting is the factory's: kept in the factory's journal, whichever
//! conductor's keeper is asked, and held once in memory for all of them.
//! [`Texts::holds`] is the one place that says whether a text is held.
//! What is done to what is running when the setting changes, or when a
//! followed text does, is the runner's (`conductor`'s `taking`).

use std::sync::Mutex;

use kernel::{Error, Principal, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::Texts;

/// Where the setting is kept: it is the factory's, and goes into a copy.
pub const NAMESPACE: &str = runs::beside::TAKING;
const KEY: &str = "factory";

/// What having texts followed gives up, in the words whoever turns it on
/// is told.
pub const GIVES_UP: &str = "where texts are followed, whoever can write a project's blueprint files, an agent \
                            among them, changes what runs with the authority of whoever applied it";

/// How texts are come by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Come {
    Held,
    Follow,
}

/// How a text that is taken lands on what is running.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Land {
    #[default]
    InPlace,
    Again,
}

/// The setting as it is kept and shown.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taking {
    /// How texts are come by, where somebody said.
    #[serde(default)]
    pub come: Option<Come>,
    #[serde(default)]
    pub land: Land,
    /// Who last said, as an actor; what they held then; and when.
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub held: Vec<String>,
    #[serde(default)]
    pub at: Option<i64>,
}

/// What is asked for when the setting is changed: each part only where it
/// is said. `come` is `held`, `follow`, or `usual` for nothing said.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Take {
    #[serde(default)]
    pub come: Option<String>,
    #[serde(default)]
    pub land: Option<String>,
}

/// The setting in memory.
#[derive(Default)]
pub(crate) struct Kept {
    now: Mutex<Taking>,
}

pub(crate) async fn load(journal: &journal::Journal) -> Kept {
    let kept = journal.kv_get(NAMESPACE, KEY).await.ok().flatten();
    let now: Taking = kept.and_then(|kept| serde_json::from_value(kept).ok()).unwrap_or_default();
    Kept { now: Mutex::new(now) }
}

impl Texts {
    /// The setting as it stands.
    pub fn taking(&self) -> Taking {
        self.taking.now.lock().expect("taking lock").clone()
    }

    /// Whether texts are followed here because somebody said so: a
    /// project's own among them.
    pub fn follows(&self) -> bool {
        self.taking().come == Some(Come::Follow)
    }

    /// Whether anything is followed here: said so, or nothing said, where
    /// what is held to nothing (a checkout's blueprints) is followed.
    pub fn brings(&self) -> bool {
        self.taking().come != Some(Come::Held)
    }

    /// Whether a text that is taken has its instance's steps run again.
    pub fn lands_again(&self) -> bool {
        self.taking().land == Land::Again
    }

    /// Whether the text `load` gives for `name` is one a running tree goes
    /// on with as it reached it ([`crate::pinned`]). Never a project's own,
    /// which is held by who vouched for it.
    pub fn holds(&self, name: &str, project: Option<&str>) -> bool {
        let library = &self.library;
        match self.taking().come {
            Some(Come::Held) => !library.is_own(name, project),
            Some(Come::Follow) => false,
            None => library.ships(name, project),
        }
    }

    /// Changes the setting as `asked`, as `by`: the factory's owner's to
    /// do, and having texts followed takes `shell` as well. What it was,
    /// and what it is now. Bringing what is running to it, and saying so,
    /// is whoever runs's.
    pub async fn set_taking(&self, asked: Take, by: &Principal) -> Result<(Taking, Taking)> {
        by.require("admin")?;
        let was = self.taking();
        let come = match asked.come.as_deref() {
            None => was.come,
            Some("held") => Some(Come::Held),
            Some("follow") => Some(Come::Follow),
            Some("usual") => None,
            Some(other) => {
                return Err(Error::invalid(format!(
                    "`{other}` is not a way texts are come by; there are `held` (what is running goes on with \
                     what it has until somebody takes the texts as they now are), `follow` (a text that \
                     changes is taken as it changes) and `usual` (as the texts are: what came with the \
                     program is held)"
                )));
            }
        };
        let land = match asked.land.as_deref().map(|said| said.replace(['-', ' '], "_")).as_deref() {
            None => was.land,
            Some("in_place") => Land::InPlace,
            Some("again") => Land::Again,
            Some(other) => {
                return Err(Error::invalid(format!(
                    "`{other}` is not a way a taken text lands; there are `in_place` (what is running is \
                     carried over, or takes the text where it is) and `again` (its steps run again)"
                )));
            }
        };
        if come == Some(Come::Follow) && was.come != Some(Come::Follow) && !by.can("shell") {
            return Err(Error::denied(format!(
                "having texts followed takes `shell`, which you do not hold: {GIVES_UP}"
            )));
        }
        let now =
            Taking { come, land, by: Some(by.actor()), held: by.scopes.clone(), at: Some(self.own.now_ms()) };
        self.factory.kv_set(NAMESPACE, KEY, &json!(now)).await?;
        *self.taking.now.lock().expect("taking lock") = now.clone();
        Ok((was, now))
    }
}
