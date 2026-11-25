//! Starting a new factory from a snapshot of another.
//!
//! A snapshot (`crate::snapshot`) is put into a data folder that holds no
//! factory yet, and the factory then starts as any does. That is the only
//! way anything of one factory comes to be in another, and it is refused
//! wherever it could land on something that is already there:
//!
//! - in a data folder that holds a factory, or anything else;
//! - without a name for the new factory, or under the name of the factory
//!   the snapshot was made from: two factories on one device that are both
//!   called `prod`, one a copy of the other, is the mistake this is for
//!   avoiding;
//! - from a folder that is not a snapshot, or from one made by a newer
//!   version, whose tables this one would not know what to take out of.
//!
//! What was taken out when the snapshot was made is taken out again here
//! (`crate::snapshot::leave_out`), since the folder may have been changed,
//! or made some other way: nothing arrives signed in, and nothing arrives
//! running. Projects are put in the new factory's own projects folder, and
//! none is left where the snapshot says it was.
//!
//! **The database is put in place last**, written beside where it goes
//! and then moved there, because the database is what makes a data folder
//! a factory. A start that is killed part-way leaves a folder that is not
//! one, with a note in it of which snapshot was arriving, and starting
//! from that snapshot again finishes the work. The snapshot is only read.
//!
//! The new factory remembers what it was started from, and says so once in
//! its own journal (`system.copied`) and to whoever started it.

use std::path::{Path, PathBuf};

use journal::Journal;
use kernel::{Error, Result, Source};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::DATABASE_FILE;
use crate::snapshot::{
    self, ATTACHMENTS, BLUEPRINTS, DATABASE, EXTENSIONS, LeftOut, Manifest, Origin, PROJECTS, Removed, TRASH,
};
use crate::tree::{self, Copier, Tally};

/// The file in a data folder that says a snapshot is arriving there, and
/// which: it is there from when the first file is copied until the
/// database is in place.
pub const MARKER: &str = "arriving.json";
/// Where a factory keeps what it was started from.
pub const REMEMBERED: (&str, &str) = ("factory", "copied");

/// What may be in a data folder that holds no factory and that a snapshot
/// can still arrive in: what a start leaves before it opens the database,
/// and the database of an arrival that did not finish.
fn left_by_a_start() -> Vec<String> {
    let stamp = crate::profile::STAMP_FILE;
    let arriving = arriving_file();
    let journals = snapshot::JOURNALS.to_string();
    vec![
        crate::folder::LOCK.into(),
        crate::folder::SERVING.into(),
        crate::folder::KEEPER.into(),
        stamp.into(),
        format!("{stamp}.new"),
        format!("{arriving}-journal"),
        arriving,
        journals,
    ]
}

/// The database of a factory that is arriving, until it is whole.
fn arriving_file() -> String {
    format!("{DATABASE_FILE}.arriving")
}

/// Whether a data folder holds a factory: it does when its database is there.
pub fn is_factory(data: &Path) -> bool {
    data.join(DATABASE_FILE).is_file()
}

/// A project that is in the new factory without its files.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Empty {
    pub id: String,
    pub name: String,
    /// The empty directory it has in the new factory.
    pub dir: String,
    /// The directory its files were left in, when the snapshot says.
    pub was: Option<String>,
    /// Its tables that are kept as files, and so have nothing in them.
    pub kept_as_files: Vec<String>,
}

/// What a factory was started from, and what it came to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Arrived {
    /// The snapshot's id.
    pub snapshot: String,
    /// When the snapshot was made, in milliseconds since the epoch.
    pub made: i64,
    /// The factory it was made from.
    pub from: Origin,
    /// When this factory was started from it.
    pub arrived: i64,
    /// What this factory's profile is called.
    // Called `environment` by a program before 0.1.0.
    #[serde(alias = "environment")]
    pub profile: String,
    /// What the snapshot says was left out when it was made.
    pub left_out: LeftOut,
    /// What was still in it and was taken out here: nothing, for a snapshot
    /// as it was made.
    pub(crate) removed: Removed,
    /// Projects that are here without their files.
    pub without_files: Vec<Empty>,
    pub projects: usize,
    pub files: usize,
    /// Whether this has been said in the factory's own journal yet.
    #[serde(default)]
    pub(crate) announced: bool,
}

