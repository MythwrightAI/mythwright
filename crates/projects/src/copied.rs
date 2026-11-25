//! Where a copy's projects are put (`studio::copy`, which says what of
//! the studio's does not go into a copy, offers what is here under the
//! names it had).
//!
//! A project is a directory, said by its whole path, and two factories
//! must never share one. In a copy a project's place is written relative
//! to a folder of projects ([`uproot`]), and the new factory puts each
//! under its own ([`plant`]). A project whose directory was not in the
//! factory's own projects folder is not copied at all: it is a project in
//! the new factory, with an empty directory of its own. What a scan last
//! saw of each is written again where it now is ([`look`]).

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use journal::copy::Connection;
use rusqlite::params;

/// A project of a copy, and where its files are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub id: String,
    pub name: String,
    /// Where its files are in a copy: under the copy's folder of projects,
    /// with `/` between the parts.
    pub at: String,
    /// Its directory: when a copy is taken, the one it had in the factory
    /// copied; when a factory is started from one, the one it has there.
    pub dir: PathBuf,
    /// When a copy is taken: whether that directory was in the factory's
    /// own projects folder, so that its files go. When a factory is started
    /// from one: whether the copy said where its files are.
    pub held: bool,
    /// Its tables that are kept as files in it, which are only there if
    /// its files are.
    pub kept_as_files: Vec<String>,
}

struct Row {
    id: String,
    name: String,
    path: String,
}

fn projects(connection: &Connection) -> rusqlite::Result<Vec<Row>> {
    let mut statement = connection.prepare("SELECT id, name, path FROM projects ORDER BY created, id")?;
    let rows =
        statement.query_map([], |row| Ok(Row { id: row.get(0)?, name: row.get(1)?, path: row.get(2)? }))?;
    rows.collect()
}

fn kept_as_files(connection: &Connection, project: &str) -> rusqlite::Result<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT name FROM collections WHERE project = ?1 AND store = 'files' ORDER BY name")?;
    let rows = statement.query_map([project], |row| row.get(0))?;
    rows.collect()
}

/// The parts of a place under a folder of projects, if `path` is one: a
/// path that goes down, and neither up nor from the top.
fn parts(path: &str) -> Option<Vec<String>> {
    let parts: Option<Vec<String>> = Path::new(path)
        .components()
        .map(|part| match part {
            Component::Normal(name) => name.to_str().map(str::to_string),
            _ => None,
        })
        .collect();
    parts.filter(|parts| !parts.is_empty() && !path.starts_with(['/', '\\']))
}

