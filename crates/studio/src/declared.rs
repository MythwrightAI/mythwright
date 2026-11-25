//! What a blueprint says a collection is, and what changing one takes.
//!
//! A collection's records are changed by whoever may change records
//! (`records:write`; a board's cards, `board:write`). Some tables are more
//! than what they hold: a fleet's `agents` and `stages` are read by a
//! blueprint that brings agents up, tells them what they are for and says
//! how much each may do, with the authority of whoever applied it. Whoever
//! could change a row of those could do all of that. So a collection may
//! say what changing its records takes besides (`takes`), and it is held to
//! wherever a record of it is added, changed, spoken on or taken away, and
//! wherever it is declared again. And one whose records say what is done
//! with what another's give (a fleet's `stages`, of its `agents`) may say
//! that it steers that one (`steers`): a record of it is then written by
//! whoever could write those.

use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};

use crate::board::BOARD;
use crate::collections::{Collection, Store, View};
use crate::fields::Field;
use crate::history::Rule;

impl Collection {
    /// Refuses unless `by` holds what changing a record of this takes.
    pub fn may_change(&self, by: &Principal) -> Result<()> {
        by.require(if self.name == BOARD { "board:write" } else { "records:write" })?;
        // What may not be read is not changed blind either.
        self.open_to(by)?;
        match &self.takes {
            Some(scope) => by.require(scope),
            None => Ok(()),
        }
    }

    /// Refuses unless `by` holds what reading a record of this takes.
    pub fn may_read(&self, by: &Principal) -> Result<()> {
        by.require(if self.name == BOARD { "board:read" } else { "records:read" })?;
        self.open_to(by)
    }

    /// Refuses where it is kept private and `by` does not hold what it is
    /// kept under.
    pub fn open_to(&self, by: &Principal) -> Result<()> {
        match self.reads.as_deref().filter(|scope| !by.can(scope)) {
            Some(scope) => Err(Error::denied(format!(
                "`{}` in `{}` is kept private: its records take `{scope}`, which {} does not hold",
                self.name, self.project, by.name
            ))),
            None => Ok(()),
        }
    }
}

impl crate::Collections {
    /// Refuses unless `by` could write each record of the collection this
    /// one steers, as each stands: what its cells say giving them their
    /// values takes, and each scope a list of its own gives. Asked
    /// wherever a record of a collection that steers another is added or
    /// changed. What is done with what those records give goes by what a
    /// record here says, with the authority of whatever reads the two; so
    /// a record here is written by somebody who could have written those,
    /// and is no way round what they hold. A collection that is not there
    /// has no records to steer.
    pub async fn steered(&self, collection: &Collection, by: &Principal) -> Result<()> {
        let Some(name) = &collection.steers else { return Ok(()) };
        let Some(steered) = self.find(&collection.project, name).await? else { return Ok(()) };
        for record in self.records(&steered).await? {
            let said = record
                .data
                .iter()
                .filter(|(_, value)| !value.is_null() && *value != &json!("") && *value != &json!([]))
                .map(|(name, value)| (name.as_str(), value, Some(value)));
            crate::fields::may_set(&steered, &record.data, said, by).map_err(|refused| {
                Error::denied(format!(
                    "a record of `{}` steers the records of `{name}`, and is added or changed only by whoever \
                     could write each of those as it stands. Of {}: {}",
                    collection.name, record.id, refused.message
                ))
            })?;
        }
        Ok(())
    }
}

/// What answers whether a record is the row the agent named is brought
/// up by: handed the agent and the record's id. Which agents a row
/// brings up is not the studio's to know, so whoever keeps the agents
/// says it.
pub type OwnRow = std::sync::Arc<
    dyn Fn(String, String) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>> + Send + Sync,
>;

impl crate::Collections {
    /// Says what answers whether a record is an agent's own row. Said
    /// once, as the daemon starts; with nothing said no row is anybody's.
    pub fn own_rows_with(&self, own: OwnRow) {
        let _ = self.own.set(own);
    }

