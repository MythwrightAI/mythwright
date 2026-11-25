//! Where a blueprint's text stands among the blueprints that ship, as
//! the daemon knows it: what is handed to the summary as plain values.
//!
//! The words are the language's (`blueprint::summary::forked`). What is
//! the daemon's is what it knows of a text beside what the text says:
//! that it ships; that a project has a fork of it, a file of its own
//! under the name; and, of a fork, what was kept when it was made
//! (`conductor::forked`) and what ships under its name now. A fork an
//! earlier program made has nothing kept, and nothing is made up for
//! it: where a text of the default calls itself the version the fork
//! calls itself (what ships now, or one a running tree is still held
//! to), that text is handed over as the one it is taken to have been
//! made from, and said to be only taken so.
//!
//! It is said only to whoever may read the project's files and the file
//! the fork is: anybody else is handed the summary the text alone comes
//! to, as before.
//!
//! Where a project's own file under a name that ships stands there is
//! worked out from what the runner already knows, and nothing is kept
//! for it ([`own_of`]): a file of the repository of a project that is not
//! trusted is passed over for what ships (`Library::passed_over`), where
//! the factory's place for its own has none; one that is read is in use unless something running there goes on with another text
//! under the name, which is so of a file written outside the factory, or
//! changed there, since that was applied (`texts::vouched`, asked through
//! `Engine::goes_on_otherwise`). It is said first in the summary of the
//! one that ships and of the file, where `mw status` lists what a
//! project has (`GET /api/blueprints/stands`), and in the answer to a save
//! of the file ([`said_of_file`]).
//!
//! Beside that, what one that ships is changed by that its text does not
//! say ([`ways`]): the usual things of which lists it is one, which a row
//! of a project's `segments` lays something before, after or in place
//! of; and how many files what it runs links.
//!
//! It came out of the daemon (`api`), which offers it under the name it
//! always had (`api::forked`): it asked of the daemon only the runner,
//! the projects and where they keep their own blueprints, which it is
//! handed ([`Knowing`]).

use blueprint::summary::forked::{
    Fork, Is, Made, Own, Running, Standing, Usually, Ways, own_sentence, usual,
};
use conductor::Engine;
use conductor::forked::Origin;
use kernel::Principal;
use serde_json::{Value, json};
use studio::Projects;

use crate::library::Own as Kept;

/// What the daemon knows a text's standing by: the runner, the projects,
/// and where each keeps its own blueprints.
#[derive(Clone, Copy)]
pub struct Knowing<'a> {
    pub engine: &'a Engine,
    pub projects: &'a Projects,
    pub own: &'a Kept,
}

/// What the daemon knows of one fork beside its text.
pub struct Known {
    /// The blueprint that ships it is a fork of.
    pub of: String,
    /// What was kept when it was made.
    pub origin: Option<Origin>,
    /// With nothing kept: a text of the default that calls itself what
    /// the fork does.
    taken: Option<String>,
    /// What ships under the name now.
    pub ships: Option<String>,
    /// The file it is, with where it stands in its project.
    pub own: Option<Mine>,
}

impl Known {
    /// The fork whose text is `text`, as plain values.
    pub fn fork<'a>(&'a self, text: &'a str) -> Fork<'a> {
        let made = self.origin.as_ref().map(|origin| Made {
            from: &origin.source,
            by: &origin.by_name,
            at: origin.at,
        });
        let own = self.own.as_ref().map(Mine::own);
        Fork { of: &self.of, text, made, taken: self.taken.as_deref(), ships: self.ships.as_deref(), own }
    }

    /// What was kept when it was made, as it is handed out: everything
    /// but the text, which is the default's to be read as.
    pub fn made(&self) -> Value {
        match &self.origin {
            Some(origin) => json!({
                "blueprint": origin.blueprint, "version": origin.version, "revision": origin.revision,
                "by": origin.by, "by_name": origin.by_name, "at": origin.at,
            }),
            None => Value::Null,
        }
    }
}

