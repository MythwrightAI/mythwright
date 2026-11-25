//! The Trash for whoever reads it (`studio::trash` is the writes that
//! move a thing in and out, each one write with what is said of it):
//! what is in it that they could read, the latest first; giving something
//! back, and deleting it for good, which is for whoever could have
//! deleted it; what empties it once a thing has been there as long as the
//! factory keeps what is deleted; and what finishes, as a daemon starts,
//! what one stopped in the middle of with a file.
//!
//! Who sees a thing is whoever could read it as things stand: a project
//! by whoever may read projects, a file by whoever may read its project's
//! files and that file, a card by whoever may read the board, and a row
//! of a table by whoever may read records and, where the table was kept
//! private, its scope, and a channel by whoever reads channels and, where
//! it was kept to its members, the one expression of a kept channel's
//! readers over who was in it as it went. One they could not read answers
//! as one that is not there, and a kept channel's name is said to nobody
//! else. What is of a project that is not there is not listed: it is
//! given back with its project.

mod attached;

use journal::Journal;
use kernel::trash::{Keeping, Whence, goes};
use kernel::{Error, Principal, Result};
use serde_json::json;
use studio::board::BOARD;
use studio::trash::{self, Kind, Trashed};
use studio::{Channels, Collections, Projects, Records, Studio, Workspaces};

/// Takes a folder of the Trash away; what cannot be is left for the next
/// start, which takes away what is marked as going.
fn remove_kept(folder: std::path::PathBuf) {
    let _ = std::fs::remove_dir_all(folder);
}

/// The Trash, for whoever reads it, gives something back or deletes it
/// for good, and for what empties it.
#[derive(Clone)]
pub struct Trash {
    factory: Journal,
    projects: Projects,
    collections: Collections,
    records: Records,
    identity: studio::Identity,
    channels: Channels,
    workspaces: Workspaces,
    /// The factory's secrets, where it keeps any: a project deleted for
    /// good takes its environments' with it.
    secrets: Option<hosting::secrets::Secrets>,
    /// The runner, where there is one: what it holds of a project's
    /// environments for what runs under them goes with the project.
    engine: Option<conductor::Engine>,
    /// What more goes with a project deleted for good that is kept above
    /// the Trash (what runs for it and its agents, which the agents keep),
    /// where whoever opens the Trash says it ([`Trash::with_also`]).
    also: Option<Also>,
    /// What deleting a project for good asks of whoever deletes it beyond
    /// `projects:write`, for what goes with it above the Trash, where
    /// whoever opens the Trash says it ([`Trash::with_also`]).
    asks: Option<Asks>,
}

/// What more is taken away with a project deleted for good, by whoever
/// keeps it: handed the project, and done before its tables go.
pub type Also = std::sync::Arc<dyn Fn(String) -> kernel::BoxFut<'static, Result<()>> + Send + Sync>;

/// Whether somebody may have a project deleted for good, for what goes
/// with it (`Also`): handed the project and who deletes it, and refusing
/// in a sentence where they may not.
pub type Asks =
    std::sync::Arc<dyn Fn(String, Principal) -> kernel::BoxFut<'static, Result<()>> + Send + Sync>;

impl Trash {
    /// The Trash of the factory whose studio this is.
    pub fn of(studio: &Studio, factory: &Journal) -> Trash {
        Trash {
            factory: factory.clone(),
            projects: studio.projects.clone(),
            collections: studio.collections.clone(),
            records: studio.records.clone(),
            identity: studio.identity.clone(),
            channels: studio.channels.clone(),
            workspaces: studio.workspaces.clone(),
            secrets: None,
            engine: None,
            also: None,
            asks: None,
        }
    }

    /// The same, with what more goes with a project deleted for good
    /// (`Also`): what runs for it, and its agents; and what that asks of
    /// whoever deletes it (`Asks`).
    pub fn with_also(mut self, also: Also, asks: Asks) -> Trash {
        self.also = Some(also);
        self.asks = Some(asks);
        self
    }

