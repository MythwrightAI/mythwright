//! How a project's blueprints are laid out, and where it says so.
//!
//! Laying a blueprint out is [`blueprint::format`], which reads no files.
//! This is the part that does: a project may say a few things of its own
//! in a file in its repository ([`blueprint::format::SETTINGS_FILE`]), and
//! that file is looked for in the blueprint's own folder and then in each
//! folder above it, the nearest one found being the one that counts. The
//! editor asks the daemon and the command line asks nobody, and both come
//! here, so that a text is laid out the same whichever wrote it last.

use std::path::Path;

use blueprint::Settings;
use blueprint::format::SETTINGS_FILE;
use kernel::whole;

/// The settings for a blueprint in `folder`. `top` is the last folder
/// looked in: a project's own, for a blueprint that is a project's file.
/// With no `top` the looking stops at the top of the repository the folder
/// is in, or where the folders end.
pub fn settings_for(folder: &Path, top: Option<&Path>) -> Result<Settings, String> {
    let folder = whole::whole(folder).unwrap_or_else(|_| folder.to_path_buf());
    let top = top.map(|top| whole::whole(top).unwrap_or_else(|_| top.to_path_buf()));
    let mut at = Some(folder.as_path());
    while let Some(dir) = at {
        let file = dir.join(SETTINGS_FILE);
        if file.is_file() {
            let text = std::fs::read_to_string(&file)
                .map_err(|err| format!("{} could not be read: {err}", file.display()))?;
            return Settings::read(&text).map_err(|said| format!("{}: {said}", file.display()));
        }
        let last = match &top {
            Some(top) => whole::same(dir, top),
            None => dir.join(".git").exists(),
        };
        at = if last { None } else { dir.parent() };
    }
    Ok(Settings::default())
}

/// Whether a file of settings is in the folder `file` is in or one above
/// it, as far as `top`: for a blueprint the factory keeps for a project,
/// whose settings are otherwise the project's top's.
pub fn said_within(file: &Path, top: &Path) -> bool {
    let mut at = file.parent();
    while let Some(dir) = at.filter(|dir| dir.starts_with(top)) {
        if dir.join(SETTINGS_FILE).is_file() {
            return true;
        }
        at = dir.parent();
    }
    false
}