/// A snapshot that may arrive in a data folder.
pub struct Plan {
    from: PathBuf,
    manifest: Manifest,
    profile: String,
    /// An arrival of this same snapshot was begun here and did not finish.
    again: bool,
}

/// Looks at whether a factory on `data`, called `profile`, may be
/// started from the snapshot `from`, before anything is touched. `None`
/// when it was not asked to be.
pub fn check(data: &Path, from: Option<&Path>, profile: Option<&str>) -> Result<Option<Plan>> {
    let cli = brand::CLI;
    // A folder that is a snapshot is not a factory's data, asked for or not.
    if data.join(snapshot::MANIFEST_FILE).is_file() {
        return Err(Error::invalid(format!(
            "{} is a snapshot of a factory, not a factory's data: nothing is started on it. \
             Start a new factory from it with `{cli} up --profile <name> --from {}`",
            data.display(),
            data.display(),
        )));
    }
    let Some(from) = from else { return Ok(None) };
    let Some(profile) = profile.map(str::to_string) else {
        return Err(Error::invalid(format!(
            "`--from` starts a new factory from a copy of another, and the new one needs a name of its own: \
             say which with `--profile`, as in `{cli} up --profile dev2 --from {}`",
            from.display()
        )));
    };
    let manifest = Manifest::read(from)?;
    if manifest.from.profile.as_deref() == Some(profile.as_str()) {
        return Err(Error::invalid(format!(
            "{} is a snapshot of the `{profile}` factory, and the factory started from it cannot be called \
             `{profile}` too: a copy is for trying things apart from the factory it was made from. \
             Give the new one a name of its own, as in `{cli} up --profile {profile}-copy --from {}`",
            from.display(),
            from.display(),
        )));
    }
    snapshot::understood(&manifest.schema, from)?;
    if is_factory(data) {
        return Err(Error::invalid(format!(
            "{} already holds a factory, and a copy is only ever put where there is none: nothing is copied \
             over, or into, a factory that exists. To start the one that is there, leave `--from` out; to start \
             a new one from the snapshot, give it another name (`--profile`) or a data folder of its own (`--data`)",
            data.display()
        )));
    }
    let arriving = std::fs::read_to_string(data.join(MARKER))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|marker| marker["snapshot"].as_str().map(str::to_string));
    let again = match arriving {
        Some(id) if id == manifest.id => true,
        Some(other) => {
            return Err(Error::invalid(format!(
                "{} holds part of another snapshot ({other}), whose arrival did not finish. Finish that one by \
                 starting from that snapshot again, or remove the folder and start from this one",
                data.display()
            )));
        }
        None => false,
    };
    let allowed = left_by_a_start();
    let other = std::fs::read_dir(data)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| !allowed.contains(name));
    if let Some(name) = other.filter(|_| !again) {
        return Err(Error::invalid(format!(
            "{} is not empty (it has `{name}` in it), and a copy is put into a data folder of its own. \
             Give the new factory another name (`--profile`) or another folder (`--data`)",
            data.display()
        )));
    }
    Ok(Some(Plan { from: from.to_path_buf(), manifest, profile, again }))
}