    /// Whether `by` may have `project` deleted for good, for what goes
    /// with it above the Trash: asked before anything of it is deleted,
    /// by whatever deletes one for good for somebody. The Trash emptying
    /// itself asks nothing: it is the system.
    pub async fn may_take_down(&self, project: &str, by: &Principal) -> Result<()> {
        match &self.asks {
            Some(asks) => asks(project.to_string(), by.clone()).await,
            None => Ok(()),
        }
    }

    /// The same, telling the runner when a project's environments go
    /// with it for good.
    pub fn with_engine(mut self, engine: conductor::Engine) -> Trash {
        self.engine = Some(engine);
        self
    }

    /// The same, taking what a project's environments keep secret away
    /// with a project deleted for good.
    pub fn with_secrets(mut self, secrets: Option<hosting::secrets::Secrets>) -> Trash {
        self.secrets = secrets;
        self
    }

    /// What is kept of `project` beside the runner's rows by the
    /// project's name (`runs::beside::Whose::Project`: its environments,
    /// what each fork of it was forked from, the last text of each name
    /// that checked for it) and the values of its environments' secrets,
    /// taken away now that the project is gone for good: by the Trash
    /// being emptied of it, by its being deleted there, and where the
    /// factory deletes at once, by whoever forgot it. Kept while it is in
    /// the Trash, so that it is given back whole. What its instances kept
    /// goes with its instances.
    pub async fn project_gone(&self, project: &str) -> Result<()> {
        use runs::beside::{KEPT, Whose, project_of};
        let own = self.factory.of(Some(project));
        let journals =
            if own.name() == self.factory.name() { vec![own] } else { vec![own, self.factory.clone()] };
        for journal in journals {
            let mut gone: Vec<(&'static str, String)> = Vec::new();
            for (name, ..) in KEPT.iter().filter(|(_, whose, _)| *whose == Whose::Project) {
                for (key, _) in journal.kv_list(name).await? {
                    if project_of(&key) == Some(project) {
                        gone.push((name, key));
                    }
                }
            }
            if gone.is_empty() {
                continue;
            }
            let written = journal.write(move |writing| {
                gone.iter().try_for_each(|(name, key)| writing.forget(name, key))?;
                Ok(())
            });
            written.await?;
        }
        if let Some(engine) = &self.engine {
            engine.environments().forget(project, None);
        }
        if let Some(secrets) = &self.secrets {
            secrets.forget_project(project)?;
        }
        // And its own blueprints, what runs for it, its agents, the files attached to its
        // records, what of it is in the Trash and its tables, where they are still there: a
        // project of its name made since has them, and they are left alone.
        if self.projects.dir_of(project).is_none() {
            let _ = std::fs::remove_dir_all(self.projects.place_of(project));
            if let Some(also) = &self.also {
                also(project.to_string()).await?;
            }
            self.collections.forget_project_attached(project).await?;
            self.forget_trashed_of(project).await?;
            self.collections.forget_project(project).await?;
        }
        Ok(())
    }

    /// What of `project`'s is in the Trash (its records, its files, what
    /// was attached to its records), deleted for good with it: given back,
    /// it would be given back into a project of its name made since.
    async fn forget_trashed_of(&self, project: &str) -> Result<()> {
        let journals = [self.factory.records_of(Some(project)), self.factory.clone()];
        for (at, journal) in journals.iter().enumerate() {
            if at > 0 && journal.name() == journals[0].name() {
                continue;
            }
            for entry in trash::rows(journal, None).await? {
                let its = !matches!(entry.kind, Kind::Project | Kind::Channel) && entry.project == project;
                if its {
                    Box::pin(self.gone(journal, &entry, &Principal::system(), "project")).await?;
                }
            }
        }
        Ok(())
    }

    /// How long what is deleted is kept.
    pub async fn kept(&self) -> Keeping {
        trash::keeping(&self.factory).await
    }

    /// Says how long what is deleted is kept: so many days, nought for
    /// deleting at once, or for ever with nothing. The owner's to say.
    pub async fn keep_for(&self, days: Option<u32>, by: &Principal) -> Result<Keeping> {
        trash::keep_for(&self.factory, days, by).await
    }

    /// Whether `by` could read what `entry` is of, as things stand.
    fn sees(&self, entry: &Trashed, by: &Principal) -> bool {
        let reads = entry.reads.as_deref().is_none_or(|scope| by.can(scope));
        match entry.kind {
            Kind::Project => by.can("projects:read"),
            Kind::Channel => self.channels.would_read(entry.reads.as_deref(), by),
            // What is of a project that is not there is not listed: it is
            // given back with the project.
            _ if !kernel::place::is_factory(&entry.project)
                && self.projects.dir_of(&entry.project).is_none() =>
            {
                false
            }
            Kind::File => {
                reads
                    && by.can("projects:read")
                    && !self.projects.is_kept_from(&entry.project, &entry.thing, by)
            }
            Kind::Record | Kind::Attachment if entry.table.as_deref() == Some(BOARD) => {
                reads && by.can("board:read")
            }
            Kind::Record | Kind::Attachment => reads && by.can("records:read"),
        }
    }

    /// Whether `by` could have deleted what `entry` is of: who gives it
    /// back, and who deletes it for good.
    async fn could_delete(&self, entry: &Trashed, by: &Principal) -> Result<()> {
        match entry.kind {
            Kind::Project | Kind::File => by.require("projects:write"),
            // As taking one away for good is, which this is.
            Kind::Channel => by.require("admin"),
            Kind::Record | Kind::Attachment if entry.table.as_deref() == Some(BOARD) => {
                by.require("board:write")
            }
            Kind::Record | Kind::Attachment => {
                by.require("records:write")?;
                let table = entry.table.as_deref().unwrap_or_default();
                match self.collections.get(&entry.project, table).await {
                    Ok(collection) => collection.may_change(by),
                    Err(_) => Ok(()),
                }
            }
        }
    }

    /// What is in the Trash that `by` could read, the latest first.
    pub async fn list(&self, by: &Principal) -> Result<Vec<Trashed>> {
        let days = self.kept().await.days;
        let mut found = Vec::new();
        for journal in self.factory.every_records() {
            for mut entry in trash::rows(&journal, None).await? {
                if !self.sees(&entry, by) {
                    continue;
                }
                entry.goes = goes(entry.at, days);
                entry.who = studio::history::who(&self.identity, &kernel::Source::actor(&entry.by)).await;
                entry.may = self.could_delete(&entry, by).await.is_ok();
                found.push(entry);
            }
        }
        found.sort_by(|one, other| other.at.cmp(&one.at).then(one.id.cmp(&other.id)));
        Ok(found)
    }

    /// One thing in the Trash, by its id, with the journal that keeps it,
    /// for somebody who could read it: one they could not answers as one
    /// that is not there.
    async fn find(&self, id: &str, by: &Principal) -> Result<(Journal, Trashed)> {
        for journal in self.factory.every_records() {
            if let Some(entry) = trash::rows(&journal, Some(id.to_string())).await?.pop()
                && (by.kind == kernel::PrincipalKind::System || self.sees(&entry, by))
            {
                return Ok((journal, entry));
            }
        }
        Err(Error::not_found(format!("there is nothing in the Trash called `{id}`")))
    }

    /// Gives something back where it was. A file whose place has
    /// something else in it by now is given back beside it only where
    /// `beside` says so. Gives back what it was, where it is now, and
    /// where that is not where it was, a sentence saying so (a page whose
    /// page above it is gone, given back as a wiki of its own).
    pub async fn restore(
        &self,
        id: &str,
        beside: bool,
        by: &Principal,
    ) -> Result<(Trashed, String, Option<String>)> {
        let (journal, entry) = self.find(id, by).await?;
        self.could_delete(&entry, by).await?;
        let mut said = None;
        let at = match entry.kind {
            Kind::Record => {
                said = self.collections.restore_record(&journal, &entry, by).await?;
                entry.thing.clone()
            }
            Kind::Attachment => {
                let folder = self.projects.trash_folder();
                attached::restore(&self.collections, &journal, &entry, folder, by).await?;
                entry.thing.clone()
            }
            Kind::File => {
                let back =
                    self.projects.restore_file(&journal, &entry, beside, by, &self.collections).await?;
                self.records.file_restored(&entry.project, &back, by).await;
                back
            }
            Kind::Project => {
                self.projects.restore_project(&entry, by).await?;
                entry.thing.clone()
            }
            Kind::Channel => {
                let project = entry.kept["channels"][0]["project"].as_str();
                let there = match project {
                    Some(project) => self.projects.get(project).await.is_ok(),
                    None => true,
                };
                let (row, actor) = (entry.clone(), by.actor());
                let out = move |writing: &mut journal::Writing<'_>| {
                    trash::take_out(writing, &row, "trash.restored", &actor, json!({}))
                };
                let folder = self.projects.trash_folder();
                self.channels.put_back(&entry, there, folder, by, out).await?.id
            }
        };
        Ok((entry, at, said))
    }