    /// Refuses an agent changing or taking away the row of a table whose
    /// rows give agents what they hold (a field that `takes: each`) that
    /// brings it up: what an agent holds and may reach is set by others,
    /// and never by itself, even within what it holds.
    pub async fn not_its_own(&self, collection: &Collection, id: &str, by: &Principal) -> Result<()> {
        let gives = collection.fields.iter().any(|field| field.takes_each);
        let Some(own) = self.own.get().filter(|_| gives && by.kind == kernel::PrincipalKind::Agent) else {
            return Ok(());
        };
        if !own(by.id.clone(), id.to_string()).await {
            return Ok(());
        }
        Err(Error::denied(format!(
            "{id} is the row `{}` is brought up by, and an agent never changes its own slot, even within what \
             it holds: what it may ask and reach is set by a person, or by an agent that holds what it gives. \
             Ask somebody who may change agents",
            by.id
        )))
    }
}

/// Refuses a scope that no credential can hold: a table or a field that
/// took one could be changed by nobody.
pub(crate) fn held_by_somebody(scope: &str) -> Result<()> {
    let area = |scope: &str| scope.split(':').next().unwrap_or(scope).to_string();
    if kernel::principal::SCOPES.iter().any(|(one, _)| area(one) == area(scope)) {
        return Ok(());
    }
    let all: Vec<&str> = kernel::principal::SCOPES.iter().map(|(one, _)| *one).collect();
    Err(Error::invalid(format!(
        "`{scope}` is not something a credential can hold, so nobody could change the records; there are: {}",
        all.join(", ")
    )))
}

/// What a blueprint says a collection is.
#[derive(Clone, Debug)]
pub struct Spec {
    pub fields: Vec<Field>,
    pub views: Vec<View>,
    /// Which events are the history of its records (`history::rules`).
    pub history: Vec<Rule>,
    pub store: Store,
    /// For a store service: its address. One declared again without it
    /// stays where it was.
    pub at: Option<String>,
    /// The model it says it is (`model`, as a blueprint writes it).
    pub model: Option<String>,
    /// The scope changing its records takes beside what changing any
    /// record does (`takes`, as a blueprint writes it).
    pub takes: Option<String>,
    /// The scope reading its records takes beside what reading any record
    /// does (`reads`, as a blueprint writes it).
    pub reads: Option<String>,
    /// The collection whose records its own steer (`steers`, as a
    /// blueprint writes it).
    pub steers: Option<String>,
    /// How it is shown: the table it sits beside, and its columns in
    /// groups (`beside`, `about` and `groups`, as a blueprint writes them).
    pub shown: crate::shown::Shown,
    /// The blueprint whose step declares it, where one does: what it
    /// does to the records as it is declared is said to be done through
    /// that ([`crate::reworded`]).
    pub through: Option<String>,
    /// Whether it is only to be made anew (`anew`, as a blueprint writes
    /// it): a table the project has that something else declared is not
    /// declared over.
    pub anew: bool,
}

impl Spec {
    /// The same, made only where the project has no table of its name
    /// that something else declared: `anew`, as a blueprint writes it.
    pub fn anew(mut self, anew: bool) -> Spec {
        self.anew = anew;
        self
    }

    /// The same, declared by a step of the blueprint `blueprint`.
    pub fn declared_by(mut self, blueprint: &str) -> Spec {
        self.through = Some(blueprint.to_string()).filter(|blueprint| !blueprint.is_empty());
        self
    }

    /// The same, kept by the store service at this address: `at`, as a
    /// blueprint writes it. Nothing written is no address.
    pub fn kept_at(mut self, at: Option<&str>) -> Spec {
        self.at = at.map(str::trim).filter(|at| !at.is_empty()).map(str::to_string);
        self
    }

    /// The same, said to be a model's: `model`, as a blueprint writes it.
    /// Nothing written says nothing.
    pub fn modelled(mut self, model: Option<&str>) -> Spec {
        self.model = model.map(str::trim).filter(|model| !model.is_empty()).map(str::to_string);
        self
    }

    /// The same, with what changing its records takes: `takes`, as a
    /// blueprint writes it, which is a scope there is. A fleet's tables
    /// say `agents:write`: whoever changes a row changes the agents.
    pub fn taking(mut self, takes: Option<&str>) -> Result<Spec> {
        self.takes = takes.map(str::trim).filter(|scope| !scope.is_empty()).map(str::to_string);
        match &self.takes {
            Some(scope) => held_by_somebody(scope).map(|()| self),
            None => Ok(self),
        }
    }