/// What is known of `project`'s fork of `name`, whose text is `text`.
pub async fn known(daemon: Knowing<'_>, project: &str, name: &str, text: &str) -> Known {
    let origin = daemon.engine.forked_from(project, name).await;
    let ships = daemon.engine.library().shipped(name, Some(project));
    // Words a blueprint links say no version, so none is taken for them.
    let version = blueprint::version(text).filter(|_| !conductor::library::is_linked(name));
    let taken = match (&origin, version) {
        // A text of the default at the version the fork says: the one
        // there now, or one a running tree reached or was moved from.
        (None, Some(version)) => daemon.engine.text_at(name, &version, None).await.ok(),
        _ => None,
    };
    Known { of: name.to_string(), origin, taken, ships, own: None }
}

/// Where a text stands, held for as long as its summary is being made.
pub enum Stands {
    Ships { own: Option<Mine>, part_of: Vec<String>, ways: Ways },
    Fork(Box<Known>),
    Passed(Mine),
}

impl Stands {
    pub fn standing<'a>(&'a self, text: &'a str) -> Standing<'a> {
        match self {
            Stands::Ships { own, part_of, ways } => {
                Standing::Ships { own: own.as_ref().map(Mine::own), part_of, ways }
            }
            Stands::Fork(known) => Standing::Fork(known.fork(text)),
            Stands::Passed(own) => Standing::Passed(own.own()),
        }
    }
}

/// A project's own file under the name of a text that ships, with where
/// it stands there, as the daemon knows it.
pub struct Mine {
    pub file: String,
    pub is: Where,
}

/// Where such a file stands ([`Is`], held).
pub enum Where {
    InUse,
    NotYet(Vec<Running>),
    PassedOver(String),
}

impl Mine {
    pub fn own(&self) -> Own<'_> {
        let is = match &self.is {
            Where::InUse => Is::InUse,
            Where::NotYet(running) => Is::NotYet { running },
            Where::PassedOver(trust) => Is::PassedOver { trust },
        };
        Own { file: &self.file, is }
    }

    pub fn in_use(&self) -> bool {
        matches!(self.is, Where::InUse)
    }
}

/// `project`'s own file under `name`, which ships, with where it stands
/// there: nothing where it has none, or `who` may not read it.
pub async fn own_of(daemon: Knowing<'_>, who: &Principal, project: &str, name: &str) -> Option<Mine> {
    let projects = daemon.projects;
    if !who.can("projects:read") {
        return None;
    }
    daemon.engine.library().shipped(name, Some(project))?;
    let cli = brand::CLI;
    let named = |named: conductor::library::Named| match named {
        conductor::library::Named::Nothing => None,
        conductor::library::Named::File(file) => Some(file),
        conductor::library::Named::Several(files) => Some(files.join("`, `")),
    };
    let (file, is) = match daemon.engine.library().passed_over(name, Some(project)) {
        // Passed over for what ships, whether it has one file or two.
        Some(_) => (
            named(crate::library::repo_files(daemon.own, name, project))?,
            Where::PassedOver(format!("{cli} project trust {project}")),
        ),
        None => {
            // Of a project that is not trusted only the factory's place is read.
            let file = match projects.is_trusted(project) {
                true => crate::library::own_file(daemon.own, name, project)?,
                false => match crate::library::place_files(daemon.own, name, project) {
                    conductor::library::Named::File(file) => file,
                    _ => return None,
                },
            };
            let running = daemon.engine.goes_on_otherwise(project, name).await;
            let running: Vec<Running> = running
                .into_iter()
                .map(|info| Running {
                    key: info.key,
                    take: format!("{cli} apply --instance {} --take", info.id),
                })
                .collect();
            (file, if running.is_empty() { Where::InUse } else { Where::NotYet(running) })
        }
    };
    let first = file.split("`, `").next().unwrap_or(&file).to_string();
    projects.may_read_path(project, &first, who).await.ok()?;
    Some(Mine { file, is })
}

/// What is said of the file at `path` of `project` once it is saved
/// through the daemon, where it is a project's own under a name that
/// ships: where it stands now, as one sentence, and whether it is in use.
pub async fn said_of_file(daemon: Knowing<'_>, who: &Principal, project: &str, path: &str) -> Option<Value> {
    for name in crate::library::names_at(daemon.own, project, path) {
        if let Some(own) = own_of(daemon, who, project, &name).await.filter(|own| own.file == path) {
            return Some(json!({ "said": own_sentence(&own.own(), &name), "in_use": own.in_use() }));
        }
    }
    None
}

