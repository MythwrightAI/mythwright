//! The board: work items and where they stand.
//!
//! A card is a piece of work with a status. Moving a card is an event
//! (`board.item.moved`), and that is the hook the rest of the factory hangs
//! on: a fleet blueprint can open a lane when a card becomes ready, post to a
//! channel when one is done, or anything else a handler can do.
//!
//! A board is not something a project is born with. It is a collection that
//! a blueprint declared (`std/board`), called `board`, with a view of it by
//! its `status`; the blueprint also says where its cards are kept. This
//! module is what a board does with that collection: columns, the order of
//! cards in them, and the events that say a card moved.
//!
//! What a card is made of is said once, in `models`: its fields, and the
//! struct this module reads them through. Nothing here reads a card's
//! field out of its record by name.

use journal::Journal;
use kernel::{Error, Principal, Result, Source};
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::collections::{Collection, Collections, Field, Record, Say, Spec, Store, View};
use crate::history::Rule;
use crate::models::{CARD, Card, MOVED, Moved};

/// The name of the collection that is a project's board.
pub const BOARD: &str = "board";

/// What boards were before they were collections: one table for all of
/// them. Kept so that an installation from then is read, once, and its
/// cards moved to where they now live.
pub(crate) const LEGACY: &[&str] = &["
    CREATE TABLE board_items (
        id TEXT PRIMARY KEY,
        project TEXT NOT NULL,
        number INTEGER NOT NULL,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        status TEXT NOT NULL,
        assignee TEXT,
        labels TEXT NOT NULL,
        position REAL NOT NULL,
        created_by TEXT NOT NULL,
        created INTEGER NOT NULL,
        updated INTEGER NOT NULL,
        UNIQUE (project, number)
    );
    CREATE INDEX board_items_project ON board_items (project, status, position);
"];

/// The choice of a card that is the column it is in. A board is its cards
/// by this; what it can be is the board's columns, which the blueprint
/// that makes the board says.
pub const COLUMN: &str = "status";

/// The columns a board has when the blueprint that makes it names none.
pub const DEFAULT_COLUMNS: &[&str] = &["draft", "ready", "doing", "review", "done"];

/// A change a blueprint's step made, and not a person or an agent asking
/// for it: which blueprint, and where in which run. Whose authority it was
/// made with is still whoever applied the blueprint; this is what did it,
/// so that a card's history does not say a person moved a card they never
/// touched.
#[derive(Clone, Debug)]
pub struct Through {
    pub blueprint: String,
    pub source: Source,
    /// The agent whose work it was done from (`agent:<name>`), where the
    /// blueprint was handed that and knew it from what is written: the
    /// architect whose plan a card was added from. It is said, and gives
    /// nobody anything.
    pub wrote: Option<String>,
    /// The agent whose decision it was (`agent:<name>`), where the
    /// blueprint did what an agent decided: a rung of a project's chain.
    /// It is said, and gives nobody anything.
    pub decided: Option<String>,
    /// How many times the card had been moved when the step that moves it
    /// looked at it: where it has been moved since, it is left where it
    /// is and nothing is said. So a step that is kept, done again after a
    /// stop, does not take back a card another stage has taken since.
    pub had: Option<u64>,
    /// Which of the things done under its step's name in its run did it
    /// (`conductor::ActionCx::doing`): said with what a step adds or says
    /// on a record, which the step looks for by it where it is done
    /// again after a stop (`crate::left`). Nothing for no step's doing.
    pub doing: Option<String>,
    /// It sends a card back for something other than a verdict on its
    /// work, which whatever counts how often a card was sent back leaves
    /// out: said with the move.
    pub uncounted: bool,
}

/// Where an event about a card comes from, and what it says did it.
pub(crate) fn told(by: &Principal, through: Option<&Through>, mut data: Value) -> (Source, Value) {
    match through {
        Some(through) => {
            data["through"] = json!(through.blueprint);
            if let Some(wrote) = &through.wrote {
                data["wrote"] = json!(wrote);
            }
            if let Some(decided) = &through.decided {
                data["decided"] = json!(decided);
            }
            (through.source.clone(), data)
        }
        None => (Source::actor(by.actor()), data),
    }
}

/// The same, of a change to a card, which says who made it (`by`): the
/// key, where somebody made it with a key they were given (`said`) and no
/// blueprint's step did; whoever a step acts for, where one did.
pub(crate) fn told_as(
    by: &Principal,
    through: Option<&Through>,
    said: Option<&str>,
    mut data: Value,
) -> (Source, Value) {
    match (through, said) {
        (None, Some(said)) => {
            data["by"] = json!(said);
            (Source::actor(said), data)
        }
        _ => {
            data["by"] = json!(by.actor());
            told(by, through, data)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Item {
    /// `<project>-<number>`.
    pub id: String,
    pub project: String,
    pub number: i64,
    /// What it is: its title, the column it is in and the rest, said
    /// beside the others as they always were.
    #[serde(flatten)]
    pub card: Card,
    /// Order within its column; smaller is higher.
    pub position: f64,
    pub created_by: String,
    pub created: i64,
    pub updated: i64,
}

/// A card on a board is read as the card it is: `item.title`, and not
/// `item.card.title`, everywhere a card was read before the two were told
/// apart.
impl std::ops::Deref for Item {
    type Target = Card;

    fn deref(&self) -> &Card {
        &self.card
    }
}

/// A card as it is added.
#[derive(Clone, Debug, Default)]
pub struct New {
    pub title: String,
    pub body: String,
    /// The column it goes to the bottom of; the first, when none is said.
    pub status: Option<String>,
    pub labels: Vec<String>,
    /// What else its board keeps of a card: the fields the board's
    /// blueprint declared beyond a card's own. A board that needs one of
    /// them is given a card no other way.
    pub more: Map<String, Value>,
}

/// Fields of a card that can be changed without moving it.
#[derive(Clone, Debug, Default)]
pub struct Changes {
    pub title: Option<String>,
    pub body: Option<String>,
    /// `Some(None)` clears the assignee.
    pub assignee: Option<Option<String>>,
    pub labels: Option<Vec<String>>,
    /// `Some(None)` takes the day it is wanted by away.
    pub due: Option<Option<String>>,
    /// What else its board keeps of it, each field by its name: the
    /// sprint a card is in, on a board whose blueprint declared one.
    pub more: Map<String, Value>,
}

/// How a card is changed ([`Board::change`]): who every check is asked
/// of, whom it is said of, and what else the write says and does.
pub struct Doing<'a> {
    pub by: &'a Principal,
    pub said_as: &'a Principal,
    pub through: Option<&'a Through>,
    pub said: Option<&'a str>,
    pub also: Option<(Value, crate::whole::Also)>,
    pub dry: bool,
}

impl Changes {
    /// The change of a card's cells, each by its name as a record has it.
    pub fn of(mut fields: Map<String, Value>) -> Changes {
        let text = |value: Value| value.as_str().map(str::to_string).filter(|text| !text.is_empty());
        let mut take = |name: &str| fields.remove(name);
        Changes {
            title: take("title").and_then(text),
            body: take("body").map(|body| body.as_str().unwrap_or_default().to_string()),
            assignee: take("assignee").map(text),
            labels: take("labels").map(|labels| {
                labels
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|one| one.as_str().map(str::to_string))
                    .collect()
            }),
            due: take("due").map(text),
            more: fields,
        }
    }
}

#[derive(Clone)]
pub struct Board {
    pub(crate) journal: Journal,
    pub(crate) collections: Collections,
    /// The boards looked at since this daemon started for what was
    /// written and never said (`crate::unsaid`): each once for a start.
    pub(crate) settled: std::sync::Arc<tokio::sync::Mutex<std::collections::HashSet<String>>>,
    /// What says whether a card of a project may be given to somebody
    /// (`gives_with`); nothing until whoever keeps the agents says it.
    pub(crate) gives: std::sync::Arc<std::sync::OnceLock<Gives>>,
}

/// What answers whether a card may be given to somebody: handed the
/// card's project and who it is to be given to, as an actor. Who an
/// agent works for is not the studio's to know, so whoever keeps the
/// agents says it, and refuses in its own sentence.
pub type Gives = std::sync::Arc<
    dyn Fn(String, String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

pub(crate) fn item(record: Record) -> Item {
    Item {
        card: Card::read(record.data),
        id: record.id,
        project: record.project,
        number: record.number,
        position: record.position,
        created_by: record.created_by,
        created: record.created,
        updated: record.updated,
    }
}

/// What a board's records are made of, for columns `columns`: a card's
/// fields, as the model has them. A blueprint that makes a board writes
/// the same out, and is held to it.
pub fn spec(columns: &[String], store: Store) -> Spec {
    Spec {
        fields: CARD
            .fields
            .iter()
            .map(|has| has.field(if has.name == COLUMN { columns.to_vec() } else { Vec::new() }))
            .collect(),
        // The board. Whoever looks at it saves the other ways of laying
        // the same records out: a table, a calendar and the rest.
        views: vec![View {
            name: BOARD.into(),
            kind: "board".into(),
            by: Some(COLUMN.into()),
            saved: false,
            hides: None,
            group: None,
            done: None,
            only: None,
            shown: Default::default(),
        }],
        history: rules(),
        store,
        at: None,
        model: Some(CARD.name.into()),
        takes: None,
        reads: None,
        steers: None,
        shown: Default::default(),
        through: None,
        anew: false,
    }
}

/// What is kept of a card's journey across its board, and how it is said.
/// A blueprint that makes a board says the same in its own words.
pub fn rules() -> Vec<Rule> {
    let says = |topic: &str, sentence: &str| Rule {
        topic: topic.into(),
        says: (!sentence.is_empty()).then(|| sentence.to_string()),
    };
    vec![
        says("board.item.created", "{actor} added it to {status}"),
        says("board.item.moved", "{actor} moved it from {from} to {to}[, saying: {note}]"),
        says("board.item.updated", "{actor} changed its {changed}"),
        says("board.item.removed", "{actor} took it away"),
        // What is said on the card itself: whatever anyone says, and what
        // an agent asked instead of guessing, which a lane says of its card.
        says("record.commented", "{actor} said: {body}"),
        says("card.asked", "{asker} asked: {question}"),
        // Anything else that is said of a card, in the usual words.
        says("board.item.*", ""),
    ]
}

impl Board {
    pub async fn open(journal: Journal, collections: Collections) -> Result<Board> {
        journal.db().migrate("board", LEGACY).await?;
        let board = Board { journal, collections, settled: Default::default(), gives: Default::default() };
        board.adopt_legacy().await?;
        Ok(board)
    }

    /// The board for reading its cards, with nothing of who a card may be
    /// given to: for what the channels hold to read a column by. What
    /// answers that holds where projects belong, which holds the channels,
    /// and the board held there whole held itself, so that no studio was
    /// given back.
    pub(crate) fn reading(&self) -> Board {
        Board { gives: Default::default(), ..self.clone() }
    }

    /// Moves the cards of an installation from before boards were
    /// collections to where they now live. A board that already had cards
    /// is not taken away for want of a blueprint.
    async fn adopt_legacy(&self) -> Result<()> {
        type Row =
            (String, String, i64, String, String, String, Option<String>, String, f64, String, i64, i64);
        let rows: Vec<Row> = self
            .journal
            .db()
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT id, project, number, title, body, status, assignee, labels, position, created_by, created, updated
                     FROM board_items ORDER BY rowid",
                )?;
                let rows = statement.query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                        row.get(9)?,
                        row.get(10)?,
                        row.get(11)?,
                    ))
                })?;
                rows.collect()
            })
            .await?;
        if rows.is_empty() {
            return Ok(());
        }
        let system = Principal::system();
        let mut projects: Vec<String> = rows.iter().map(|row| row.1.clone()).collect();
        projects.sort();
        projects.dedup();
        for project in projects {
            let kept = self.journal.kv_get("board.columns", &project).await?;
            let mut columns: Vec<String> = kept
                .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
                .filter(|columns| !columns.is_empty())
                .unwrap_or_else(|| DEFAULT_COLUMNS.iter().map(|column| column.to_string()).collect());
            let cards: Vec<&Row> = rows.iter().filter(|row| row.1 == project).collect();
            // A card in a column the board no longer has keeps its place.
            for card in &cards {
                if !columns.contains(&card.5) {
                    columns.push(card.5.clone());
                }
            }
            let made =
                self.collections.ensure(&project, BOARD, spec(&columns, Store::Local), None, &system).await;
            // Its project is gone: there is nowhere to move its cards to.
            let Ok(collection) = made else { continue };
            for card in cards {
                let was = Card {
                    title: card.3.clone(),
                    body: card.4.clone(),
                    status: card.5.clone(),
                    assignee: card.6.clone(),
                    labels: serde_json::from_str(&card.7).unwrap_or_default(),
                    ..Card::default()
                };
                let record = Record {
                    id: card.0.clone(),
                    project: project.clone(),
                    collection: BOARD.into(),
                    number: card.2,
                    data: was.data(),
                    position: card.8,
                    created_by: card.9.clone(),
                    created: card.10,
                    updated: card.11,
                };
                self.collections.restore(&collection, record).await?;
            }
            let gone = project.clone();
            self.journal
                .db()
                .call(move |c| c.execute("DELETE FROM board_items WHERE project = ?1", [&gone]))
                .await?;
        }
        Ok(())
    }

    /// Makes a project's board, or changes its columns: what `std/board`
    /// does by declaring the collection itself.
    pub async fn make(
        &self,
        project: &str,
        columns: Vec<String>,
        store: Store,
        owner: Option<&str>,
        by: &Principal,
    ) -> Result<Vec<String>> {
        by.require("board:write")?;
        let columns: Vec<String> =
            columns.iter().map(|column| blueprint::expr::slug(column)).filter(|c| !c.is_empty()).collect();
        if columns.is_empty() {
            return Err(Error::invalid("a board needs at least one column"));
        }
        // A board that is there keeps everything its blueprint said of it
        // but its columns: the fields it declared beyond a card's own, the
        // views, and what is kept as a card's history. Declared again as
        // a board is when nothing is said, it would lose them, and what
        // its cards hold of those fields with them.
        let spec = match self.collections.find(project, BOARD).await? {
            Some(board) => Spec {
                fields: board
                    .fields
                    .iter()
                    .cloned()
                    .map(|field| match field.name.as_str() {
                        // Arranged by somebody, the order is theirs: it is kept as the order
                        // they chose, which the board's blueprint applied again does not put back,
                        // and a column they added is kept with it.
                        // What was added here is kept beside it, so that a declaration said
                        // again takes away only what was its own, and never a column added here.
                        COLUMN => Field {
                            of: columns.clone(),
                            of_chosen: columns.clone(),
                            of_added: crate::added::added_to(&field, &columns),
                            ..field
                        },
                        _ => field,
                    })
                    .collect(),
                views: board.views.iter().filter(|view| !view.saved).cloned().collect(),
                history: board.history.clone(),
                store,
                at: None,
                model: board.model.clone(),
                takes: board.takes.clone(),
                reads: None,
                steers: board.steers.clone(),
                shown: board.shown.clone(),
                through: None,
                anew: false,
            },
            None => spec(&columns, store),
        };
        self.collections.ensure(project, BOARD, spec, owner, &Principal::system()).await?;
        let data = json!({ "project": project, "columns": columns });
        let _ = self.journal.emit("board.columns", Source::actor(by.actor()), data).await;
        Ok(columns)
    }

    /// The collection that is a project's board.
    pub(crate) async fn of(&self, project: &str) -> Result<Collection> {
        self.collections.get(project, BOARD).await.map_err(|_| {
            Error::not_found(format!(
                "`{project}` has no board yet. A board is made by a blueprint: apply `std/board` to the project"
            ))
        })
    }

    /// The cards of a board, in the order they were made.
    pub(crate) async fn cards(&self, board: &Collection) -> Result<Vec<Item>> {
        Ok(self.collections.records(board).await?.into_iter().map(item).collect())
    }

    /// Whether a project has a board.
    pub async fn exists(&self, project: &str) -> bool {
        self.collections.get(project, BOARD).await.is_ok()
    }

    /// A project's columns, left to right.
    pub async fn columns(&self, project: &str) -> Result<Vec<String>> {
        Ok(columns_of(&self.of(project).await?))
    }

    /// Changes a board's columns. One that still has cards in it stays.
    pub async fn set_columns(
        &self,
        project: &str,
        columns: Vec<String>,
        by: &Principal,
    ) -> Result<Vec<String>> {
        let board = self.of(project).await?;
        self.make(project, columns, board.store, None, by).await
    }

    /// Adds a card at the bottom of `status` (the first column if not given).
    pub async fn add(
        &self,
        project: &str,
        title: &str,
        body: &str,
        status: Option<&str>,
        labels: Vec<String>,
        by: &Principal,
    ) -> Result<Item> {
        let card = New {
            title: title.to_string(),
            body: body.to_string(),
            status: status.map(str::to_string),
            labels,
            more: Map::new(),
        };
        self.add_new(project, card, by).await
    }

    /// The same, with everything that can be said of a card as it is added.
    pub async fn add_new(&self, project: &str, card: New, by: &Principal) -> Result<Item> {
        self.added(project, card, by, None, None).await
    }

    /// The same, saying which blueprint's step it was: the card's history
    /// then says the blueprint added it, and whose work it was added from
    /// where the blueprint knew that, and not whoever applied the
    /// blueprint, whose authority it was added with all the same.
    pub async fn add_through(
        &self,
        project: &str,
        card: New,
        by: &Principal,
        through: &Through,
    ) -> Result<Item> {
        self.added(project, card, by, Some(through), None).await
    }

    /// The same, for a card somebody added with a key they were given:
    /// the event says that key added it (`said`, `key:<id>`), and not the
    /// account the key is of, which may hold more than the key does.
    /// Whatever then asks what whoever added it holds is answered for the
    /// key.
    pub async fn add_said(&self, project: &str, card: New, by: &Principal, said: &str) -> Result<Item> {
        self.added(project, card, by, None, Some(said)).await
    }

    async fn added(
        &self,
        project: &str,
        card: New,
        by: &Principal,
        through: Option<&Through>,
        said: Option<&str>,
    ) -> Result<Item> {
        let New { title, body, status, labels, more } = card;
        let status = status.as_deref();
        by.require("board:write")?;
        let board = self.of(project).await?;
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::invalid("a card needs a title"));
        }
        let columns = columns_of(&board);
        let status = match status {
            Some(status) => {
                check_status(&columns, status)?;
                status.to_string()
            }
            None => columns.first().cloned().unwrap_or_default(),
        };
        // The bottom of its column, asked of the column alone.
        let position = lowest(self.collections.highest_in(&board, &status, None, None).await?);
        if let Some(own) = more.keys().find(|name| CARD.has(name)) {
            return Err(Error::invalid(own_among(own, "written", "give it by its name")));
        }
        let card = Card { title: title.to_string(), body, status, labels, more, ..Card::default() };
        let created = |record: &Record| {
            let item = item(record.clone());
            let mut data =
                json!({ "id": item.id, "project": item.project, "title": item.title, "status": item.status });
            crate::left::mark(through, &mut data);
            vec![Say::new("board.item.created", told_as(by, through, said, data))]
        };
        Ok(item(self.collections.add_saying(&board, card.data(), position, by, created).await?))
    }

    pub async fn get(&self, id: &str) -> Result<Item> {
        Ok(item(self.card(id).await?.1))
    }

    /// A project's cards, column by column and top to bottom.
    pub async fn list(&self, project: &str) -> Result<Vec<Item>> {
        let board = self.of(project).await?;
        let items = self.cards(&board).await?;
        Ok(in_order(&board, items))
    }

    /// The cards of one column, top to bottom, without the rest of the
    /// board being read: what [`Board::list`] has of that column.
    pub async fn column(&self, project: &str, column: &str) -> Result<Vec<Item>> {
        let board = self.of(project).await?;
        Ok(self.collections.in_column(&board, column).await?.into_iter().map(item).collect())
    }

    /// The cards that say they wait on other cards, in the order of the
    /// board, without the rest of it being read; and none of a board that
    /// keeps nothing of what a card waits for. Whoever goes by what a card
    /// says it waits for ([`crate::cards`]) reads these, and the board
    /// whole only where it does not say.
    pub async fn waiting(&self, project: &str) -> Result<Vec<Item>> {
        let board = self.of(project).await?;
        let items = self.collections.waiting_on_others(&board).await?.into_iter().map(item).collect();
        Ok(in_order(&board, items))
    }

    /// Every card of every board, for indexing. A board whose store is
    /// not answering is left out: one project's database being down must
    /// not be why nothing can be found in the others.
    pub async fn all(&self) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        for collection in self.collections.list(None).await? {
            if collection.name != BOARD {
                continue;
            }
            match self.collections.records(&collection).await {
                Ok(records) => items.extend(records.into_iter().map(item)),
                Err(err) if err.kind == kernel::ErrorKind::Unavailable => {}
                Err(err) => return Err(err),
            }
        }
        Ok(items)
    }

    /// Says what answers whether a card may be given to somebody. Said
    /// once, as the daemon starts; with nothing said a card is given to
    /// anybody, as it always was.
    pub fn gives_with(&self, gives: Gives) {
        let _ = self.gives.set(gives);
    }

    /// Changes a card's content.
    pub async fn update(&self, id: &str, changes: Changes, by: &Principal) -> Result<Item> {
        self.update_through(id, changes, by, None).await
    }

    /// The same, saying which blueprint's step it was when it was one.
    pub async fn update_through(
        &self,
        id: &str,
        changes: Changes,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Item> {
        self.changed(id, changes, by, through, None).await
    }

    /// The same, for a change somebody made with a key they were given:
    /// the event says that key made it, as of a card added with one
    /// (`add_said`).
    pub async fn update_said(&self, id: &str, changes: Changes, by: &Principal, said: &str) -> Result<Item> {
        self.changed(id, changes, by, None, Some(said)).await
    }

    async fn changed(
        &self,
        id: &str,
        changes: Changes,
        by: &Principal,
        through: Option<&Through>,
        said: Option<&str>,
    ) -> Result<Item> {
        by.require("board:write")?;
        let _turn = self.collections.changing(id).await;
        let doing = Doing { by, said_as: by, through, said, also: None, dry: false };
        Ok(self.change(id, changes, doing).await?.expect("a change that is not dry is written"))
    }

    /// A card's change as `doing` says, by whoever holds its turn: every
    /// check asked of `doing.by`, and what is said of it said as
    /// `doing.said_as`, with what `also` says beside and does in the same
    /// write (an approved change, `told::asks`). Dry, nothing is written,
    /// and a change that would be made is none: a card that is as asked
    /// already is the card.
    pub async fn change(&self, id: &str, changes: Changes, doing: Doing<'_>) -> Result<Option<Item>> {
        let Doing { by, said_as, through, said, also, dry } = doing;
        let (board, mut record) = self.card(id).await?;
        let was = item(record.clone());
        let mut card = was.card.clone();
        // What the event says changed: each field by the name it is known by.
        let mut changed: Vec<String> = Vec::new();
        if let Some(title) =
            changes.title.map(|title| title.trim().to_string()).filter(|title| *title != was.title)
        {
            if title.is_empty() {
                return Err(Error::invalid("a card needs a title"));
            }
            card.title = title;
            changed.push("title".into());
        }
        if let Some(body) = changes.body.filter(|body| *body != was.body) {
            card.body = body;
            changed.push("body".into());
        }
        if let Some(assignee) = changes.assignee.filter(|assignee| *assignee != was.assignee) {
            // Asked of the giving alone: a card that names somebody already is changed in its other cells.
            if let (Some(to), Some(gives)) = (&assignee, self.gives.get()) {
                gives(was.project.clone(), to.clone()).await?;
            }
            card.assignee = assignee;
            changed.push("assignee".into());
        }
        if let Some(labels) = changes.labels.filter(|labels| *labels != was.labels) {
            card.labels = labels;
            changed.push("labels".into());
        }
        if let Some(due) = changes.due.filter(|due| *due != was.due) {
            card.due = due;
            changed.push("due".into());
        }
        // What each cell of the board's own held before it was changed.
        let mut held = Map::new();
        for (name, value) in changes.more {
            if CARD.has(&name) {
                let how = if name == COLUMN { "move the card to change it" } else { "change it by its name" };
                return Err(Error::invalid(own_among(&name, "changed", how)));
            }
            // Nothing written and nothing kept are the same nothing.
            let nothing = |value: &Value| value.is_null() || value.as_str().is_some_and(str::is_empty);
            let kept = card.more.get(&name).unwrap_or(&Value::Null);
            if *kept != value && !(nothing(kept) && nothing(&value)) {
                held.insert(name.clone(), kept.clone());
                card.more.insert(name.clone(), value);
                changed.push(name);
            }
        }
        if changed.is_empty() {
            return Ok(Some(was));
        }
        let mut data = card.data();
        crate::fields::stamp(&board, &mut data, &changed, by, false)?;
        let given = changed
            .iter()
            .map(|name| (name.as_str(), data.get(name).unwrap_or(&Value::Null), held.get(name)));
        crate::fields::may_set(&board, &data, given, by)?;
        self.collections.steered(&board, by).await?;
        self.collections.may_name(&board, &data, Some(&held), by).await?;
        if dry {
            return Ok(None);
        }
        record.data = data;
        let (more, also) = also.map_or((Value::Null, None), |(more, also)| (more, Some(also)));
        let updated = |record: &Record| {
            let item = item(record.clone());
            let data = json!({
                "id": item.id, "project": item.project, "title": item.title, "status": item.status,
                "assignee": item.assignee, "changed": changed,
                // What a card asks is its title and its body: whatever plans
                // it or works it hears that those were changed without going
                // through the names.
                "reworded": changed.iter().any(|name| name == "title" || name == "body"),
            });
            let (source, mut data) = told_as(said_as, through, said, data);
            for (key, value) in more.as_object().into_iter().flatten() {
                data[key] = value.clone();
            }
            vec![Say::new("board.item.updated", (source, data))]
        };
        Ok(Some(item(self.collections.put_also(&board, record, updated, also).await?)))
    }

    /// A card and the board it is on.
    pub(crate) async fn card(&self, id: &str) -> Result<(Collection, Record)> {
        let missing = || Error::not_found(format!("there is no card `{id}`"));
        // A board whose store could not be asked has not said the card is
        // not there, and that is passed on as it is.
        let (collection, record) = self.collections.record(id).await.map_err(|err| match err.kind {
            kernel::ErrorKind::NotFound => missing(),
            _ => err,
        })?;
        if collection.name == BOARD { Ok((collection, record)) } else { Err(missing()) }
    }

    /// Moves a card to a column, to the bottom unless `before` names the
    /// card it should sit above.
    pub async fn move_to(
        &self,
        id: &str,
        status: &str,
        before: Option<&str>,
        by: &Principal,
    ) -> Result<Item> {
        self.move_through(id, status, before, None, by, None).await
    }

    /// The same, for a move somebody asked for with a key they were given:
    /// the event says that key did it (`said`, `key:<id>`), and not the
    /// account the key is of, which may hold more than the key does.
    /// Whatever then asks what the mover holds is answered for the key.
    pub async fn move_said(
        &self,
        id: &str,
        status: &str,
        (before, note): (Option<&str>, Option<&str>),
        by: &Principal,
        said: &str,
    ) -> Result<Item> {
        self.moved(id, status, (before, note), by, None, Some(said)).await
    }

    /// The same, with what was said as it was moved (`note`: why a card in
    /// review was sent back, say), and saying which blueprint's step it was
    /// when it was one. The note goes with the event and into the card's
    /// history; it is not kept on the card.
    pub async fn move_through(
        &self,
        id: &str,
        status: &str,
        before: Option<&str>,
        note: Option<&str>,
        by: &Principal,
        through: Option<&Through>,
    ) -> Result<Item> {
        self.moved(id, status, (before, note), by, through, None).await
    }

    async fn moved(
        &self,
        id: &str,
        status: &str,
        (before, note): (Option<&str>, Option<&str>),
        by: &Principal,
        through: Option<&Through>,
        said: Option<&str>,
    ) -> Result<Item> {
        by.require("board:write")?;
        let _turn = self.collections.changing(id).await;
        let (board, mut record) = self.card(id).await?;
        check_status(&columns_of(&board), status)?;
        // A move changes a record, and of a board that steers another
        // table it is asked what any change of one is.
        self.collections.steered(&board, by).await?;
        let mut card = Card::read(std::mem::take(&mut record.data));
        let from = card.status.clone();
        let moves = Moved::read(&card.moved).map_or(0, |moved| moved.count);
        if through.and_then(|through| through.had).is_some_and(|had| had != moves) {
            record.data = card.data();
            return Ok(item(record));
        }
        // Where it goes is asked of its column alone: the card it is to
        // go above, by its id, and the one above that.
        let below = match before.filter(|before| *before != id) {
            Some(before) => self.collections.record_of(&board, before).await?.map(item),
            None => None,
        };
        let highest = |below: Option<f64>| self.collections.highest_in(&board, status, Some(id), below);
        record.position = match below.filter(|below| below.status == status) {
            // Halfway between the card it goes above and the one above that.
            Some(below) => {
                let above = highest(Some(below.position)).await?;
                (above.unwrap_or(below.position - 2.0) + below.position) / 2.0
            }
            None => lowest(highest(None).await?),
        };
        card.status = status.to_string();
        let actor = said.map(str::to_string).unwrap_or_else(|| by.actor());
        // How it came to be there goes on the row in the write that puts
        // it there: whatever reads the row next knows it, whoever has
        // heard of the move by then. A board whose own blueprint keeps
        // something else under that name is left what it keeps.
        if from != status && board.field(MOVED).is_some_and(|field| field.kind == "list") {
            let kind =
                if through.is_some() { "blueprint" } else { actor.split(':').next().unwrap_or_default() };
            let words = note.map(str::trim).unwrap_or_default().to_string();
            card.moved =
                Moved { from: from.clone(), by: kind.to_string(), said: words, count: moves + 1 }.kept();
        }
        record.data = card.data();
        // That it moved is kept in the write that puts it there: stopped
        // anywhere, a card is where it was said to be.
        let moved = |record: &Record| {
            let item = item(record.clone());
            if from == item.status {
                let data = json!({ "id": item.id, "project": item.project, "status": item.status, "changed": ["position"], "reworded": false });
                return vec![Say::new("board.item.updated", told_as(by, through, said, data))];
            }
            let mut data = json!({
                "id": item.id, "project": item.project, "title": item.title,
                "from": from, "to": item.status, "by": actor,
            });
            if let Some(note) = note.map(str::trim).filter(|note| !note.is_empty()) {
                data["note"] = json!(note);
            }
            if through.is_some_and(|through| through.uncounted) {
                data["uncounted"] = json!(true);
            }
            let told = match (through, said) {
                (None, Some(said)) => (Source::actor(said), data),
                _ => told(by, through, data),
            };
            vec![Say::new("board.item.moved", told)]
        };
        Ok(item(self.collections.put_saying(&board, record, moved).await?))
    }

    /// Deletes a card, into the Trash: what it is there, where it is kept.
    pub async fn remove(&self, id: &str, by: &Principal) -> Result<Option<String>> {
        by.require("board:write")?;
        let (board, record) = self.card(id).await?;
        let data = json!({ "id": record.id, "project": record.project });
        let removed = Say::new("board.item.removed", (Source::actor(by.actor()), data));
        Ok(self.collections.trash_saying(&board, &record, vec![removed], by).await?.trash)
    }
}