    /// The same, said to steer the records of another collection of its
    /// project: `steers`, as a blueprint writes it, which is that
    /// collection's name. What those records do, and for whom, then goes
    /// by what a record of this one says, so one of this is added or
    /// changed by whoever could write each of those as it stands
    /// (`Collections::steered`). A fleet's `stages` say `agents`: a stage
    /// gives the fleet's agents their work.
    pub fn steering(mut self, steers: Option<&str>) -> Spec {
        self.steers = steers.map(blueprint::expr::slug).filter(|name| !name.is_empty());
        self
    }

    /// The same, kept private: `reads`, as a blueprint writes it, the
    /// scope its records take to be read, and with that to be changed,
    /// spoken on or heard of. A person's notes say `private:notes`, and
    /// whoever is to read them is given that.
    pub fn read_by(mut self, reads: Option<&str>) -> Result<Spec> {
        self.reads = reads.map(str::trim).filter(|scope| !scope.is_empty()).map(str::to_string);
        match &self.reads {
            Some(scope) => held_by_somebody(scope).map(|()| self),
            None => Ok(self),
        }
    }

    /// The same, with what its blueprint says is the history of its
    /// records: `history`, as a blueprint writes it.
    pub fn with_history(mut self, written: &Value) -> Result<Spec> {
        self.history = crate::history::rules(written)?;
        Ok(self)
    }

    /// Reads `fields` and `views` as a blueprint writes them: each a map
    /// from a name to what it is.
    pub fn read(fields: &Value, views: &Value, store: Option<&str>) -> Result<Spec> {
        let store = match store {
            None => Store::Local,
            Some(named) => Store::parse(named).ok_or_else(|| {
                Error::invalid(format!(
                    "`{named}` is not somewhere a collection can be kept; there are `local` (this installation's database), `files` (files in the project), `service` (a store service, at the address given as `at`), `documents` (the Markdown files of a folder of the project, as they are) and `mockups` (the folders of a folder of the project that each hold a page)"
                ))
            })?,
        };
        let named = |value: &Value, what: &str| -> Result<Vec<(String, Map<String, Value>)>> {
            let map = value.as_object().ok_or_else(|| {
                Error::invalid(format!("`{what}` must be a map from each one's name to what it is"))
            })?;
            Ok(map
                .iter()
                .map(|(name, about)| (name.clone(), about.as_object().cloned().unwrap_or_default()))
                .collect())
        };
        let read = crate::fields::read(named(fields, "fields")?)?;
        let mut looked = Vec::new();
        for (name, about) in if views.is_null() { Vec::new() } else { named(views, "views")? } {
            let kind = about.get("kind").and_then(Value::as_str).unwrap_or("table").to_string();
            let by = about.get("by").and_then(Value::as_str).map(str::to_string);
            let hides = match about.get("hides") {
                None | Some(Value::Null) => None,
                Some(Value::Array(names)) => Some(names.iter().map(blueprint::expr::display).collect()),
                Some(_) => {
                    return Err(Error::invalid(format!(
                        "the view `{name}` says `hides`, which is a list of the fields it leaves out, as in `hides: [notes]`"
                    )));
                }
            };
            let said = |key: &str| about.get(key).and_then(Value::as_str).map(str::to_string);
            let only = View::held_to(&name, about.get("only"))?;
            let (group, done, shown) =
                (said("group"), said("done"), kernel::columns::Columns::read(&name, &about)?);
            let view = View { name, kind, by, saved: false, hides, group, done, only, shown };
            // Whether it goes by a field there is, is asked when the
            // collection is declared: by then the fields a model gives it
            // are among them, and a field that is not what its model
            // says has been said to be that, and not a view's trouble.
            view.laid_out()?;
            looked.push(view);
        }
        Ok(Spec {
            fields: read,
            views: looked,
            history: Vec::new(),
            store,
            at: None,
            model: None,
            takes: None,
            reads: None,
            steers: None,
            shown: Default::default(),
            through: None,
            anew: false,
        })
    }
}
