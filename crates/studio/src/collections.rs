//! Collections: typed records that a blueprint declares.
//!
//! A collection is the data of a component such as a board: what its
//! records are made of, how they can be looked at, and where they are kept.
//! Nothing makes one but a blueprint saying so (`collection.ensure`), and
//! the blueprint chooses the store: this installation's own database;
//! files in the project, one a record, which people, agents and whatever
//! keeps the project's history can read as they are; a store service, a
//! database some other blueprint made and hosts (`service`); or, for a
//! library, the Markdown files of a folder as they are (`documents`).
//!
//! This module knows nothing of boards. What a board does with a collection
//! (columns, the order of cards, what moving one means) is in `board`.

use journal::{Journal, Prepared, optional};
use kernel::{Error, Principal, Result, Source};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::Projects;
pub use crate::declared::Spec;
use crate::fields::check;
pub use crate::fields::{Field, KINDS, Takes};
use crate::history::Rule;
use crate::service::StoreService;
pub(crate) use crate::tables::COLLECTIONS as SCHEMA;
pub use crate::views::View;
pub use crate::whole::Say;

/// When a record was made and when it was last changed: every record has
/// both without declaring them, and a calendar can go by either.
pub const STAMPS: &[&str] = &["created", "updated"];

/// The ways a collection can be laid out. Each is the same records: a
/// `table` is a row a record with a column a field; a `board` is the
/// records in columns by one of their choices; a `list` is a line a
/// record; a `gallery` is a card a record; a `calendar` is the records on
/// the days one of their dates says.
pub const LAYOUTS: &[&str] = &["table", "board", "list", "gallery", "calendar", "order"];

/// Where a collection's records are kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    /// In this installation's own database.
    Local,
    /// As files in the project, one a record, in a folder named after the
    /// collection.
    Files,
    /// By a store service: a process some blueprint made and hosts, which
    /// answers at the collection's address (`at`).
    Service,
    /// As the Markdown files of a folder of the project (`at`; the one
    /// called what the collection is, unless said), each as it is: a
    /// library ([`crate::documents`]).
    Documents,
    /// As the folders of a folder of the project (`at`; the one called
    /// what the collection is, unless said) that each hold a page: mockups
    /// ([`crate::mockups`]).
    Mockups,
}