/// `path` with its links followed, when it is there to follow them in.
fn real(path: &Path) -> PathBuf {
    kernel::whole::whole(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A place of its own under a folder of projects for a project that has
/// none there: its id, or its id with a number, whichever is not the first
/// part of a place already taken.
fn fresh(id: &str, taken: &mut BTreeSet<String>) -> String {
    let id = Some(blueprint::expr::slug(id)).filter(|id| !id.is_empty()).unwrap_or_else(|| "project".into());
    let mut name = id.clone();
    let mut count = 2;
    while taken.contains(&name) {
        name = format!("{id}-{count}");
        count += 1;
    }
    taken.insert(name.clone());
    name
}

/// Writes every project's place over with a new one. They are all moved
/// out of the way first: no two projects may be in one place, and a place
/// one of them is leaving may be the place another is going to.
fn place(connection: &Connection, placed: &[(String, String)]) -> rusqlite::Result<()> {
    connection.execute("UPDATE projects SET path = '//moving//' || id", [])?;
    for (id, path) in placed {
        connection.execute("UPDATE projects SET path = ?2 WHERE id = ?1", params![id, path])?;
    }
    Ok(())
}

/// Takes the projects of a copy out of the directories they were in: each
/// one's place is written relative to a folder of projects, and no path of
/// the factory copied is left in the copy. `root` is that factory's own
/// projects folder: a project inside it keeps its place under it, and its
/// files are to go with the copy; one anywhere else is given a place of
/// its own, for a directory that will be empty.
pub fn uproot(connection: &Connection, root: &Path) -> rusqlite::Result<Vec<Placed>> {
    let root = real(root);
    let rows = projects(connection)?;
    let inside: Vec<Option<Vec<String>>> = rows
        .iter()
        .map(|row| {
            let dir = real(Path::new(&row.path));
            kernel::whole::under(&dir, &root).and_then(|under| parts(&under.to_string_lossy()))
        })
        .collect();
    let mut taken: BTreeSet<String> = inside.iter().flatten().map(|parts| parts[0].clone()).collect();
    let mut placed = Vec::new();
    for (row, inside) in rows.into_iter().zip(inside) {
        let held = inside.is_some();
        let at = match inside {
            Some(parts) => parts.join("/"),
            None => fresh(&row.id, &mut taken),
        };
        let kept_as_files = kept_as_files(connection, &row.id)?;
        placed.push(Placed {
            id: row.id,
            name: row.name,
            at,
            dir: PathBuf::from(row.path),
            held,
            kept_as_files,
        });
    }
    let moves: Vec<(String, String)> = placed.iter().map(|one| (one.id.clone(), one.at.clone())).collect();
    place(connection, &moves)?;
    Ok(placed)
}

/// Puts the projects of a copy under `root`, the projects folder of the
/// factory being started from it: each at the place the copy gives it
/// there. A project whose place in the copy is not one under a folder of
/// projects (a copy that was not made by [`uproot`]) is never left where
/// it says: it is given a place of its own under `root` like any other.
pub fn plant(connection: &Connection, root: &Path) -> rusqlite::Result<Vec<Placed>> {
    let rows = projects(connection)?;
    let said: Vec<Option<Vec<String>>> = rows.iter().map(|row| parts(&row.path)).collect();
    let mut taken: BTreeSet<String> = said.iter().flatten().map(|parts| parts[0].clone()).collect();
    let mut placed = Vec::new();
    for (row, said) in rows.into_iter().zip(said) {
        let held = said.is_some();
        let at = match said {
            Some(parts) => parts.join("/"),
            None => fresh(&row.id, &mut taken),
        };
        let dir = at.split('/').fold(root.to_path_buf(), |dir, part| dir.join(part));
        let kept_as_files = kept_as_files(connection, &row.id)?;
        placed.push(Placed { id: row.id, name: row.name, at, dir, held, kept_as_files });
    }
    let moves: Vec<(String, String)> =
        placed.iter().map(|one| (one.id.clone(), one.dir.to_string_lossy().into_owned())).collect();
    place(connection, &moves)?;
    Ok(placed)
}

/// Writes down what each project of a copy holds now, where it now is, as
/// a scan would have found it: the first scan in the new factory then
/// reports what changed after, and not every file there is. It is
/// written where the new factory will read it: in the copy of the
/// project's own journal for a project that has one in `beside` (the
/// journals of the copy that keep their projects' tables, each by its
/// project), and otherwise in the factory's. The files of the factory's
/// place for each project's own blueprints, in the copy's data folder
/// `data`, are among them, as a scan stamps them. Returns how many files
/// that was.
pub fn look(
    connection: &Connection,
    beside: &[(String, Connection)],
    data: &Path,
) -> rusqlite::Result<usize> {
    connection.execute("DELETE FROM project_files", [])?;
    let mut seen = 0;
    for row in projects(connection)? {
        let kept = beside.iter().find(|(project, _)| *project == row.id).map(|(_, own)| own);
        let kept = kept.unwrap_or(connection);
        kept.execute("DELETE FROM project_files WHERE project = ?1", [&row.id])?;
        let place = texts::library::folders::kept_at(data, &row.id);
        let roots = crate::Roots { dir: PathBuf::from(&row.path), place };
        for (path, (modified, size)) in roots.stamps() {
            kept.execute(
                "INSERT OR REPLACE INTO project_files (project, path, mtime, size) VALUES (?1, ?2, ?3, ?4)",
                params![row.id, path, modified, size as i64],
            )?;
            seen += 1;
        }
    }
    Ok(seen)
}