/// The table whose rows lay something into what a stage goes through,
/// as the fleet that ships declares it.
const ROWS: &str = "segments";

/// What else is known of how what ships as `name` is changed, beside
/// its text: each list among what ships of which it is one of the usual
/// things; how many files what it runs links, by what links them; the
/// file a project's own of its name is; and the table and the command
/// that lay something into a list.
pub fn ways(
    daemon: Knowing<'_>,
    name: &str,
    project: Option<&str>,
    listed: &conductor::library::listed::Listed,
) -> Ways {
    let library = daemon.engine.library();
    let read = |name: &str| {
        let text = library.load(name, None)?;
        blueprint::check(&text, &blueprint::NoCatalog).blueprint
    };
    let mut usually = Vec::new();
    for (by, _) in listed.runs.iter().filter(|(_, runs)| runs.iter().any(|one| one == name)) {
        let Some(read) = read(by) else { continue };
        for one in usual(&read).into_iter().filter(|one| one.does.as_deref() == Some(name)) {
            usually.push(Usually { by: by.clone(), order: one.order });
        }
    }
    let mut linking = Vec::new();
    for part in listed.parts(name) {
        let Some(read) = read(&part) else { continue };
        let mut files: Vec<&str> =
            blueprint::parts::linked(&read).into_iter().map(|link| link.from).collect();
        files.sort();
        files.dedup();
        if !files.is_empty() {
            linking.push((part.clone(), files.len()));
        }
    }
    let whose = project.unwrap_or("<project>");
    let command = format!(
        "{} record add {whose} {ROWS} title=<what it does> stage=<the stage, as its row is called> order=<n> \
         does=<a blueprint>",
        brand::CLI
    );
    // Where the project keeps its own, which is the factory's place for
    // them where no project is asked of.
    let kept_in = project.map_or(Some(conductor::library::folders::PLACE.to_string()), |project| {
        daemon.own.blueprints_of(project)
    });
    let keeps_none = match (project, &kept_in) {
        (Some(project), None) => Some(format!("{} project blueprints {project} --usual", brand::CLI)),
        _ => None,
    };
    let own_as = match project {
        Some(project) => crate::library::own_path(daemon.own, name, project),
        None => crate::library::own_path(daemon.own, name, ""),
    };
    Ways { usually, linking, own_as, own_in: kept_in, keeps_none, rows: Some((ROWS.to_string(), command)) }
}

/// What whoever asks for a summary says of the text they send, beside
/// which project it is looked at for.
#[derive(Clone, Copy, Default)]
pub struct Said<'a> {
    /// It is the file at this path of the project.
    pub path: Option<&'a str>,
    /// It is what ships under the name it says.
    pub ships: bool,
}

/// Where the text `source`, which calls itself `name`, stands for `who`:
/// nothing where it is neither one that ships nor a fork of one, where
/// nothing was said of what it is, and where what would be said is not
/// `who`'s to be told.
pub async fn stands(
    daemon: Knowing<'_>,
    who: &Principal,
    project: Option<&str>,
    name: &str,
    source: &str,
    said: Said<'_>,
) -> Option<Stands> {
    // Only what ships has a default to stand beside.
    daemon.engine.library().shipped(name, project)?;
    match (said.path, project) {
        (Some(path), Some(project)) => {
            let own = own_of(daemon, who, project, name).await.filter(|own| own.file == path)?;
            if matches!(own.is, Where::PassedOver(_)) {
                return Some(Stands::Passed(own));
            }
            let mut known = known(daemon, project, name, source).await;
            known.own = Some(own);
            Some(Stands::Fork(Box::new(known)))
        }
        (Some(_), None) => None,
        (None, _) if said.ships => {
            let own = match project {
                Some(project) => own_of(daemon, who, project, name).await,
                None => None,
            };
            // What it is a part of, among the blueprints that are no project's.
            let actions = daemon.engine.registry().all_specs();
            let listed = conductor::library::listed::listed(daemon.engine.library().as_ref(), &actions);
            let ways = ways(daemon, name, project, &listed);
            Some(Stands::Ships { own, part_of: listed.part_of(name), ways })
        }
        (None, _) => None,
    }
}
