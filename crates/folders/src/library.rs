//! A project's own blueprints, as a layer of the library.
//!
//! A project's files are whoever's wrote them. One the factory made is
//! its owner's own; one made of a folder that was already there (a
//! repository somebody else wrote, taken up as it stood) is not, and its
//! files do not get to say what `std/board` means: a blueprint is run
//! with the authority of whoever applies it, and "set up the board" would
//! otherwise run whatever that repository put under the name. So until
//! the factory's owner trusts such a project, a file of its own with the
//! name of a blueprint that ships (a standard one, or a package's) is
//! passed over for the one that ships, and whoever checks or applies is
//! told so. Its blueprints under names of its own are untouched: applying
//! one of those is something somebody chose to do.
//!
//! Where a project's own blueprints are is the project's to say
//! (`conductor::library::folders`): in the factory's place for them, which
//! is in the factory's data folder and is named by paths of the project's
//! that begin `@blueprints/`, unless it says a folder of its repository is
//! read before that, or that it keeps none. A project's fleet's blueprint
//! at its top is read whatever it says, as a project from before is set
//! going by it; where the factory's place has one too, that is the one.
//! A name with a file in the repository's folder and one in the factory's
//! is refused, as two files for one name are. Only a file of the
//! repository is passed over for a project that is not trusted: the
//! factory's place holds only what was written through the factory.
//! Everything here asks where they are through [`Own::blueprints_of`]
//! and [`Own::folder`], and nothing writes a folder's name.

use std::sync::Arc;

use conductor::Library;
use conductor::library::Named;
/// How a path of a project's that names the factory's place for its own begins.
pub use conductor::library::folders::PLACE;
use conductor::library::folders::{Folder, Folders, in_place};
use studio::Projects;

/// The name of a project's fleet blueprint, at its top: read whatever the
/// project says of its folder of blueprints, since that is how its fleet
/// is applied.
const FLEET: &str = "fleet";

/// A factory's projects with where each keeps its own blueprints: what
/// every question of a project's own blueprint is asked of.
#[derive(Clone)]
pub struct Own {
    pub projects: Projects,
    pub folders: Folders,
}

impl Own {
    /// Where a file of `project`'s own is written, as a path of the
    /// project's (the factory's place); nothing where it keeps none.
    pub fn blueprints_of(&self, project: &str) -> Option<String> {
        self.folders.blueprints_of(project)
    }

    /// What `project` said of where its own are.
    pub fn folder(&self, project: &str) -> Folder {
        self.folders.of(project)
    }

    /// Where in the project's repository a file for `name` is looked for,
    /// each as what is put before the name: the top and the folder it
    /// said; its top alone for its fleet's where it said none.
    fn looked_in(&self, project: &str, name: &str) -> Vec<String> {
        match self.folder(project).repo() {
            Some(folder) => vec![String::new(), format!("{folder}/")],
            None if name == FLEET => vec![String::new()],
            None => Vec::new(),
        }
    }

    /// Whether the factory's place is looked in for `name`: for every
    /// name but where the project keeps none, and there for its fleet's.
    fn in_place(&self, project: &str, name: &str) -> bool {
        self.folder(project) != Folder::Nowhere || name == FLEET
    }

    /// `name` without the project's folder of its repository before it,
    /// where it has it there.
    fn short<'a>(&self, project: &str, name: &'a str) -> Option<&'a str> {
        name.strip_prefix(&format!("{}/", self.folder(project).repo()?))
    }
}

/// Looks blueprints up in the project they are asked for from: in the
/// factory's place for its own (`@blueprints/`), and before that, where
/// it said one, at its top and under a folder of its repository, so that
/// a project can shadow `std/slot` with a file of its own. Its fleet's at
/// its top is read whatever it says.
pub struct ProjectLibrary {
    own: Own,
    /// What ships: the standard blueprints and the installed packages'.
    shipped: Arc<dyn Library>,
}

impl ProjectLibrary {
    pub fn new(own: Own, shipped: Arc<dyn Library>) -> Self {
        Self { own, shipped }
    }

    /// The project's own file of its repository for `name` that is passed
    /// over, when it has one that may not be put in place of the one that
    /// ships, and nothing of the factory's place is read in its place.
    fn not_used(&self, name: &str, project: &str) -> Option<String> {
        if self.own.projects.is_trusted(project) || self.shipped.load(name, None).is_none() {
            return None;
        }
        if place_files(&self.own, name, project) != Named::Nothing {
            return None;
        }
        // Passed over whether it has one file or two: what ships is read.
        match repo_files(&self.own, name, project) {
            Named::Nothing => None,
            Named::File(file) => Some(file),
            Named::Several(files) => Some(files.join("`, `")),
        }
    }