impl Store {
    pub fn parse(text: &str) -> Option<Store> {
        match text {
            "local" => Some(Store::Local),
            "files" => Some(Store::Files),
            "service" => Some(Store::Service),
            "documents" => Some(Store::Documents),
            "mockups" => Some(Store::Mockups),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Store::Local => "local",
            Store::Files => "files",
            Store::Service => "service",
            Store::Documents => "documents",
            Store::Mockups => "mockups",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Collection {
    pub project: String,
    pub name: String,
    /// The model it is, where it is one: what this program itself reads
    /// of its records (`models`). Its fields are then at least the model's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub fields: Vec<Field>,
    pub views: Vec<View>,
    /// Which events are the history of its records, and how each is worded.
    #[serde(default)]
    pub history: Vec<Rule>,
    pub store: Store,
    /// For one kept by a store service: where that service answers.
    pub at: Option<String>,
    /// The instance whose blueprint declared it.
    pub owner: Option<String>,
    /// The scope changing its records takes, beside what changing any
    /// record does (`declared`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub takes: Option<String>,
    /// The scope reading its records takes, beside what reading any record
    /// does: what it is kept private under (`closed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reads: Option<String>,
    /// The collection of the same project whose records its own steer:
    /// what those do, and for whom, goes by what a record here says. So a
    /// record here is added or changed by whoever could write each of
    /// those as it stands (`steered`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steers: Option<String>,
    /// What it says of how it is shown, and nothing else (`shown`): the
    /// table it sits beside, and its columns in groups.
    #[serde(flatten)]
    pub shown: crate::shown::Shown,
    pub created: i64,
    pub updated: i64,
}

impl Collection {
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.name == name)
    }
}

// A record as it is read from its row came out of this crate with a
// table read a window at a time (`windowed`), and is offered under the
// names it had here.
pub(crate) use ::windowed::{RECORD, read_record};
pub use ::windowed::{Record, record_id};

#[derive(Clone)]
pub struct Collections {
    /// The factory's own journal: its clock, its ids, what is said that
    /// is no one table's, and the way to the journal that keeps a
    /// project's rows ([`Collections::rows`]). No table of a project's
    /// is asked for through this by itself.
    factory: Journal,
    projects: Projects,
    pub(crate) service: StoreService,
    /// Held by whoever reads a record in order to change it, until it has
    /// written it back ([`Collections::changing`]).
    changing: std::sync::Arc<Vec<std::sync::Arc<tokio::sync::Mutex<()>>>>,
    /// What says whether a record is the row an agent is brought up by
    /// (`own_rows_with`); nothing until whoever keeps the agents says it.
    pub(crate) own: std::sync::Arc<std::sync::OnceLock<crate::declared::OwnRow>>,
    /// The factory's bot, whose doing what a blueprint's step does to a
    /// record is said to be (`history`); told as histories are first kept.
    pub(crate) bot: std::sync::Arc<std::sync::OnceLock<talk::Speaker>>,
    /// Which blueprint's step said what says no `through` of its own (`crate::through`).
    pub(crate) said_by: crate::through::SaidBy,
    /// How many records each table kept here has, as far as it is known (`counted`).
    pub(crate) counted: crate::counted::Counted,
    /// Which columns are kept in order by an index (`indexes`), and what was counted of a view (`window`).
    pub(crate) indexes: crate::indexes::Indexes,
    pub(crate) counts: crate::window::Counts,
    /// Where the files of what is attached to records are kept, the
    /// folder a channel's attachments are kept in (`crate::attaching`).
    pub(crate) attached: std::sync::Arc<std::sync::OnceLock<std::path::PathBuf>>,
    /// What more the write of a record of a model does (`factory::Written`).
    pub(crate) written: std::sync::Arc<std::sync::OnceLock<std::sync::Arc<dyn crate::factory::Written>>>,
}

/// How many records can be changed at the same moment. A record's turn is
/// one of this many, by its id, so two records seldom share one and what
/// is kept is the same however many records there are.
const AT_ONCE: usize = 64;

/// Which of the turns records are changed under is `id`'s.
fn turn_of(id: &str) -> usize {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish() as usize % AT_ONCE
}

pub(crate) fn read_collection(row: &rusqlite::Row<'_>) -> rusqlite::Result<Collection> {
    let name: String = row.get(1)?;
    let mut fields: Vec<Field> = serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default();
    // One that is a model's has the model's fields whatever was written
    // down when it was declared: a board from before cards had a day they
    // are wanted by has one now, and nobody applied anything.
    let said: Option<String> = row.get(10)?;
    let model = crate::models::of(&name, said.as_deref()).ok().flatten();
    if let Some(model) = model {
        model.complete(&mut fields);
    }
    Ok(Collection {
        project: row.get(0)?,
        name,
        model: model.map(|model| model.name.to_string()),
        fields,
        views: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
        store: Store::parse(&row.get::<_, String>(4)?).unwrap_or(Store::Local),
        owner: row.get(5)?,
        created: row.get(6)?,
        updated: row.get(7)?,
        history: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
        at: row.get(9)?,
        takes: row.get(11)?,
        reads: row.get(12)?,
        steers: row.get(13)?,
        shown: crate::shown::Shown::read(row.get(14)?),
    })
}

pub(crate) const COLLECTION: &str = "project, name, fields, views, store, owner, created, updated, history, at, model, takes, reads, steers, beside";
/// What is said of a collection a project has not got.
pub(crate) fn missing(project: &str, name: &str) -> Error {
    Error::not_found(format!("the project `{project}` has no collection called `{name}`"))
}

impl Collections {
    /// The factory's own journal.
    pub(crate) fn factory(&self) -> &Journal {
        &self.factory
    }

    /// The journal whose database keeps `project`'s tables, their
    /// records and each record's history ([`Journal::records_of`]): what
    /// every place here that reads or writes one of those asks, with the
    /// project it has in hand.
    pub(crate) fn rows(&self, project: &str) -> Journal {
        self.factory.records_of(Some(project))
    }

    pub(crate) fn projects(&self) -> &Projects {
        &self.projects
    }

    pub async fn open(journal: Journal, projects: Projects) -> Result<Collections> {
        journal.db().migrate("collections", SCHEMA).await?;
        // A project's journal that keeps its records has the same
        // tables, made as the journal was (`crate::apart::makes`, which
        // `Projects` said as it was opened).
        let service = StoreService::new(journal.clone());
        let changing = std::sync::Arc::new((0..AT_ONCE).map(|_| Default::default()).collect());
        let collections = Collections {
            factory: journal,
            projects,
            service,
            changing,
            own: Default::default(),
            bot: Default::default(),
            said_by: Default::default(),
            counted: Default::default(),
            indexes: Default::default(),
            counts: Default::default(),
            attached: Default::default(),
            written: Default::default(),
        };
        // What each keeps private is known from the start: the files of a
        // project are asked about before anybody declares anything again.
        for table in collections.list(None).await? {
            table.say_kept(&collections.projects);
            collections.reword_left(&table).await;
        }
        Ok(collections)
    }

