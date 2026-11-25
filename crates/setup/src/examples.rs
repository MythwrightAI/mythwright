//! The examples that came with the program, and one put into a project.
//!
//! An example is a folder of blueprints and what they link, written to be
//! read and tried. The repository keeps them under `examples/`, where a
//! test runs each; but a program somebody unpacked has no repository
//! beside it, and where a project's folder is on the device is the
//! daemon's to know, not something to look up and copy into by hand. So
//! the ones worth trying on a first day are built into the program, as
//! the standard blueprints are, and one is put into a project when asked
//! for: its files written among the project's own blueprints, as whoever
//! asked, who has then vouched for them as for any file they saved.
//!
//! From there the example is the project's own: read, changed and applied
//! like anything written there. Nothing here knows what an example is
//! for.

use conductor::Engine;
use conductor::library::Named;
use folders::library::Own;
use kernel::{Error, Principal, Result};
use serde_json::{Value, json};
use studio::Studio;

/// An example that came with the program.
pub struct Offered {
    /// What it is called: the folder it is in the repository's `examples`,
    /// and the folder it is put in among a project's blueprints.
    pub name: &'static str,
    /// What it is, in a sentence.
    pub says: &'static str,
    /// Its files, each by where it is within the example, and its text.
    pub files: &'static [(&'static str, &'static str)],
}

/// The examples built into the program. One more is a folder of
/// `examples/` and an entry here with every file of it: a test holds each
/// entry to the files its folder has.
pub const OFFERED: &[Offered] = &[Offered {
    name: "research",
    says: "Questions put to an agent of a project, each answer kept as a document; with the words the agent \
           is told in a file, and other words beside it to compare them with.",
    files: &[
        ("answer.bp", include_str!("../../../examples/research/answer.bp")),
        ("ask.bp", include_str!("../../../examples/research/ask.bp")),
        ("prompts/answer.md", include_str!("../../../examples/research/prompts/answer.md")),
        ("prompts/careful.md", include_str!("../../../examples/research/prompts/careful.md")),
    ],
}];

impl Offered {
    /// Where one of its files goes in a project, among the project's own
    /// blueprints (`folder`).
    fn path(&self, folder: &str, file: &str) -> String {
        format!("{folder}/{}/{file}", self.name)
    }

    /// The blueprints it brings, by the names they are applied by.
    pub fn blueprints(&self) -> Vec<String> {
        self.files.iter().filter_map(|(file, _)| self.blueprint(file)).collect()
    }

    /// What a file of it is called as a blueprint, where it is one.
    fn blueprint(&self, file: &str) -> Option<String> {
        brand::blueprint_stem(&format!("{}/{file}", self.name)).map(str::to_string)
    }

    /// As it is listed: its name, what it is, its files and its blueprints.
    pub fn listed(&self) -> Value {
        let files: Vec<&str> = self.files.iter().map(|(file, _)| *file).collect();
        json!({ "name": self.name, "says": self.says, "files": files, "blueprints": self.blueprints() })
    }
}

/// The example of that name, or which there are.
pub fn offered(name: &str) -> Result<&'static Offered> {
    OFFERED.iter().find(|one| one.name == name).ok_or_else(|| {
        let names: Vec<&str> = OFFERED.iter().map(|one| one.name).collect();
        Error::not_found(format!(
            "no example called `{name}` came with this program; there is {}. `{} example list` says what each is",
            names.join(", "),
            brand::CLI
        ))
    })
}

/// Puts an example among a project's own blueprints, as `who`.
///
/// It is the same asked twice: a file that is there and says what the
/// example says is left. A file that is there and says something else is
/// somebody's (it was written before, or changed since the example was
/// put there), and then nothing at all is written: half an example over
/// somebody's words is worse than none.
pub async fn put(
    engine: &Engine,
    studio: &Studio,
    own: &Own,
    project: &str,
    name: &str,
    who: &Principal,
) -> Result<Value> {
    // Before anything is said of the project or of what it holds.
    who.require("projects:write")?;
    let example = offered(name)?;
    let projects = &studio.projects;
    // Among the project's own blueprints, which it may keep none of.
    let Some(folder) = own.blueprints_of(project) else {
        return Err(Error::conflict(format!(
            "`{project}` keeps no blueprints of its own, so the example `{name}` has nowhere to go there. \
             Nothing was written. To have it keep them, in the factory's place for them: `{cli} project \
             blueprints {project} --usual`",
            cli = brand::CLI
        )));
    };
    let mut wanted = Vec::new();
    let mut there = Vec::new();
    for (file, text) in example.files {
        let path = example.path(&folder, file);
        projects.may_read_path(project, &path, who).await?;
        // A blueprint may be a file of the project's under another of the
        // names a blueprint's file may have, or at its top.
        let library_name = example.blueprint(file).unwrap_or_else(|| format!("{}/{file}", example.name));
        let has = match folders::library::own_files(own, &library_name, project) {
            Named::Nothing => None,
            Named::File(found) => Some(found),
            Named::Several(files) => {
                return Err(Error::conflict(format!(
                    "in `{project}` {}. Nothing was written",
                    conductor::library::several(&library_name, &files)
                )));
            }
        };
        match has {
            None => wanted.push((path, *text)),
            Some(found) => {
                projects.may_read_path(project, &found, who).await?;
                if projects.read(project, &found).await? != *text {
                    return Err(Error::conflict(format!(
                        "`{project}` already has `{found}`, and it does not say what the example `{name}` says \
                         there: somebody wrote it, or changed it since the example was put there. Nothing was \
                         written. What is there is the project's own and can be gone on with; to have the \
                         example as it came, take that file away and ask again"
                    )));
                }
                there.push(found);
            }
        }
    }
    let mut written = Vec::new();
    for (path, text) in wanted {
        let put = projects.write(project, &path, text, who).await?;
        // Whoever puts it there has vouched for it as it now is, as for
        // any blueprint of a project's they save through the daemon.
        for one in folders::library::names_at(own, project, &put.path) {
            engine.vouch_saved(project, &one, who).await;
        }
        written.push(put.path);
    }
    Ok(json!({
        "example": example.name,
        "project": project,
        "folder": format!("{folder}/{}", example.name),
        "written": written,
        "there": there,
        "blueprints": example.blueprints(),
    }))
}