    /// What is read for `name`: the factory's place alone where the
    /// repository's is passed over.
    fn read_from(&self, name: &str, project: &str) -> Named {
        match self.own.projects.is_trusted(project) || self.shipped.load(name, None).is_none() {
            true => own_files(&self.own, name, project),
            false => place_files(&self.own, name, project),
        }
    }
}

/// The file that is a project's own blueprint of this name, as a path of
/// the project's: in the factory's place, or at its top or under the
/// folder of its repository it said. A name a project has no file for is
/// one that ships with the product, and so, to whoever asks for the text,
/// is one that has more than one ([`own_files`]).
pub fn own_file(own: &Own, name: &str, project: &str) -> Option<String> {
    match own_files(own, name, project) {
        Named::File(file) => Some(file),
        Named::Nothing | Named::Several(_) => None,
    }
}

/// The files of `project`'s own that its own blueprint `name` links, each
/// as a path of the project's, in the order its text names them: those
/// named in words written out and there; one whose name is worked out as
/// a step runs is found only then, and is not among them.
pub fn own_links(own: &Own, name: &str, project: &str) -> Vec<String> {
    let Some(file) = own_file(own, name, project) else { return Vec::new() };
    let Some(text) = own.projects.file_of(project, &file).and_then(|at| std::fs::read_to_string(at).ok())
    else {
        return Vec::new();
    };
    let Some(read) = blueprint::check(&text, &blueprint::NoCatalog).blueprint else { return Vec::new() };
    let mut links: Vec<String> = Vec::new();
    for link in blueprint::parts::linked(&read) {
        let linked = blueprint::parts::linked_as(name, link.from);
        if let Some(path) = own_file(own, &linked, project).filter(|path| !links.contains(path)) {
            links.push(path);
        }
    }
    links
}

/// Where a copy of what ships as `name` is written in a project, to be
/// read in its place: in the factory's place for its own, a blueprint in a
/// file called as one is, and a file a blueprint links under its own name,
/// which is what it is read by ([`own_files`]). Nothing for a project
/// that keeps no blueprints of its own.
pub fn own_path(own: &Own, name: &str, project: &str) -> Option<String> {
    let folder = own.blueprints_of(project)?;
    Some(match conductor::library::is_linked(name) {
        true => format!("{folder}/{name}"),
        false => format!("{folder}/{}", brand::blueprint_file(name)),
    })
}

/// What there is for `name` under `within` of the project (a path of its,
/// ending in `/`, or nothing for its top), each file as a path of the
/// project's.
fn found(own: &Own, project: &str, name: &str, within: &str) -> Named {
    let root = match in_place(within.trim_end_matches('/')) {
        Some(rest) => own.projects.place_of(project).join(rest),
        None => match own.projects.dir_of(project) {
            Some(dir) => dir.join(within),
            None => return Named::Nothing,
        },
    };
    // A file a blueprint links is called what its file is.
    if conductor::library::is_linked(name) {
        return match root.join(name).is_file() {
            true => Named::File(format!("{within}{name}")),
            false => Named::Nothing,
        };
    }
    match conductor::library::named(&root, name, brand::BLUEPRINT_EXT) {
        Named::Nothing => Named::Nothing,
        Named::File(file) => Named::File(format!("{within}{file}")),
        Named::Several(files) => Named::Several(files.iter().map(|file| format!("{within}{file}")).collect()),
    }
}

/// What the project's repository has for `name`: its top is looked at
/// first, then the folder it said, and where the first of them that has
/// the name has it under more than one of the names a blueprint's file
/// may have, both are given.
pub fn repo_files(own: &Own, name: &str, project: &str) -> Named {
    if !conductor::library::is_safe_name(name) {
        return Named::Nothing;
    }
    for within in own.looked_in(project, name) {
        match found(own, project, name, &within) {
            Named::Nothing => {}
            there => return there,
        }
    }
    Named::Nothing
}

/// What the factory's place for the project's own has for `name`.
pub fn place_files(own: &Own, name: &str, project: &str) -> Named {
    if !conductor::library::is_safe_name(name) || !own.in_place(project, name) {
        return Named::Nothing;
    }
    found(own, project, name, &format!("{PLACE}/"))
}

