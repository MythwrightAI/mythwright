//! What is asked of the conductor when a blueprint is applied, and what it
//! answers.

use blueprint::Labels;
use kernel::{Fields, Principal};

use crate::update::Took;

/// A request to start, or update, an instance of a blueprint.
#[derive(Clone, Debug)]
pub struct Apply {
    /// The blueprint's name, looked up in the library unless `source` is given.
    pub blueprint: String,
    /// The blueprint's text, when applying something not (yet) in the library.
    pub source: Option<String>,
    pub params: Fields,
    /// The instance's name within its parent. Defaults to the blueprint's name.
    pub key: Option<String>,
    pub parent: Option<String>,
    /// The instance to attach to, for a blueprint started by `uses`.
    pub target: Option<String>,
    pub project: Option<String>,
    pub principal: Principal,
    /// Run the steps again even if nothing changed.
    pub force: bool,
    /// Take the project's own blueprints as they now are ([`crate::vouched`]).
    pub take: bool,
    /// What everything measured under the instance also carries.
    pub labels: Labels,
    /// The environment of its project's the instance is to run under,
    /// where somebody asks for one: what a step starts, and what an
    /// instance applies for itself, run under that instance's
    /// ([`crate::environs`]).
    pub environment: Option<String>,
}

impl Apply {
    pub fn new(blueprint: impl Into<String>, principal: Principal) -> Self {
        Self {
            blueprint: blueprint.into(),
            source: None,
            params: Fields::new(),
            key: None,
            parent: None,
            target: None,
            project: None,
            principal,
            force: false,
            take: false,
            labels: Labels::new(),
            environment: None,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Applied {
    pub instance: String,
    /// The run that was started, if one was. Where what was asked for is
    /// what the instance is in the middle of, nothing was started and this
    /// is the run that is going, for whoever asked to wait on.
    pub run: Option<String>,
    /// A new instance was created.
    pub created: bool,
    /// Something was different from what was already running.
    pub changed: bool,
    /// For an instance that was there and now runs something else: how it
    /// came to it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub took: Option<Took>,
    /// What became of the project's own blueprints that are other than they
    /// were when this was last applied ([`crate::vouched`]).
    #[serde(default, skip_serializing_if = "crate::Vouched::is_empty")]
    pub vouched: crate::Vouched,
    /// What became of the texts that ship among those it reaches, where
    /// they are other than they were when it was last applied
    /// ([`crate::pinned`]).
    #[serde(default, skip_serializing_if = "crate::Reached::is_empty")]
    pub reached: crate::Reached,
}