impl Plan {
    /// Puts the snapshot into the data folder `data`, as of `now`, with its
    /// projects under `projects` (the data folder's own where none is said).
    /// It blocks for as long as the copying takes. The folder must be this
    /// start's alone by now.
    pub fn seed(&self, data: &Path, projects: Option<&Path>, now: i64) -> Result<()> {
        let from = &self.from;
        // Looked at again now that no other start can be here: one that
        // was, a moment ago, may have made this a factory.
        if is_factory(data) {
            return Err(Error::invalid(format!(
                "{} already holds a factory, and a copy is only ever put where there is none",
                data.display()
            )));
        }
        let cannot = |path: &Path| {
            let path = path.to_path_buf();
            move |err: std::io::Error| Error::from(err).context(path.display())
        };
        // The database is worked on beside where it will go. Until it is
        // moved there the folder holds no factory.
        let working = data.join(arriving_file());
        for stale in [working.clone(), data.join(format!("{}-journal", arriving_file()))] {
            let _ = std::fs::remove_file(stale);
        }
        std::fs::copy(from.join(DATABASE), &working).map_err(cannot(&working))?;
        tree::private_file(&working)?;
        let connection = journal::copy::open(&working)?;
        let found: std::collections::BTreeMap<String, i64> =
            journal::copy::versions(&connection).map_err(journal::db_error)?.into_iter().collect();
        snapshot::understood(&found, from)?;
        let mut removed = snapshot::leave_out(&connection, now, None)?;
        // The copies of the journals that keep their projects' tables,
        // each by its project, held open until what a scan would find
        // of the project's files is written in them.
        let (mut as_files, mut apart) = (Vec::new(), Vec::new());
        // What is attached to records kept in a journal of a project's own, a file each beside the factory's.
        let mut attached: Vec<String> = Vec::new();
        // The journals of the projects that have one of their own, as
        // the copy lists them: put where the factory will open them,
        // before the folder is a factory. What an arrival that did not
        // finish left there goes first; the folder holds no factory.
        let journals = data.join(snapshot::JOURNALS);
        let _ = std::fs::remove_dir_all(&journals);
        for listed in journal::listed(&connection).map_err(journal::db_error)? {
            let (held, put) = (from.join(snapshot::JOURNALS).join(&listed.file), journals.join(&listed.file));
            if !held.is_file() {
                return Err(Error::invalid(format!(
                    "{} is not whole: it lists a journal of the project `{}` and does not hold it ({})",
                    from.display(),
                    listed.project,
                    held.display()
                )));
            }
            tree::private_dir(&journals)?;
            std::fs::copy(&held, &put).map_err(cannot(&put))?;
            tree::private_file(&put)?;
            let beside = journal::copy::open(&put)?;
            let left = snapshot::leave_out_beside(&beside, now, None)?;
            removed.studio.with(&left.studio);
            removed.stood.turns += left.turns;
            as_files.extend(left.studio.as_files);
            attached.extend(studio::attaching::attached_ids(&beside).map_err(journal::db_error)?);
            match listed.keeps >= journal::keeps::RECORDS {
                true => apart.push((listed.project, beside)),
                false => journal::copy::seal(beside)?,
            }
        }
        let root = projects.map_or_else(|| data.join(PROJECTS), Path::to_path_buf);
        let mut placed = studio::copy::plant(&connection, &root).map_err(journal::db_error)?;
        studio::copy::kept_beside(&mut placed, &as_files);

        // Nothing is put where there is already something: two factories
        // never share a directory, and nobody's files are written over.
        let taken = |dir: &Path| match std::fs::symlink_metadata(dir) {
            Ok(meta) if meta.is_dir() => {
                std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some())
            }
            Ok(_) => true,
            Err(_) => false,
        };
        if let Some(project) = placed.iter().find(|project| !self.again && taken(&project.dir)) {
            drop(connection);
            let _ = std::fs::remove_file(&working);
            return Err(Error::invalid(format!(
                "{} is where the project `{}` would be put, and something is there already. A copy's projects \
                 are put in a projects folder that has none of those names in it; say another with `--projects`",
                project.dir.display(),
                project.id
            )));
        }
        let marker = json!({ "snapshot": self.manifest.id, "from": from });
        std::fs::write(data.join(MARKER), format!("{marker}\n")).map_err(cannot(&data.join(MARKER)))?;

        // What the snapshot holds of files: only read there, and nothing
        // followed out of it.
        let copier = Copier { guarded: &[], stop: &|| false };
        let mut tally = Tally::default();
        let mut without_files = Vec::new();
        for project in &placed {
            std::fs::create_dir_all(&project.dir).map_err(cannot(&project.dir))?;
            let files = snapshot::within(&from.join(PROJECTS), &project.at);
            if project.held && files.is_dir() {
                copier.folder(&files, &project.dir, &project.id, &mut tally)?;
                continue;
            }
            let said = self.manifest.left_out.projects.iter().find(|left| left.id == project.id);
            without_files.push(Empty {
                id: project.id.clone(),
                name: project.name.clone(),
                dir: project.dir.to_string_lossy().into_owned(),
                was: said.map(|left| left.directory.clone()),
                kept_as_files: project.kept_as_files.clone(),
            });
        }
        let kept = data.join(ATTACHMENTS);
        std::fs::create_dir_all(&kept).map_err(cannot(&kept))?;
        attached.extend(studio::copy::attached(&connection).map_err(journal::db_error)?);
        for id in attached {
            let file = from.join(ATTACHMENTS).join(&id);
            if snapshot::is_name(&id) && std::fs::symlink_metadata(&file).is_ok_and(|meta| meta.is_file()) {
                tally.bytes += tree::file(&file, &kept.join(&id))?;
                tally.files += 1;
            }
        }
        copier.folder(&from.join(EXTENSIONS), &data.join(EXTENSIONS), EXTENSIONS, &mut tally)?;
        copier.folder(&from.join(TRASH), &data.join(TRASH), TRASH, &mut tally)?;
        copier.folder(&from.join(BLUEPRINTS), &data.join(BLUEPRINTS), BLUEPRINTS, &mut tally)?;