    /// A record's turn to be changed. A change reads the record, works out
    /// what it becomes and writes the whole of it back; two of those side
    /// by side each write what they read with their own change in it, and
    /// the one that writes last takes the other's away. Whoever changes a
    /// record holds this from before the reading until after the writing,
    /// so that changes to one record are made one after another, each to
    /// what the last left.
    pub async fn changing(&self, id: &str) -> tokio::sync::OwnedMutexGuard<()> {
        let turn = self.changing[turn_of(id)].clone();
        turn.lock_owned().await
    }

    /// The turns of several records, for whoever holds the turns `held`
    /// names already: each taken once and in one order, whatever ids they
    /// are, since several share a turn (there are so many of them, and
    /// no more), and a turn taken twice by one asker, or two taken in
    /// another order by another, would be waited for for ever.
    pub async fn changing_all(&self, held: &[&str], ids: &[String]) -> Vec<tokio::sync::OwnedMutexGuard<()>> {
        let held: Vec<usize> = held.iter().map(|id| turn_of(id)).collect();
        let mut turns: Vec<usize> =
            ids.iter().map(|id| turn_of(id)).filter(|turn| !held.contains(turn)).collect();
        turns.sort_unstable();
        turns.dedup();
        let mut taken = Vec::with_capacity(turns.len());
        for turn in turns {
            taken.push(self.changing[turn].clone().lock_owned().await);
        }
        taken
    }

