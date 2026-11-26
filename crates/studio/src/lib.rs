//! What people and agents work in.
//!
//! Accounts and credentials, projects and their files, channels, the board,
//! the records of whatever else a blueprint declared, and search. Each is a small service over the journal: what it stores is
//! its own, what it does is an event, and what a blueprint may do with it is
//! an action registered here.

mod actions;
pub mod added;
pub mod apart;
pub mod attaching;
pub mod board;
pub mod cards;
pub mod change;
mod closed;
pub mod collections;
mod column;
mod commented;
pub mod copy;
mod counted;
mod declared;
pub mod documents;
pub mod done;
pub mod events;
pub mod factory;
mod fields;
pub mod fieldsaid;
pub mod first;
pub mod forgotten;
mod held;
pub mod history;
pub mod icons;
mod indexed;
pub mod indexes;
mod left;
pub mod mockups;
pub mod models;
pub mod moved;
pub mod named;
// How a page is made came out of this crate with the library written
// out as pages and a document and a mockup as files (`pages`), and is
// offered under the name it had here.
pub use ::pages::made as pages;
pub mod pressed;
mod questioned;
pub mod records;
mod replied;
mod reworded;
pub mod rows;
pub mod said;
// Projects and their files came out of this crate (`projects`), and so
// did the search index (`search`) and the workspaces (`workspaces`): each
// is offered under the name it had here.
pub use ::projects;
pub use ::search;
mod service;
pub mod shown;
pub mod site;
mod stood;
mod tables;
mod takes;
mod through;
mod titled;
pub mod trash;
mod unsaid;
mod views;
pub mod whole;
pub mod window;
mod within;
pub use ::workspaces;

use std::path::PathBuf;
use std::sync::Arc;

use conductor::{Engine, Registry};
use journal::{Db, Journal};
use kernel::Result;

pub use attaching::Attached;
pub use board::{Board, Changes, Doing, Item};
pub use collections::{Collection, Collections, Record, Store, View};
pub use identity::{Account, Came, Credential, CredentialKind, Identity, SignedIn, Tier};
pub use models::Card;
pub use projects::{Change, Entry, Holdings, Project, Projects};
pub use records::{Checked, Checking, Records};
pub use search::{Hit, Progress, Search};
pub use trash::Trashed;
pub use workspaces::{Belongs, Settled, Workspace, Workspaces};

// Who is who came out of this crate (`people`), and where people and
// agents talk (`talk`): each is still offered under the names it had here.
pub use people::{
    Appearance, Arranged, Clocked, Themes, appearance, arranged, clocked, enrolled, groups, handle, identity,
    invited, themes,
};
pub use talk::{
    Attachment, Attachments, Channel, Channels, Member, Message, Place, Posted, Reader, SaidBy, Speaker,
};
pub use talk::{
    Conversing, Inbox, answered, archived, attachments, channels, commanded, conversed, discussed, docs,
    door, inbox, inboxed, managed, renamed, speaker, taken, telling,
};

#[derive(Clone)]
pub struct Studio {
    pub identity: Identity,
    pub themes: Themes,
    pub arranged: Arranged,
    pub clocked: Clocked,
    pub appearance: Appearance,
    pub projects: Projects,
    pub channels: Channels,
    pub collections: Collections,
    pub board: Board,
    pub records: Records,
    pub search: Search,
    pub workspaces: Workspaces,
    journal: Journal,
    /// What keeps the histories of records ([`Collections::keep_history`]).
    keeper: Arc<tokio::task::AbortHandle>,
    /// The records, held here for what answers for a line of something
    /// said on one, which the channels' inbox holds only weakly
    /// ([`named::OnRecords`]): held there, the journal held them back.
    _records: Arc<Collections>,
}