        // What a scan will compare the projects' files with is what is
        // there now; and the factory is told what it was started from.
        studio::copy::look(&connection, &apart, data).map_err(journal::db_error)?;
        for (_, beside) in apart {
            journal::copy::seal(beside)?;
        }
        let arrived = Arrived {
            snapshot: self.manifest.id.clone(),
            made: self.manifest.made,
            from: self.manifest.from.clone(),
            arrived: now,
            profile: self.profile.clone(),
            left_out: self.manifest.left_out.clone(),
            removed,
            without_files,
            projects: placed.len(),
            files: tally.files,
            announced: false,
        };
        journal::copy::note(&connection, REMEMBERED.0, REMEMBERED.1, &json!(arrived))
            .map_err(journal::db_error)?;
        journal::copy::seal(connection)?;
        // From here the folder is a factory.
        let database = data.join(DATABASE_FILE);
        std::fs::rename(&working, &database).map_err(cannot(&database))?;
        let _ = std::fs::remove_file(data.join(MARKER));
        Ok(())
    }
}

/// Says, once, in a factory's own journal, that it was started from a copy
/// of another; and gives back what there is to tell whoever started it.
/// `None` for a factory that was not, or that has said so before.
pub async fn announce(journal: &Journal) -> Option<Arrived> {
    let kept = journal.kv_get(REMEMBERED.0, REMEMBERED.1).await.ok().flatten()?;
    let arrived: Arrived = serde_json::from_value(kept).ok()?;
    if arrived.announced {
        return None;
    }
    let mut data = json!(arrived);
    data["note"] = json!(arrived.said().first().cloned().unwrap_or_default());
    let _ = journal.emit("system.copied", Source::actor("system"), data).await;
    let said = Arrived { announced: true, ..arrived.clone() };
    let _ = journal.kv_set(REMEMBERED.0, REMEMBERED.1, &json!(said)).await;
    Some(arrived)
}

/// A moment as a day and a time of day, in UTC: `2026-03-01 14:05 UTC`.
pub(crate) fn when(ms: i64) -> String {
    let minutes = ms.div_euclid(60_000);
    let (days, minute) = (minutes.div_euclid(1440), minutes.rem_euclid(1440));
    // Days since the epoch as a date, counted in eras of 400 years from a
    // year that starts in March, so that a leap day is the last of its year.
    let shifted = days + 719_468;
    let (era, day_of_era) = (shifted.div_euclid(146_097), shifted.rem_euclid(146_097));
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
    let month = if month_from_march < 10 { month_from_march + 3 } else { month_from_march - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02} UTC", minute / 60, minute % 60)
}

fn listed(names: &[String]) -> String {
    names.iter().map(|name| format!("`{name}`")).collect::<Vec<_>>().join(", ")
}