/// What a project has of its own for this name, each file as a path of
/// the project's: what its repository has and what the factory's place
/// has, both where both have it, since neither is read then; and its
/// fleet's in the factory's place before the one at its top, where it
/// said no folder of its repository.
pub fn own_files(own: &Own, name: &str, project: &str) -> Named {
    let (repo, place) = (repo_files(own, name, project), place_files(own, name, project));
    let files = |named: Named| match named {
        Named::Nothing => Vec::new(),
        Named::File(file) => vec![file],
        Named::Several(files) => files,
    };
    match (repo, place) {
        (repo, Named::Nothing) => repo,
        (Named::Nothing, place) => place,
        (_, place) if own.folder(project).repo().is_none() => place,
        (repo, place) => Named::Several([files(repo), files(place)].concat()),
    }
}

impl Library for ProjectLibrary {
    fn load(&self, name: &str, project: Option<&str>) -> Option<String> {
        let project = project?;
        let Named::File(path) = self.read_from(name, project) else { return None };
        std::fs::read_to_string(self.own.projects.file_of(project, &path)?).ok()
    }

    fn passed_over(&self, name: &str, project: Option<&str>) -> Option<String> {
        let project = project?;
        let file = self.not_used(name, project)?;
        Some(format!(
            "`{project}` has a file of its own for `{name}` (`{file}`), which is not used: the one that \
             ships is. This project was made of a folder that was already there, and its files do not \
             stand in for the blueprints that ship until its factory's owner trusts it: \
             `{cli} project trust {project}`",
            cli = brand::CLI
        ))
    }

    fn names(&self, project: Option<&str>) -> Vec<String> {
        project.map(|id| own_names(&self.own, id)).unwrap_or_default()
    }

    fn refused(&self, name: &str, project: Option<&str>) -> Option<String> {
        let project = project?;
        let Named::Several(files) = self.read_from(name, project) else { return None };
        Some(format!("in `{project}` {}", conductor::library::several(name, &files)))
    }

    fn is_own(&self, name: &str, project: Option<&str>) -> bool {
        self.load(name, project).is_some()
    }

    fn called(&self, name: &str, project: Option<&str>) -> Option<String> {
        let project = project?;
        let Named::File(file) = self.read_from(name, project) else { return None };
        // What a blueprint's file is called is its name and its ending,
        // whichever of the project's folders it is in: as what is wrong
        // in one was always said.
        let short = in_place(&file).or_else(|| self.own.short(project, &file));
        Some(short.unwrap_or(&file).to_string())
    }

    fn own(&self, project: Option<&str>) -> Vec<(String, String)> {
        let Some(project) = project else { return Vec::new() };
        let names = every_own_name(&self.own, project);
        names
            .into_iter()
            .filter_map(|name| self.load(&name, Some(project)).map(|text| (name, text)))
            .collect()
    }

    fn folder(&self, project: Option<&str>) -> Option<Folder> {
        project.map(|project| self.own.folders.of(project))
    }

    /// Nothing here ships: every text this gives is a file of a project's.
    fn shipped(&self, _name: &str, _project: Option<&str>) -> Option<String> {
        None
    }
}

/// How many of a project's files are looked at for its blueprints.
const MOST_FILES: usize = 100_000;

/// Every file under `dir` that is a blueprint, by what it is called from
/// the top of the project: `load` finds a name there first. What begins
/// with a dot is passed by, a repository's own folder and the lanes'
/// worktrees among it.
fn every(dir: &std::path::Path, prefix: &str, out: &mut Vec<String>, seen: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        *seen += 1;
        if *seen > MOST_FILES {
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => every(&entry.path(), &format!("{prefix}{name}/"), out, seen),
            // What a blueprint links is the project's own text too, and is
            // answered for with the rest when somebody applies.
            Ok(_) if conductor::library::is_linked(&format!("{prefix}{name}")) => {
                out.push(format!("{prefix}{name}"));
            }
            Ok(_) => {
                if let Some(stem) = brand::blueprint_stem(&name) {
                    out.push(format!("{prefix}{stem}"));
                }
            }
            _ => {}
        }
    }
}

/// The names a file at `path` of a project may be the blueprint of: in
/// the factory's place, what it is called there; in its repository, what
/// it is called at its top and under the folder of it the project said,
/// and its fleet's alone where it said none.
pub fn names_at(own: &Own, project: &str, path: &str) -> Vec<String> {
    let (placed, path) = match in_place(path) {
        Some(rest) => (true, rest),
        None => (false, path),
    };
    // A file a blueprint links is called what its file is.
    let stem = match conductor::library::is_linked(path) {
        true => path,
        false => match brand::blueprint_stem(path) {
            Some(stem) => stem,
            None => return Vec::new(),
        },
    };
    if placed {
        return vec![stem.to_string()];
    }
    if own.folder(project).repo().is_none() {
        return if stem == FLEET { vec![stem.to_string()] } else { Vec::new() };
    }
    let mut names = vec![stem.to_string()];
    names.extend(own.short(project, stem).map(str::to_string));
    names
}