    /// Deletes something in the Trash for good.
    pub async fn forget(&self, id: &str, by: &Principal) -> Result<Trashed> {
        let (journal, entry) = self.find(id, by).await?;
        self.could_delete(&entry, by).await?;
        // What goes with it, where it goes: a project made since under its name keeps all of it.
        if entry.kind == Kind::Project && self.projects.dir_of(&entry.thing).is_none() {
            self.may_take_down(&entry.thing, by).await?;
        }
        self.gone(&journal, &entry, by, "asked").await?;
        Ok(entry)
    }

    async fn gone(&self, journal: &Journal, entry: &Trashed, by: &Principal, why: &str) -> Result<()> {
        // What a channel left: what is attached to it, by what the Trash
        // keeps of it, which a listing does not read.
        // And what went in with a record (the pages under a page), which go with it for good.
        let whole = match entry.kind {
            Kind::Channel | Kind::Attachment | Kind::Record => {
                trash::rows(journal, Some(entry.id.clone())).await?.pop()
            }
            _ => None,
        };
        let records: Vec<String> = match &whole {
            Some(whole) if whole.kind == Kind::Record => std::iter::once(whole.thing.clone())
                .chain(
                    whole.kept["with"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|one| one["id"].as_str().map(str::to_string)),
                )
                .collect(),
            _ => Vec::new(),
        };
        // What the keeper of its table's model keeps beside each, in the write that deletes them.
        let forgotten = match (entry.kind, &entry.table) {
            (Kind::Record, Some(table)) => match self.collections.get(&entry.project, table).await {
                Ok(collection) => self.collections.forgotten_with(&collection, records.clone()),
                Err(_) => None,
            },
            _ => None,
        };
        let folder = self.projects.trash_folder().map(|folder| folder.join(&entry.id));
        // Marked first, so that a start that finds the folder with no row
        // takes it away and does not keep it again.
        if let Some(folder) = folder.as_ref().filter(|_| entry.kind == Kind::File) {
            let _ = std::fs::write(folder.join(trash::GOING), b"");
        }
        let (row, actor, why) = (entry.clone(), by.actor(), json!({ "why": why }));
        let written = journal.write(move |writing| {
            if let Some(forgotten) = forgotten {
                forgotten(writing)?;
            }
            trash::take_out(writing, &row, "trash.gone", &actor, why)
        });
        written.await?;
        // What it said of where it keeps its own blueprints went with it, in that write.
        if entry.kind == Kind::Project {
            self.projects.folders().put(&entry.thing, Default::default());
        }
        if let Some(folder) = folder {
            remove_kept(folder);
        }
        match whole {
            Some(whole) if whole.kind == Kind::Channel => self.channels.forget_trashed(&whole).await?,
            // Its file where it was, by a stop before it was moved into the Trash.
            Some(whole) if whole.kind == Kind::Attachment => attached::forget_left(&self.collections, &whole),
            _ => {}
        }
        if entry.kind == Kind::Project {
            self.project_gone(&entry.thing).await?;
        }
        // What was attached to a record, or to the document or mockup a file was, goes with it for good.
        match entry.kind {
            Kind::Record => {
                for record in &records {
                    self.collections.forget_attached(&entry.project, record).await?;
                }
            }
            Kind::File => {
                self.collections.forget_attached_at(&entry.project, &entry.thing).await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Takes a channel away into the Trash, as the owner may: a channel
    /// whose own name or id `channel` is, as `by` finds it. Gives back
    /// the channel, and its id in the Trash; nothing of the second where
    /// the factory deletes at once, which takes it away for good.
    pub async fn delete_channel(
        &self,
        channel: &str,
        by: &Principal,
    ) -> Result<(studio::Channel, Option<String>)> {
        by.require("admin")?;
        if self.kept().await.days == Some(0) {
            return Ok((self.channels.remove(channel, by).await?, None));
        }
        let found = self.channels.may_find(channel, by).await?;
        let found = self.channels.get(&found, channel).await?;
        let workspace = self.workspaces.of_channel(&found.id).await?;
        let place = (self.factory.new_id("trash"), workspace, self.projects.trash_folder());
        let keep = |writing: &mut journal::Writing<'_>, entry: &Trashed| trash::put_in(writing, entry);
        let (gone, entry) = self.channels.trash(&found.id, by, place, keep).await?;
        Ok((gone, Some(entry.id)))
    }

    /// Deletes for good what has been in the Trash for as long as the
    /// factory keeps what is deleted. Gives back how many went.
    pub async fn empty(&self) -> Result<usize> {
        let days = self.kept().await.days;
        let now = self.factory.now_ms();
        let mut gone = 0;
        for journal in self.factory.every_records() {
            for entry in trash::rows(&journal, None).await? {
                if kernel::trash::due(entry.at, days, now) {
                    self.gone(&journal, &entry, &Principal::system(), "days").await?;
                    gone += 1;
                }
            }
        }
        Ok(gone)
    }

    /// Finishes, as a daemon starts, what one stopped in the middle of
    /// with a file: one moved into the Trash whose row was not kept yet
    /// is kept; one marked as going, or never moved, is taken away; and a
    /// row whose file was given back already is taken out.
    pub async fn settle(&self) -> Result<()> {
        let Some(folder) = self.projects.trash_folder() else { return Ok(()) };
        let mut rows = std::collections::HashMap::new();
        for journal in self.factory.every_records() {
            for entry in trash::rows(&journal, None).await? {
                rows.insert(entry.id.clone(), (journal.clone(), entry));
            }
        }
        let names: Vec<String> = std::fs::read_dir(&folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        for name in names {
            let at = folder.join(&name);
            let whence: Option<Whence> = std::fs::read(at.join(trash::WHENCE))
                .ok()
                .and_then(|text| serde_json::from_slice(&text).ok());
            let held = at.join(trash::KEPT).exists() && !at.join(trash::GOING).exists();
            match (rows.remove(&name), whence) {
                (Some(_), _) if held => {}
                // A channel's folder holds what is attached to it, and
                // its row is the whole of what it is: whatever its folder
                // holds is given back, or deleted, with the row.
                (Some((_, entry)), _) if entry.kind == Kind::Channel => {}
                // A project's holds its own blueprints, which are carried
                // back as it is given back; its row is what it is.
                (Some((_, entry)), _) if entry.kind == Kind::Project => {}
                // An attachment's is its file, moved there after its row was kept: given
                // back or deleted for good with the row, from there or from where it was.
                (Some((_, entry)), _) if entry.kind == Kind::Attachment => {}
                (Some((journal, entry)), _) => {
                    // Its file is gone from here: given back, or going.
                    let (row, why) = (entry.clone(), json!({ "why": "settled" }));
                    let topic = if at.join(trash::GOING).exists() { "trash.gone" } else { "trash.restored" };
                    journal
                        .write(move |writing| trash::take_out(writing, &row, topic, "system", why))
                        .await?;
                    let _ = std::fs::remove_dir_all(&at);
                }
                (None, Some(whence)) if held => {
                    let entry = whence.entry();
                    let journal = self.factory.records_of(Some(&whence.project));
                    journal.write(move |writing| trash::put_in(writing, &entry)).await?;
                }
                (None, _) => {
                    let _ = std::fs::remove_dir_all(&at);
                }
            }
        }
        // A row of a file whose folder is not there at all was given back.
        for (_, (journal, entry)) in rows.into_iter().filter(|(_, (_, entry))| entry.kind == Kind::File) {
            let (row, why) = (entry.clone(), json!({ "why": "settled" }));
            journal
                .write(move |writing| trash::take_out(writing, &row, "trash.restored", "system", why))
                .await?;
        }
        Ok(())
    }
}