    /// Makes a collection what `spec` says, creating it if there is none.
    /// Where it is kept is settled when it is made and does not change.
    /// The address of a store service is not where it is kept but how the
    /// same store is reached, and that may be said again: a store that was
    /// started somewhere else is still the store.
    pub async fn ensure(
        &self,
        project: &str,
        name: &str,
        spec: Spec,
        owner: Option<&str>,
        by: &Principal,
    ) -> Result<Collection> {
        by.require("projects:write")?;
        crate::factory::placed(&self.projects, project, spec.store, by).await?;
        let name = blueprint::expr::slug(name);
        // Saying what changing its records takes, or saying it again of one
        // that says so already, takes holding that: nobody declares a
        // table out from under what guards it.
        // A store service is an address this daemon then sends requests
        // to: pointing a table at one is changing what the fleet reaches.
        if spec.store == Store::Service {
            by.require("fleet:write")?;
        }
        crate::within::keeps(&self.projects, spec.store)?;
        let guarded = self.find(project, &name).await?;
        // Made anew, a table is never declared over one the project has that something else declared: a
        // table somebody made from the side panel is not a fleet's, and a fleet's is not theirs.
        // One kept from whoever asks is refused in words that say nothing of it.
        if let Some(there) = guarded.as_ref().filter(|there| spec.anew && there.owner.as_deref() != owner) {
            return Err(Error::conflict(match there.open_to(by) {
                Ok(()) => format!(
                    "`{project}` has a table called `{name}` already{}: call this one something else",
                    if there.owner.is_some() { ", which something else declared" } else { "" }
                ),
                Err(_) => format!("`{name}` cannot be made in `{project}`: call this one something else"),
            }));
        }
        // Nor out from under what its records steer: declared again so
        // that it steers nothing, or something else, anybody who may
        // change its records could then have those do as they say.
        let steered = guarded.as_ref().and_then(|existing| existing.steers.as_ref());
        if let Some(steered) = steered.filter(|steered| spec.steers.as_ref() != Some(*steered))
            && !by.can("admin")
        {
            return Err(Error::denied(format!(
                "a record of `{name}` steers the records of `{steered}`, and declaring `{name}` again so that it \
                 does not takes `admin`, which {} does not hold",
                by.name
            )));
        }
        let guarded = guarded.iter().flat_map(|existing| existing.takes.iter().chain(&existing.reads));
        for scope in guarded.chain(&spec.takes).chain(&spec.reads) {
            by.require(scope)?;
        }
        if spec.reads.is_some() && name == crate::board::BOARD {
            return Err(Error::invalid(
                "a board cannot be kept private (`reads`): its cards are what a project's agents are given to work",
            ));
        }
        if name.is_empty() {
            return Err(Error::invalid("a collection needs a name with at least one letter or digit"));
        }
        // What this program itself reads of its records is held to what it
        // takes them to be, here, where the blueprint is applied.
        let mut spec = spec;
        let model = crate::models::of(&name, spec.model.as_deref())?;
        if let Some(model) = model {
            spec.fields = model.hold(&name, std::mem::take(&mut spec.fields))?;
        }
        // What somebody added to the table stands beside what is declared (`added`).
        let existing = self.find(project, &name).await?;
        spec.fields = crate::added::beside(spec.fields, existing.as_ref());
        spec.shown.fits(&name, &spec.fields, spec.store)?;
        for view in &spec.views {
            view.check(&spec.fields)?;
            view.shows(&spec.fields)?;
            view.leaves_out_fields(&spec.fields)?;
        }
        if let Some(existing) = &existing
            && existing.store != spec.store
        {
            return Err(Error::conflict(format!(
                "`{name}` is kept in `{}` and cannot be moved to `{}` by declaring it again",
                existing.store.as_str(),
                spec.store.as_str()
            )));
        }
        self.pointing(project, &name, &spec.fields).await?;
        let at = crate::service::place(&name, spec.store, spec.at.as_deref(), existing.as_ref())?;
        // For a store service this is the moment to find that nothing
        // answers where it is now said to be: when the blueprint is
        // applied, not at the first record somebody adds. What the rows
        // hold is asked only of a field that is to be something else than
        // it was (`held`).
        if spec.store == Store::Service {
            self.service.answers(project, &name, at.clone()).await?;
        }
        let mut reworded = Vec::new();
        if let Some(existing) = &existing {
            let was = Collection { at: at.clone(), ..existing.clone() };
            self.still_chosen(&was, &spec.fields).await?;
            crate::fields::still_guarded(&name, &existing.fields, &spec.fields, by)?;
            self.refit(&was, &spec.fields).await?;
            reworded = crate::held::rewordable(&existing.fields, &spec.fields);
        }
        // Kept with the declaration that begins it, as being under way.
        for field in &mut spec.fields {
            field.rewording = reworded.contains(&field.name);
        }
        let through = spec.through.clone();
        let now = self.factory.now_ms();
        // The views people saved are theirs, and are kept: one of the same
        // name as the blueprint's stands in its place, the rest come after.
        // One that the fields no longer allow (a board by a choice that is
        // gone) goes with what it was of.
        let saved: Vec<View> = existing
            .iter()
            .flat_map(|existing| existing.views.iter().filter(|view| view.saved))
            .filter(|view| view.check(&spec.fields).is_ok())
            .cloned()
            .collect();
        let mut views: Vec<View> = spec
            .views
            .into_iter()
            .map(|declared| saved.iter().find(|view| view.name == declared.name).cloned().unwrap_or(declared))
            .collect();
        for view in saved {
            if !views.iter().any(|kept| kept.name == view.name) {
                views.push(view);
            }
        }
        let mut collection = Collection {
            project: project.to_string(),
            name: name.clone(),
            model: model.map(|model| model.name.to_string()),
            fields: spec.fields,
            views,
            history: spec.history,
            store: spec.store,
            at,
            owner: owner.map(str::to_string).or_else(|| existing.as_ref().and_then(|c| c.owner.clone())),
            takes: spec.takes,
            reads: spec.reads,
            steers: spec.steers,
            shown: spec.shown,
            created: existing.as_ref().map_or(now, |c| c.created),
            updated: now,
        };
        self.keep(&mut collection).await?;
        collection.say_kept(&self.projects);
        // A number from before comes to a word of a choice now (`reworded`).
        self.reword(&collection, &reworded, by, through.as_deref()).await?;
        self.index_declared(&collection, by).await?;
        let data = json!({
            "project": project, "name": name, "store": collection.store.as_str(), "at": collection.at,
            "owner": collection.owner, "made": existing.is_none(),
        });
        let _ = self.factory.emit("collection.ensured", Source::actor(by.actor()), data).await;
        Ok(collection)
    }