/// Every name a file of a project's own may be the text of, a blueprint's
/// or one a blueprint links, whether or not each is the one used there:
/// what it is called in the factory's place, and in its repository from
/// its top and without the folder it said before it. A project that keeps
/// none has its fleet's alone, where it has one.
pub fn every_own_name(own: &Own, project: &str) -> Vec<String> {
    names_in(own, project, &own.folder(project))
}

/// [`every_own_name`], where the project said `folder`: what its own are
/// read by for what it said before.
pub fn names_in(own: &Own, project: &str, folder: &Folder) -> Vec<String> {
    let Some(dir) = own.projects.dir_of(project) else { return Vec::new() };
    let mut names = Vec::new();
    if *folder != Folder::Nowhere {
        every(&own.projects.place_of(project), "", &mut names, &mut 0);
    }
    match folder.repo() {
        Some(repo) => {
            let mut there = Vec::new();
            every(&dir, "", &mut there, &mut 0);
            // A folder the walk from the top passes by (one under a folder
            // whose name begins with a dot) is walked by itself.
            every(&dir.join(repo), &format!("{repo}/"), &mut there, &mut 0);
            let within = format!("{repo}/");
            let short = there.iter().filter_map(|name| name.strip_prefix(&within).map(str::to_string));
            names.extend(short.collect::<Vec<_>>());
            names.extend(there);
        }
        None => {
            let fleet = brand::blueprint_files(FLEET).iter().any(|file| dir.join(file).is_file());
            let placed = own.projects.place_of(project);
            if fleet || brand::blueprint_files(FLEET).iter().any(|file| placed.join(file).is_file()) {
                names.push(FLEET.to_string());
            }
            if *folder == Folder::Nowhere {
                names.retain(|name| name == FLEET);
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

/// The names of the blueprints that are files of a project's own, whether
/// or not each is the one used there: those in the factory's place, those
/// at its top and under the folder of its repository it said, and its
/// fleet's alone where it keeps none.
pub fn own_names(own: &Own, project: &str) -> Vec<String> {
    let Some(dir) = own.projects.dir_of(project) else { return Vec::new() };
    let folder = own.folder(project);
    let (placed, ext) = (own.projects.place_of(project), brand::BLUEPRINT_EXT);
    let mut names = conductor::DirLibrary::new(&placed, ext).names(None);
    if let Some(repo) = folder.repo() {
        names.extend(conductor::DirLibrary::new(dir.join(repo), ext).names(None));
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(stem) =
                brand::blueprint_stem(&name).filter(|stem| folder.repo().is_some() || *stem == FLEET)
            {
                names.push(stem.to_string());
            }
        }
    }
    if folder == Folder::Nowhere {
        names.retain(|name| name == FLEET);
    }
    names.sort();
    names.dedup();
    names
}

/// Settles, once, what a factory from before vouched for of the folder a
/// project that said nothing read its own from (`blueprints/`, and its
/// top): from this program on no file of its repository is its own, and
/// what running instances were vouched for under those names would
/// otherwise be run as it was then, each with a note that it is no
/// longer the project's own. A checkout of this program's source held
/// every blueprint that ships there, and was taken to run a fork of each.
/// So the names are vouched for again as the system, which leaves what is
/// there now: what ships, or what the factory keeps for the project; and
/// what each instance of the project says of having gone on with another
/// text under one of them (that the file was passed over, or is no longer
/// as it was vouched for) is taken back there and then, which its next
/// call would only have done later. Done after the conductors have started, and kept as done where the
/// setting is (`folders::SETTLED`), so that a project that says nothing
/// later is never taken back from what was vouched for.
pub async fn settle(own: &Own, engine: &conductor::Engine, journal: &journal::Journal) {
    use conductor::library::folders::{NAMESPACE, SETTLED, USUAL};
    if journal.kv_get(NAMESPACE, SETTLED).await.ok().flatten().is_some() {
        return;
    }
    let library = engine.library();
    for project in own.projects.list().await.unwrap_or_default() {
        if own.folder(&project.id) != Folder::Usual {
            continue;
        }
        let before = names_in(own, &project.id, &Folder::In(USUAL.to_string()));
        for name in before.iter().filter(|name| !library.is_own(name, Some(&project.id))) {
            engine.vouch_saved(&project.id, name, &kernel::Principal::system()).await;
            engine.mend_all(&project.id, name).await;
        }
    }
    let _ = journal.kv_set(NAMESPACE, SETTLED, &serde_json::json!(true)).await;
}
