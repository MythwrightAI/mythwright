//! A file of this user's settings that holds a credential: the session
//! `login` keeps, an agent's that joined from here.
//!
//! Every command reads such a file as it starts, and one that is there
//! but not whole, or not there for a moment, reads as no credential: the
//! command is sent without one and told to sign in, with a session that
//! worked before and works after. So the file is never written where it
//! is. What it is to hold is written whole into a file of its own beside
//! it, its owner's alone from the moment it is there, and that is then
//! moved over it, which the system does in one step: a reader finds the
//! file as it was or as it is now, and two writers at once each put a
//! whole file there, the later standing.

use std::io::Write;
use std::path::Path;

/// Writes `bytes` as the file at `path`, whole, readable by this user
/// only.
pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder)?;
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("kept");
    // Its own name, so that two commands writing at once never write
    // into one file, and beginning with a dot, so that nothing that lists
    // the folder takes it for one that is kept.
    let part = folder.join(format!(".{name}.{}.part", &kernel::id::secret()[..12]));
    let written = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&part)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&part, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    written
}