    /// Writes down what a collection is, as it was just declared. What
    /// somebody chose for a field's column, and a column somebody added,
    /// is theirs and not the declaration's, and is the row's own in this
    /// write (`added::carry`).
    async fn keep(&self, collection: &mut Collection) -> Result<()> {
        let mut row = collection.clone();
        (collection.fields, collection.shown) = self
            .rows(&collection.project)
            .db()
            .call(move |connection| {
                crate::added::carry(connection, &mut row)?;
                connection.execute_kept(
                    &format!("INSERT OR REPLACE INTO collections ({COLLECTION}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"),
                    params![
                        row.project,
                        row.name,
                        serde_json::to_string(&row.fields).unwrap_or_default(),
                        serde_json::to_string(&row.views).unwrap_or_default(),
                        row.store.as_str(),
                        row.owner,
                        row.created,
                        row.updated,
                        serde_json::to_string(&row.history).unwrap_or_default(),
                        row.at,
                        row.model,
                        row.takes,
                        row.reads,
                        row.steers,
                        row.shown.kept()
                    ],
                )?;
                Ok((row.fields, row.shown))
            })
            .await?;
        Ok(())
    }

    /// Saves a view of a collection for whoever looks at it next: a new
    /// one, or the one of that name as it now is. It is kept when the
    /// blueprint that declared the collection is applied again.
    pub async fn save_view(
        &self,
        project: &str,
        name: &str,
        view: View,
        by: &Principal,
    ) -> Result<Collection> {
        // A way of looking at a table is the table's: saved by whoever
        // may change its records, a board's by whoever may change cards.
        if !by.can("board:write") {
            by.require("records:write")?;
        }
        let mut collection = self.get(project, name).await?;
        collection.may_change(by)?;
        let view = view.into_saved();
        if view.name.is_empty() {
            return Err(Error::invalid("a view needs a name"));
        }
        view.check(&collection.fields)?;
        view.shows(&collection.fields)?;
        match collection.views.iter_mut().find(|kept| kept.name == view.name) {
            Some(kept) => *kept = view.clone(),
            None => collection.views.push(view.clone()),
        }
        self.viewed(collection, &view.name, by).await
    }

    /// Takes a view of a collection away. Its records are not touched: a
    /// view is only a way of looking at them. The last one stays.
    pub async fn remove_view(
        &self,
        project: &str,
        name: &str,
        view: &str,
        by: &Principal,
    ) -> Result<Collection> {
        if !by.can("board:write") {
            by.require("records:write")?;
        }
        let mut collection = self.get(project, name).await?;
        collection.may_change(by)?;
        if !collection.views.iter().any(|kept| kept.name == view) {
            return Err(Error::not_found(format!("`{name}` has no view called `{view}`")));
        }
        if collection.views.len() == 1 {
            return Err(Error::conflict(format!(
                "`{view}` is the only view of `{name}`; save another before taking it away"
            )));
        }
        collection.views.retain(|kept| kept.name != view);
        self.viewed(collection, view, by).await
    }

    async fn viewed(&self, collection: Collection, view: &str, by: &Principal) -> Result<Collection> {
        // Only the views are written, and the row is handed back as it
        // then is: the rest of it is as whoever wrote it last left it (an
        // icon chosen for a field), whatever was read here a moment ago.
        let row = (collection.project.clone(), collection.name.clone(), self.factory.now_ms());
        let views = serde_json::to_string(&collection.views).unwrap_or_default();
        let collection = self
            .rows(&collection.project)
            .db()
            .call(move |connection| {
                connection.execute_kept(
                    "UPDATE collections SET views = ?3, updated = ?4 WHERE project = ?1 AND name = ?2",
                    params![row.0, row.1, views, row.2],
                )?;
                connection.query_row_kept(
                    &format!("SELECT {COLLECTION} FROM collections WHERE project = ?1 AND name = ?2"),
                    [&row.0, &row.1],
                    read_collection,
                )
            })
            .await?;
        let data = json!({ "project": collection.project, "name": collection.name, "view": view });
        let _ = self.factory.emit("collection.views", Source::actor(by.actor()), data).await;
        Ok(collection)
    }

    pub(crate) async fn find(&self, project: &str, name: &str) -> Result<Option<Collection>> {
        let (project, name) = (project.to_string(), name.to_string());
        self.rows(&project)
            .db()
            .call(move |connection| {
                optional(connection.query_row_kept(
                    &format!("SELECT {COLLECTION} FROM collections WHERE project = ?1 AND name = ?2"),
                    [&project, &name],
                    read_collection,
                ))
            })
            .await
    }

    pub async fn get(&self, project: &str, name: &str) -> Result<Collection> {
        self.find(project, name).await?.ok_or_else(|| missing(project, name))
    }