/// What is said to whoever gave one of a card's own fields among the
/// ones its board declared beyond them.
fn own_among(name: &str, done: &str, how: &str) -> String {
    format!("`{name}` is one of a card's own fields and is not {done} among the others; {how}")
}

/// A board's columns: what the column of a card can be, in order.
fn columns_of(board: &Collection) -> Vec<String> {
    board.field(COLUMN).map(|field| field.of.clone()).unwrap_or_default()
}

fn check_status(columns: &[String], status: &str) -> Result<()> {
    if columns.iter().any(|column| column == status) {
        Ok(())
    } else {
        Err(Error::invalid(format!(
            "`{status}` is not a column of this board; its columns are: {}",
            columns.join(", ")
        )))
    }
}

/// The position below the lowest card of a column, that card's being
/// `highest`: where a card goes that goes to the bottom. Where every
/// card of the column stands above nought, it goes at one.
fn lowest(highest: Option<f64>) -> f64 {
    highest.map_or(0.0, |highest| highest.max(0.0)) + 1.0
}

/// A board's cards column by column and top to bottom, of two at one
/// place the one made first first, as they are handed in that order.
fn in_order(board: &Collection, mut items: Vec<Item>) -> Vec<Item> {
    let columns = columns_of(board);
    let column_of =
        |item: &Item| columns.iter().position(|column| *column == item.status).unwrap_or(usize::MAX);
    items.sort_by(|a, b| column_of(a).cmp(&column_of(b)).then(a.position.total_cmp(&b.position)));
    items
}

/// A card as an action returns it.
pub fn item_value(item: &Item) -> Value {
    serde_json::to_value(item).unwrap_or(Value::Null)
}
