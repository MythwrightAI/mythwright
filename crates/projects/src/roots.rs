//! Where a path of a project's is: in the project's own folder, or, where
//! it begins `@blueprints/`, in the factory's place for the project's own
//! blueprints (`conductor::library::folders`), which is in the factory's
//! data folder and never the project's. Every route that reads or writes a
//! project's file asks here first, so that a fork, the fleet's blueprint
//! and anything else of the project's own is read, written, said to have
//! changed, scanned and put in the Trash as any file of the project is.

use texts::library::folders::{PLACE, in_place, kept_at};

use super::*;

/// A project's two folders: its own, and the factory's place for its
/// own blueprints.
#[derive(Clone, Debug)]
pub struct Roots {
    pub dir: PathBuf,
    pub place: PathBuf,
}

impl Roots {
    /// The folder a path is under, and the path within it.
    fn of<'a>(&self, path: &'a str) -> (&Path, &'a str) {
        match in_place(path) {
            Some(rest) => (&self.place, rest),
            None => (&self.dir, path),
        }
    }

    /// The path as the system has it, refused where it leads out of the
    /// folder it is under. The factory's place is not made by it: what
    /// writes there makes the folders it writes in, and a reading made it
    /// again under a project that had just gone into the Trash with it.
    pub fn resolve(&self, path: &str) -> Result<PathBuf> {
        let (root, rest) = self.of(path);
        if root != self.place {
            return resolve(root, rest);
        }
        if rest.is_empty() {
            return Err(Error::invalid(format!(
                "`{path}` is where the factory keeps the project's own blueprints"
            )));
        }
        if root.exists() {
            return resolve(root, rest);
        }
        // Where the place is not there yet, nothing under it can lead out
        // of it, and what is written is held to the same names.
        let mut above: &Path = root;
        while !above.exists() {
            above = above.parent().ok_or_else(|| Error::invalid(format!("`{path}` has nowhere to be")))?;
        }
        let named = resolve(above, rest)?;
        Ok(root.join(named.strip_prefix(above).unwrap_or(&named)))
    }

    /// Whether `target` is the top of either folder: never a file.
    pub fn is_top(&self, target: &Path) -> bool {
        target == self.dir || target == self.place
    }

    /// What a file is called as a path of the project's.
    pub fn called(&self, target: &Path) -> Option<String> {
        let (root, before) = match target.starts_with(&self.place) {
            true => (&self.place, format!("{PLACE}/")),
            false => (&self.dir, String::new()),
        };
        let within = target.strip_prefix(root).ok()?.components();
        let parts: Vec<String> = within.map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
        Some(format!("{before}{}", parts.join("/")))
    }

    /// The entry of a file or folder under either folder.
    pub(crate) fn entry(&self, target: &Path, meta: &std::fs::Metadata) -> Entry {
        let mut entry = entry(&self.dir, target, meta);
        entry.path = self.called(target).unwrap_or(entry.path);
        entry
    }

    /// What a scan looks at: every file of the project's folder, and of
    /// the factory's place for it, each by its path of the project's.
    pub fn stamps(&self) -> BTreeMap<String, (i64, u64)> {
        let place = walk(&self.place).into_iter().map(|e| Entry { path: format!("{PLACE}/{}", e.path), ..e });
        let all = walk(&self.dir).into_iter().chain(place).filter(|e| !e.dir);
        all.map(|e| (e.path, (e.modified, e.size))).collect()
    }

    /// What a file is stamped with now, as [`Roots::stamps`] stamps it.
    pub(crate) fn stamp(&self, path: &str) -> Option<(i64, u64)> {
        let (root, rest) = self.of(path);
        stamp_now(root, rest)
    }
}

impl Projects {
    /// Where the factory keeps `id`'s own blueprints: in its data folder,
    /// or beside where it makes projects where it was told of none.
    pub fn place_of(&self, id: &str) -> PathBuf {
        let data = self.data.lock().expect("projects lock").clone();
        kept_at(&data.unwrap_or_else(|| self.root.parent().unwrap_or(&self.root).to_path_buf()), id)
    }

    /// A project's two folders, or why there is no such project.
    pub fn roots(&self, id: &str) -> Result<Roots> {
        Ok(Roots { dir: self.dir(id)?, place: self.place_of(id) })
    }

    /// Where a file of a project is on the disk, with nothing asked of it:
    /// for whatever reads a project's own blueprints as a library does.
    pub fn file_of(&self, id: &str, path: &str) -> Option<PathBuf> {
        let roots = Roots { dir: self.dir_of(id)?, place: self.place_of(id) };
        let (root, rest) = roots.of(path);
        texts::library::is_safe_name(rest).then(|| root.join(rest))
    }
}

impl Projects {
    /// Carries the factory's place for `id`'s own blueprints into the
    /// Trash's folder of the project's entry `trash` (`into`), or back out
    /// of it, so that it goes into the Trash with its project and comes
    /// back with it. Kept there as what the entry keeps (`crate::trashed::KEPT`),
    /// which a start leaves where it is. Nothing to carry is nothing done.
    pub(crate) fn carry_place(&self, id: &str, trash: &str, into: bool) -> Result<()> {
        let Some(folder) = self.trash_folder() else { return Ok(()) };
        let (place, kept) = (self.place_of(id), folder.join(trash).join(crate::trashed::KEPT));
        let (from, to) = if into { (place, kept) } else { (kept, place) };
        if !from.exists() || to.exists() {
            return Ok(());
        }
        std::fs::create_dir_all(to.parent().unwrap_or(&to))?;
        Ok(std::fs::rename(from, to)?)
    }
}