    /// The collections of one project, or of all of them: asked of the
    /// journal that keeps the project's, or of each that keeps any.
    pub async fn list(&self, project: Option<&str>) -> Result<Vec<Collection>> {
        let asked = match project {
            Some(project) => vec![self.rows(project)],
            None => self.factory.every_records(),
        };
        let mut found = Vec::new();
        for journal in asked {
            let project = project.map(str::to_string);
            let listed: Vec<Collection> = journal
                .db()
                .call(move |connection| {
                    let mut statement = connection.prepare_cached(&format!(
                        "SELECT {COLLECTION} FROM collections WHERE ?1 IS NULL OR project = ?1 ORDER BY project, name"
                    ))?;
                    let rows = statement.query_map([&project], read_collection)?;
                    rows.collect()
                })
                .await?;
            found.extend(listed);
        }
        found.sort_by(|one, other| (&one.project, &one.name).cmp(&(&other.project, &other.name)));
        Ok(found)
    }

    /// The tables of `project` that may own the record `id`, numbered
    /// `number`, in the order [`Collections::list`] gives them: of every
    /// table only its name and where it keeps its rows are read, and only
    /// one that may own it is read whole, in the same piece of work.
    async fn may_own(&self, project: &str, id: &str, number: i64) -> Result<Vec<Collection>> {
        let db = self.rows(project).db().clone();
        let (project, id) = (project.to_string(), id.to_string());
        let read = move |connection: &mut rusqlite::Connection| {
            let mut named = connection.prepare_cached(
                "SELECT name, store FROM collections WHERE project = ?1 ORDER BY project, name",
            )?;
            let tables: Vec<(String, String)> = named
                .query_map([&project], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            let mut whole = connection.prepare_cached(&format!(
                "SELECT {COLLECTION} FROM collections WHERE project = ?1 AND name = ?2"
            ))?;
            let mut found = Vec::new();
            for (name, store) in tables {
                let may = match Store::parse(&store) {
                    Some(Store::Documents | Store::Mockups) => true,
                    _ => record_id(&project, &name, number) == id,
                };
                if may {
                    found.extend(optional(whole.query_row([&project, &name], read_collection))?);
                }
            }
            Ok(found)
        };
        db.call(read).await
    }

    /// Every record of a collection, in the order they were made.
    pub async fn records(&self, collection: &Collection) -> Result<Vec<Record>> {
        self.written_since(collection, i64::MIN).await
    }

    /// The records of a collection written at a moment or after it, and
    /// those that do not say when they were written (a store service's).
    /// Of a collection kept here the rest are not read at all: whoever
    /// wants only what changed of late does not pay for the whole table.
    pub async fn written_since(&self, collection: &Collection, since: i64) -> Result<Vec<Record>> {
        let mut records = self.kept(collection, since).await?;
        records.retain(|record| record.updated >= since || record.updated == 0);
        // Only what the collection declares is read. A field that was
        // declared once and is not now may still be written in records
        // from then; a record is checked whole when it is written, so
        // handing that back would make every one of them unwritable.
        // It is left out here and gone for good the next time each is
        // written.
        for record in &mut records {
            record.data.retain(|name, _| collection.field(name).is_some());
        }
        Ok(records)
    }

    /// The records as they are where they are kept: all of them, but
    /// where the journal's database can be asked for those written since.
    async fn kept(&self, collection: &Collection, since: i64) -> Result<Vec<Record>> {
        match collection.store {
            Store::Local => {
                let (project, name) = (collection.project.clone(), collection.name.clone());
                self.rows(&project)
                    .db()
                    .call(move |connection| {
                        let mut statement = connection.prepare_cached(&format!(
                            "SELECT {RECORD} FROM records
                             WHERE project = ?1 AND collection = ?2 AND (updated >= ?3 OR updated = 0)
                             ORDER BY number"
                        ))?;
                        let rows =
                            statement.query_map(rusqlite::params![project, name, since], read_record)?;
                        rows.collect()
                    })
                    .await
            }
            Store::Files => {
                let dir = self.projects.dir_of(&collection.project).ok_or_else(|| {
                    Error::not_found(format!("there is no project `{}`", collection.project))
                })?;
                let about = collection.clone();
                tokio::task::spawn_blocking(move || {
                    let mut found = Vec::new();
                    for entry in std::fs::read_dir(dir.join(&about.name)).into_iter().flatten().flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let Some(number) = name.strip_suffix(".md").and_then(|n| n.parse::<i64>().ok())
                        else {
                            continue;
                        };
                        if let Ok(text) = std::fs::read_to_string(entry.path()) {
                            found.push(files::read(&about, number, &text));
                        }
                    }
                    found.sort_by_key(|record: &Record| record.number);
                    found
                })
                .await
                .map_err(|err| Error::internal(format!("reading the records crashed: {err}")))
            }
            Store::Service => self.service.records(collection).await,
            Store::Documents => self.documents(collection).await,
            Store::Mockups => self.mockups(collection).await,
        }
    }