impl Arrived {
    /// What there is to tell whoever started the factory: what it is a
    /// copy of, what did not come, and what to do next. A sentence or two
    /// a line.
    pub fn said(&self) -> Vec<String> {
        let cli = format!("{} --profile {}", brand::CLI, self.profile);
        let source = match &self.from.profile {
            Some(name) => format!("the `{name}` factory"),
            None => "a factory with no profile".to_string(),
        };
        let mut said = vec![format!(
            "This factory was started from a copy of {source} on `{}`, made {} (snapshot {}).",
            self.from.device,
            when(self.made),
            self.snapshot
        )];
        said.push(
            "It has that factory's accounts, channels and messages, what was attached to them, tables and boards, \
             projects, agents, extensions and history, as they were then. Nothing that factory has was changed."
                .to_string(),
        );
        said.push(format!(
            "Nobody is signed in here: sign in again with the password you have there (`{cli} login`), or, as \
             its owner, by this machine (`{cli} login --here`). Its sessions, credentials, invitations, devices and link to a hub did not come, nor its measurements, \
             logs or search index, which is being built again."
        ));
        let ended = &self.left_out.ended;
        let hosted = &self.left_out.unhosted;
        // What was running there, said only where there was some: a factory
        // that had nothing running is not told that none of its nothing came.
        let mut was_running = Vec::new();
        if ended.instances > 0 {
            was_running.push(format!(
                "{} that {} live there {} here as stopped",
                count(ended.instances, "instance"),
                if ended.instances == 1 { "was" } else { "were" },
                if ended.instances == 1 { "is" } else { "are" },
            ));
        }
        if hosted.services + hosted.routers > 0 {
            was_running.push(format!(
                "none of its {} and {} came",
                count(hosted.services, "service"),
                count(hosted.routers, "router"),
            ));
        }
        let was_running =
            if was_running.is_empty() { String::new() } else { format!(": {}", was_running.join(", and ")) };
        said.push(format!(
            "Nothing is running here{was_running}. Apply again the blueprints you want running here; \
             `{cli} apply std/board --project <project>` gives a project its board again, and each unfinished \
             card its own blueprint.",
        ));
        if !self.left_out.agents.is_empty() {
            said.push(format!(
                "The agents ({}) are here as they were set up. Each is given a credential of this factory's by \
                 itself, when it next works. What an agent's program is signed in to its model's provider as is \
                 not the factory's: it is whoever runs this daemon, so under the same user as that factory these \
                 agents use the same sign-in, and under another user or on another device the program has to be \
                 signed in there.",
                listed(&self.left_out.agents)
            ));
        }
        if !self.left_out.slots.is_empty() {
            said.push(format!(
                "What {} wrote in {} own folder, working with no project, was left where it is.",
                listed(&self.left_out.slots),
                if self.left_out.slots.len() == 1 { "its" } else { "their" },
            ));
        }
        for project in &self.without_files {
            let was = match &project.was {
                Some(was) => format!(
                    ": they are in {was}, which is the other factory's, and two factories never share a directory"
                ),
                None => String::new(),
            };
            let tables = if project.kept_as_files.is_empty() {
                String::new()
            } else {
                format!(
                    " Its {} kept as files there, and {} nothing in {} until they are.",
                    if project.kept_as_files.len() == 1 {
                        format!("table {} is", listed(&project.kept_as_files))
                    } else {
                        format!("tables {} are", listed(&project.kept_as_files))
                    },
                    if project.kept_as_files.len() == 1 { "has" } else { "have" },
                    if project.kept_as_files.len() == 1 { "it" } else { "them" },
                )
            };
            said.push(format!(
                "The project `{}` came without its files{was}. It has an empty directory here, {}; put its files there.{tables}",
                project.id, project.dir
            ));
        }
        for store in &self.left_out.stores {
            said.push(format!(
                "The table `{name}` of `{project}` was kept by a store service of that factory, and has no store \
                 here until it is given one: host one (`{cli} apply std/store --project {project}`) and say where \
                 it is (for a board, `{cli} apply std/board --project {project} --param store=service --param database=<its url>`).",
                name = store.collection,
                project = store.project,
            ));
        }
        let not_copied = self.left_out.files.len() + self.left_out.more_files;
        if not_copied > 0 {
            said.push(format!(
                "{} left where {} when the snapshot was made (links into that factory, the working directories of \
                 lanes, a repository's list of them); its `{}` says which.",
                if not_copied == 1 { "1 file or folder was".to_string() } else { format!("{not_copied} files or folders were") },
                if not_copied == 1 { "it was" } else { "they were" },
                snapshot::MANIFEST_FILE,
            ));
        }
        let late = &self.removed;
        let still = late.studio.sessions
            + late.studio.credentials
            + late.studio.invites
            + late.ended.instances
            + late.ended.runs
            + late.unhosted.services
            + late.unhosted.routers
            + late.stood.turns;
        if still > 0 {
            said.push(format!(
                "The snapshot was not as a factory makes one: {still} sessions, credentials or running things were \
                 still in it, and were taken out here."
            ));
        }
        said
    }
}

/// `3 instances`, `1 instance`; `what` may be several words, the first of
/// which is what there is one or more of.
fn count(number: usize, what: &str) -> String {
    let (thing, rest) =
        what.split_once(' ').map_or((what, String::new()), |(thing, rest)| (thing, format!(" {rest}")));
    let rest = if number == 1 { rest } else { rest.replace(" was", " were") };
    format!("{number} {thing}{}{rest}", if number == 1 { "" } else { "s" })
}