impl Studio {
    /// `search_db` is a database of its own, so indexing has its own thread.
    /// `projects_root` is where new projects are created, and `attached` is
    /// where what is attached to messages is kept.
    pub async fn open(
        journal: Journal,
        search_db: Db,
        projects_root: impl Into<PathBuf>,
        attached: impl Into<PathBuf>,
    ) -> Result<Studio> {
        let identity = Identity::open(journal.clone()).await?;
        let themes = Themes::open(journal.clone()).await?;
        let arranged = Arranged::open(journal.clone()).await?;
        let clocked = Clocked::open(journal.clone()).await?;
        let appearance = Appearance::open(journal.clone()).await?;
        let projects = Projects::open(journal.clone(), projects_root).await?;
        // A project's tables, and what a scan last saw of its files, are
        // kept in its own journal where that keeps them, and such a
        // journal is made with the tables for them: said here, before
        // any project can be added.
        apart::makes(&journal).await?;
        let attached: PathBuf = attached.into();
        let attachments = Attachments::open(journal.clone(), attached.clone()).await?;
        let channels = Channels::open(journal.clone(), attachments).await?;
        // Who a word after an `@` names among the accounts, for what a
        // message puts in the box of somebody it names.
        channels.names_with(Arc::new(people::named));
        channels.inbox.holds_with(Arc::new(people::held));
        let collections = Collections::open(journal.clone(), projects.clone()).await?;
        let _ = collections.attached.set(attached);
        // From here on, what happens to a record that its collection counts as its history is written down.
        // What is said on a record that names somebody is told in their box:
        // what reading each record takes is of the tables as the keeper
        // reads them, filled before anything is asked of it.
        let tables = Arc::new(std::sync::RwLock::new(collections.list(None).await.unwrap_or_default()));
        let held = Arc::new(collections.clone());
        let on_records = named::OnRecords { collections: Arc::downgrade(&held), declared: tables.clone() };
        channels.inbox.records_with(Arc::new(on_records));
        let keeper = Arc::new(collections.keep_history(identity.clone(), channels.clone(), tables));
        let board = Board::open(journal.clone(), collections.clone()).await?;
        // Which cards a column holds, by its index, for a told line's part to keep.
        let reading = board.reading();
        channels.columns_with(Arc::new(move |project, column| {
            let board = reading.clone();
            Box::pin(async move {
                Ok(board.column(&project, &column).await?.into_iter().map(|card| card.id).collect())
            })
        }));
        let records = Records::new(journal.clone(), collections.clone());
        let indexed = Arc::new(indexed::Indexed { projects: projects.clone(), board: board.clone() });
        let search = Search::open(search_db, journal.clone(), channels.clone(), indexed).await?;
        let workspaces = Workspaces::open(journal.clone(), channels.clone()).await?;
        // From here on a project is made with its place, and is forgotten
        // with where its channels go.
        projects.placed().makes_with(Arc::new(::workspaces::place_new));
        projects.placed().forgets_with(Arc::new(::workspaces::forget_project));
        Ok(Studio {
            identity,
            themes,
            arranged,
            clocked,
            appearance,
            projects,
            channels,
            collections,
            board,
            records,
            search,
            workspaces,
            journal,
            keeper,
            _records: held,
        })
    }

    /// Ends what the studio keeps doing by itself, the histories of
    /// records, as whatever opened it is let go of: it holds what it
    /// follows, so it would otherwise outlive the studio.
    pub fn halt(&self) {
        self.keeper.abort();
    }

    /// Brings about that the factory has a workspace and that every
    /// project is in one ([`Workspaces::settle`]): the first workspace is
    /// called after the factory's owner, and where nobody owns it yet
    /// none is made. Asked as the owner registers and as a daemon starts.
    pub async fn settle(&self) -> Result<Settled> {
        let accounts = self.identity.accounts().await?;
        let owner = accounts.iter().find(|account| account.role == "owner");
        self.workspaces.settle(owner.map(|owner| owner.name.as_str())).await
    }

    /// Adds this crate's actions to `registry`.
    pub fn register(&self, registry: &mut Registry) {
        actions::register(registry, self);
        registry.emits(events::events());
    }

    /// Tells the engine where projects live, so steps run in the right place.
    pub fn attach(&self, engine: &Engine) {
        let projects = self.projects.clone();
        engine.set_workdirs(Arc::new(move |project| project.and_then(|id| projects.dir_of(id))));
        self.collections.said_by.attach(engine);
    }

    /// Looks for file changes whenever a run that belongs to a project ends.
    /// A run is when files change behind our back (a build, an agent), so
    /// that is when to look; nothing is watched or polled in between.
    pub fn watch_runs(&self, engine: &Engine) -> tokio::task::JoinHandle<()> {
        // A run is its project's, and is said to have ended in its
        // project's journal: every journal is followed.
        self.watch_runs_from(engine, self.journal.follow_every(self.journal.now(), "run.finished"))
    }

    /// The same, from what `follow` has got to.
    ///
    /// That a run ended is passed on as it is said and kept only where a
    /// text hears it (`conductor::owed`), so where the follower falls too
    /// far behind what is said and goes back to the journal, what ended
    /// meanwhile is not there: every project is looked at instead, which
    /// finds whatever any run wrote before the look.
    pub fn watch_runs_from(
        &self,
        engine: &Engine,
        mut follow: journal::FollowEvery,
    ) -> tokio::task::JoinHandle<()> {
        let (studio, engine) = (self.clone(), engine.clone());
        tokio::spawn(async move {
            while let Ok(heard) = follow.next_or_behind().await {
                let Some(event) = heard else {
                    for project in studio.projects.list().await.unwrap_or_default() {
                        let _ = studio.projects.scan(&project.id).await;
                    }
                    continue;
                };
                let Some(instance) = &event.source.instance else { continue };
                if let Ok(info) = engine.instance(instance).await
                    && let Some(project) = info.project
                {
                    let _ = studio.projects.scan(&project).await;
                }
            }
        })
    }
}