    /// One record, by its id, with the collection it is in.
    pub async fn record(&self, id: &str) -> Result<(Collection, Record)> {
        let missing = || Error::not_found(format!("there is no record `{id}`"));
        // A document is known by where its file is, and is read from there.
        if id.contains(':') {
            if let Some(found) = self.mockup(id).await? {
                return Ok(found);
            }
            return self.document(id).await?.ok_or_else(missing);
        }
        // An id is a name, and nothing is read from where its dashes
        // are: a project's name and a table's may each be several words
        // joined by them. It begins with its project and ends with its
        // number, so each project it may be of is asked for its tables,
        // and the one that owns the id (`Collection::owns`) has it.
        let number = id.rsplit_once('-').and_then(|(_, number)| number.parse::<i64>().ok());
        let number = number.ok_or_else(missing)?;
        for (at, _) in id.rmatch_indices('-') {
            // Only a table that may own the id is read whole: what a table
            // says of itself is read and worked out each time, and a card
            // is looked up at every move and every write on it.
            for collection in self.may_own(&id[..at], id, number).await? {
                if !collection.owns(id) {
                    continue;
                }
                if let Some(found) = self.one(&collection, id, number).await? {
                    return Ok((collection, found));
                }
            }
        }
        Err(missing())
    }

    /// One record of a collection, without reading the rest. A card is
    /// looked up each time it is read, moved or written on; found by
    /// reading its whole board, a board of a thousand cards was read a
    /// thousand times over to work through it.
    pub(crate) async fn one(&self, collection: &Collection, id: &str, number: i64) -> Result<Option<Record>> {
        let found = match collection.store {
            Store::Local => {
                let (id, project, name) =
                    (id.to_string(), collection.project.clone(), collection.name.clone());
                let by_id = format!(
                    "SELECT {RECORD} FROM records WHERE id = ?1 AND project = ?2 AND collection = ?3"
                );
                self.rows(&collection.project)
                    .db()
                    .call(move |connection| {
                        journal::optional(connection.query_row_kept(
                            &by_id,
                            [&id, &project, &name],
                            read_record,
                        ))
                    })
                    .await?
            }
            Store::Files => {
                let dir = self.projects.dir_of(&collection.project);
                let file = dir.map(|dir| dir.join(&collection.name).join(format!("{number}.md")));
                let text = match file {
                    Some(file) => tokio::fs::read_to_string(file).await.ok(),
                    None => None,
                };
                text.map(|text| files::read(collection, number, &text)).filter(|record| record.id == id)
            }
            // Asked of whatever keeps them, which is asked for them all.
            _ => self.kept(collection, i64::MIN).await?.into_iter().find(|record| record.id == id),
        };
        Ok(found.map(|mut record| {
            record.data.retain(|name, _| collection.field(name).is_some());
            record
        }))
    }

    /// Adds a record. `data` is checked against the collection's fields,
    /// and a record it names must be there.
    pub async fn add(
        &self,
        collection: &Collection,
        data: Map<String, Value>,
        position: f64,
        by: &Principal,
    ) -> Result<Record> {
        self.add_saying(collection, data, position, by, |_| Vec::new()).await
    }

    /// The same, with what `say` says of the record kept in the write
    /// that keeps the record ([`crate::whole`]).
    pub async fn add_saying(
        &self,
        collection: &Collection,
        data: Map<String, Value>,
        position: f64,
        by: &Principal,
        say: impl FnOnce(&Record) -> Vec<Say> + Send,
    ) -> Result<Record> {
        self.add_numbered(collection, data, (position, None), by, say).await
    }

