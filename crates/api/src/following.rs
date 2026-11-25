//! Texts that are followed: what tells the conductor that one changed.
//!
//! How what is running comes by new texts is the conductor's
//! (`conductor::taking`), and where the factory's owner said they are
//! followed something has to say when a text changed. A project's files
//! are already followed, and say so (`project.file.changed`), and so is
//! which folder of a project holds its own (`project.blueprints`); the
//! blueprints of a checkout the program is run from are followed here,
//! with the node's one watch, unless texts were said to be held.
//! Nothing is looked for on a timer.
//!
//! A text that does not check is not taken, and what goes on without it
//! says so once. Where the tree it is of reports in a channel, that is
//! said there too, by the factory's bot.

use std::path::{Path, PathBuf};

use conductor::library::folders::Folder;
use conductor::taking::NotTaken;

use crate::state::App;

/// How many folders of a checkout's blueprints are followed.
const MOST_FOLDERS: usize = 200;

/// A folder and every folder under it: the system is asked of each.
fn folders(root: &Path) -> Vec<PathBuf> {
    let mut found = vec![root.to_path_buf()];
    let mut at = 0;
    while at < found.len() && found.len() < MOST_FOLDERS {
        let inside = std::fs::read_dir(&found[at]).into_iter().flatten().flatten();
        let under: Vec<PathBuf> = inside.map(|entry| entry.path()).filter(|path| path.is_dir()).collect();
        found.extend(under);
        at += 1;
    }
    found
}

/// The channel the tree of `instance` reports in: what it, or the nearest
/// above it that says, names as `channel`; and who that one runs as,
/// which is by whose leave anything is said there for it.
async fn channel_of(app: &App, instance: &str) -> Option<(String, kernel::Principal)> {
    let mut at = Some(instance.to_string());
    for _ in 0..64 {
        let one = app.engine.instance(&at.take()?).await.ok()?;
        let said = [one.params.get("channel"), one.state.get("channel")];
        let named = said.into_iter().flatten().filter_map(|said| said.as_str()).find(|said| !said.is_empty());
        if let Some(named) = named {
            return Some((named.trim_start_matches('#').to_string(), one.principal));
        }
        at = one.parent;
    }
    None
}

/// Says in the channel where each tree reports that a text was not taken.
pub async fn told(app: &App, left: Vec<NotTaken>) {
    for one in left {
        let Some((channel, runs_as)) = channel_of(app, &one.instance).await else { continue };
        let body = format!(
            "`{}` was changed and does not check, so `{}` goes on with the text it has until it does.\n{}",
            one.blueprint, one.key, one.error
        );
        // Said by leave of whoever the tree runs as: in an open channel
        // as it always was, and in one kept to its members only where
        // they are one of them and may write there. Naming a channel is
        // not a way to have the factory speak in it.
        let Ok(reader) = app.studio.channels.may_find(&channel, &runs_as).await else { continue };
        let _ = app.studio.channels.post_speaker(&reader, &channel, &body, studio::Place::default()).await;
    }
}

/// What is heard of a project's files and the setting: none a channel's,
/// and nothing of it handed on.
const HEARD: &str = "project.file.changed,project.rescanned,project.blueprints,system.texts";

/// Runs for as long as the node does: tells the conductor of each text
/// that changed, where texts are followed.
pub async fn follow(app: App, checkout: Option<PathBuf>) {
    let files = app.hosting.watch();
    let mut said = app.journal.follow_every(app.journal.now(), HEARD);
    // First, for whatever changed while nothing was listening.
    told(&app, app.engine.follow(None, None).await).await;
    loop {
        let followed: Vec<PathBuf> = match (&checkout, app.engine.brings()) {
            (Some(dir), true) => folders(dir),
            _ => Vec::new(),
        };
        let named: Vec<&Path> = followed.iter().map(PathBuf::as_path).collect();
        let mut changes = files.within(&named);
        let watching = !named.is_empty();
        tokio::select! {
            () = changes.next(), if watching => told(&app, app.engine.follow(None, None).await).await,
            event = said.next() => {
                let Ok(event) = event else { return };
                let project = event.data["project"].as_str();
                match event.topic.as_str() {
                    "project.file.changed" => {
                        let path = event.data["path"].as_str().unwrap_or_default();
                        for name in crate::library::names_at(&app.own, project.unwrap_or_default(), path) {
                            told(&app, app.engine.follow(project, Some(&name)).await).await;
                        }
                    }
                    "project.rescanned" => told(&app, app.engine.follow(project, None).await).await,
                    // Its own are read from another folder now, or none: as
                    // though those there were taken away and these saved.
                    "project.blueprints" => {
                        let project = project.unwrap_or_default();
                        let was = match (event.data["was_said"].as_str(), event.data["was"].as_str()) {
                            (Some("none"), _) => Folder::Nowhere,
                            (_, Some(folder)) => Folder::In(folder.to_string()),
                            _ => Folder::Usual,
                        };
                        let were = crate::library::names_in(&app.own, project, &was);
                        told(&app, app.engine.follow_folder(project, &were).await).await;
                    }
                    // What is followed may be other folders now.
                    _ => {}
                }
            }
        }
    }
}