    /// The same, numbered `number` where one is said ([`crate::records::Records::keep`]):
    /// refused where a record of the table has that number.
    pub async fn add_numbered(
        &self,
        collection: &Collection,
        data: Map<String, Value>,
        (position, number): (f64, Option<i64>),
        by: &Principal,
        say: impl FnOnce(&Record) -> Vec<Say> + Send,
    ) -> Result<Record> {
        let mut data = check(collection, data)?;
        let said = |value: &Value| !value.is_null() && *value != json!("") && *value != json!([]);
        let named: Vec<String> =
            data.iter().filter(|(_, value)| said(value)).map(|(name, _)| name.clone()).collect();
        crate::fields::stamp(collection, &mut data, &named, by, true)?;
        let given = data.iter().filter(|(_, value)| said(value));
        crate::fields::may_set(
            collection,
            &data,
            given.map(|(name, value)| (name.as_str(), value, None)),
            by,
        )?;
        self.steered(collection, by).await?;
        self.may_name(collection, &data, None, by).await?;
        self.pointed(collection, &data, None).await?;
        if matches!(collection.store, Store::Documents | Store::Mockups) {
            let record = match collection.store {
                Store::Documents => self.add_document(collection, data, by).await?,
                _ => self.add_mockup(collection, data, by).await?,
            };
            self.say_after(say(&record)).await;
            return Ok(record);
        }
        let now = self.factory.now_ms();
        // A record is numbered after the last, and two added in the same
        // moment each found the same last: both were given one number, and
        // the second was written over the first, which was gone with
        // nobody told. They are numbered one at a time.
        let _turn =
            self.changing(&format!("a new record of {}/{}", collection.project, collection.name)).await;
        let number = match number {
            Some(number)
                if self
                    .one(collection, &record_id(&collection.project, &collection.name, number), number)
                    .await?
                    .is_some() =>
            {
                return Err(Error::conflict(format!(
                    "`{}` has a record numbered {number} already",
                    collection.name
                )));
            }
            Some(number) => number,
            None => self.last_number(collection).await? + 1,
        };
        let record = Record {
            id: record_id(&collection.project, &collection.name, number),
            project: collection.project.clone(),
            collection: collection.name.clone(),
            number,
            data,
            position,
            created_by: by.actor(),
            created: now,
            updated: now,
        };
        self.save(collection, &record, say(&record)).await?;
        Ok(record)
    }

    /// The highest number a record of the collection has; 0 for none.
    async fn last_number(&self, collection: &Collection) -> Result<i64> {
        if collection.store != Store::Local {
            return Ok(self
                .kept(collection, i64::MIN)
                .await?
                .iter()
                .map(|record| record.number)
                .max()
                .unwrap_or(0));
        }
        let (project, name) = (collection.project.clone(), collection.name.clone());
        // And after what of it is in the Trash, which is given back under the id it had.
        let highest = format!(
            "SELECT max((SELECT coalesce(max(number), 0) FROM records WHERE project = ?1 AND collection = ?2), ({}))",
            Collections::trashed_number_sql()
        );
        self.rows(&collection.project)
            .db()
            .call(move |connection| connection.query_row_kept(&highest, [&project, &name], |row| row.get(0)))
            .await
    }

    /// Puts a record in exactly as given, numbers and times included:
    /// for moving records in from somewhere they were kept before.
    pub async fn restore(&self, collection: &Collection, record: Record) -> Result<()> {
        self.save(collection, &record, Vec::new()).await
    }

    /// Writes a record that already exists back, changed.
    pub async fn put(&self, collection: &Collection, record: Record) -> Result<Record> {
        self.put_saying(collection, record, |_| Vec::new()).await
    }

    /// The same, with what `say` says of the record as it now is kept in
    /// the write that keeps it ([`crate::whole`]).
    pub async fn put_saying(
        &self,
        collection: &Collection,
        record: Record,
        say: impl FnOnce(&Record) -> Vec<Say> + Send,
    ) -> Result<Record> {
        self.put_also(collection, record, say, None).await
    }

    /// Takes a record away.
    pub async fn remove(&self, collection: &Collection, record: &Record) -> Result<()> {
        self.remove_saying(collection, record, Vec::new()).await
    }
}

/// A record as a file (`windowed::files`), of the fields `collection`
/// declares.
pub(crate) mod files {
    use super::{Collection, Record};

    fn named(collection: &Collection) -> Vec<String> {
        collection.fields.iter().map(|field| field.name.clone()).collect()
    }

    pub fn write(collection: &Collection, record: &Record) -> String {
        ::windowed::files::write(&named(collection), record)
    }

    pub fn read(collection: &Collection, number: i64, text: &str) -> Record {
        ::windowed::files::read(&collection.project, &collection.name, &named(collection), number, text)
    }
}
