//! The factories on this device, known to each other by name and address
//! and by nothing else.
//!
//! Factories share nothing: each has its own data folder and its own
//! database, and none reads another's. But a person at this device says
//! "the `dev1` one", and a page in one factory offers the way to another,
//! so somewhere has to say which factories there are. This is that place: a
//! small file in the user's configuration folder, written by each factory
//! as it starts, holding for each named factory its name, its data folder
//! and its address. It never holds a credential, and nothing in it lets one
//! factory act as another: signing in is to each factory separately.
//!
//! It also holds which directories the factories have adopted. A project
//! is a directory, and two factories that adopted the same one would share
//! its files, whatever else they keep apart. So a directory that a factory
//! of one profile holds is refused to a factory of another ([`Held`]).
//! Nothing is written into the directory itself: an adopted directory is
//! left exactly as it was. Only a directory outside its factory's own data
//! folder is written down here; one inside it is known for another's by
//! where it is.
//!
//! A factory with no profile is as factories always were, and writes
//! here only when it has adopted a directory, which is the one thing about
//! it another factory has to know.
//!
//! The file is the only thing here that more than one factory writes. Each
//! change is made holding a lock beside it, to the file as it then is, and
//! the file is replaced whole.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use kernel::{Error, Result, whole};
use serde::{Deserialize, Serialize};

/// A named factory that has been started on this device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Factory {
    /// What its profile is called. Not what tells it from another: two
    /// factories may be called the same, and their data folders differ.
    pub name: String,
    pub data: PathBuf,
    /// Where it listened when it last started.
    pub address: SocketAddr,
}

/// A directory that is a project's in some factory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Directory {
    path: PathBuf,
    /// The data folder of the factory that holds it.
    data: PathBuf,
    /// What that factory's profile is called; empty for one with none.
    // Written as `environment`, as it always was: the file is read by
    // every program on the device, of whatever version.
    #[serde(rename = "environment")]
    profile: String,
}

#[derive(Default, Serialize, Deserialize)]
struct Contents {
    #[serde(default)]
    factories: Vec<Factory>,
    #[serde(default)]
    directories: Vec<Directory>,
}

#[derive(Clone, Debug)]
pub struct Registry {
    path: PathBuf,
}

/// `path` with every link in it followed, as far as it exists: two ways of
/// writing one directory are then one path. What does not exist yet is kept
/// as it was written, under the part that does.
pub(crate) fn settled(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(real) = whole::whole(existing) {
            return rest.iter().rev().fold(real, |path, part| path.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// Whether one directory is the other, or is inside it.
fn overlap(a: &Path, b: &Path) -> bool {
    whole::within(a, b) || whole::within(b, a)
}

/// How a factory is spoken of in a sentence.
fn called(profile: &str) -> String {
    if profile.is_empty() { "a factory with no profile".into() } else { format!("a `{profile}` factory") }
}

impl Registry {
    pub fn at(path: impl Into<PathBuf>) -> Registry {
        Registry { path: path.into() }
    }

    /// This user's registry: the one every factory they start writes to.
    pub fn of_this_user() -> Registry {
        Registry::at(brand::config_dir().join("environments.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> Result<Contents> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Contents::default()),
            Err(err) => return Err(Error::from(err).context(self.path.display())),
        };
        let mut contents: Contents = serde_json::from_str(&text).map_err(|err| {
            Error::invalid(format!(
                "{} lists the factories on this device, and cannot be read ({err}). Remove it, and start \
                 each factory again: each writes itself back as it starts",
                self.path.display()
            ))
        })?;
        // On Windows a program from before whole paths were said plainly
        // wrote the system's long form of each. Read as it is said now, a
        // factory is still the one that was written down.
        let plainly = |path: &mut PathBuf| *path = whole::said_plainly(std::mem::take(path));
        contents.factories.iter_mut().for_each(|factory| plainly(&mut factory.data));
        for held in &mut contents.directories {
            plainly(&mut held.path);
            plainly(&mut held.data);
        }
        // A factory whose data is gone is gone, and holds nothing.
        contents.factories.retain(|factory| factory.data.is_dir());
        contents.directories.retain(|directory| directory.data.is_dir());
        Ok(contents)
    }

    /// Changes the file as it is at this moment, with nobody else changing
    /// it meanwhile.
    fn change<T>(&self, change: impl FnOnce(&mut Contents) -> Result<T>) -> Result<T> {
        let folder = self.path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(folder).map_err(|err| Error::from(err).context(folder.display()))?;
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.path.with_extension("lock"))?;
        lock.lock()?;
        let mut contents = self.read()?;
        let outcome = change(&mut contents)?;
        contents.factories.sort_by(|a, b| (&a.name, &a.data).cmp(&(&b.name, &b.data)));
        contents.directories.sort_by(|a, b| (&a.path, &a.data).cmp(&(&b.path, &b.data)));
        let partial = self.path.with_extension(format!("new-{}", std::process::id()));
        std::fs::write(&partial, format!("{}\n", serde_json::to_string_pretty(&contents)?))?;
        std::fs::rename(&partial, &self.path)?;
        Ok(outcome)
    }

    /// The named factories that have been started on this device and are
    /// still there, by name.
    pub fn factories(&self) -> Result<Vec<Factory>> {
        let mut factories = self.read()?.factories;
        factories.sort_by(|a, b| (&a.name, &a.data).cmp(&(&b.name, &b.data)));
        Ok(factories)
    }

    /// The factories called `name`. More than one is possible, and whoever
    /// asked for "the one called that" has then to be told there are two.
    pub fn named(&self, name: &str) -> Result<Vec<Factory>> {
        Ok(self.factories()?.into_iter().filter(|factory| factory.name == name).collect())
    }

    /// A factory has started: where it is now, and the directories it has
    /// adopted, replace whatever was written of it before. One with no
    /// profile is not listed, and writes nothing unless it holds
    /// something, or did.
    pub fn started(
        &self,
        data: &Path,
        profile: &str,
        address: SocketAddr,
        projects: &[PathBuf],
    ) -> Result<()> {
        let data = settled(data);
        let holds: Vec<PathBuf> =
            projects.iter().map(|path| settled(path)).filter(|path| !whole::within(path, &data)).collect();
        if profile.is_empty()
            && holds.is_empty()
            && !self.read()?.directories.iter().any(|directory| directory.data == data)
        {
            return Ok(());
        }
        self.change(|contents| {
            contents.factories.retain(|factory| factory.data != data);
            if !profile.is_empty() {
                contents.factories.push(Factory { name: profile.to_string(), data: data.clone(), address });
            }
            contents.directories.retain(|directory| directory.data != data);
            contents.directories.extend(holds.into_iter().map(|path| Directory {
                path,
                data: data.clone(),
                profile: profile.to_string(),
            }));
            Ok(())
        })
    }
}

/// The data folder a directory is in, if it is in one: a factory's data
/// folder is known by the database in it, whoever's it is and whether or
/// not that factory was ever written down anywhere.
fn data_around(dir: &Path) -> Option<&Path> {
    dir.ancestors().find(|folder| folder.join(crate::DATABASE_FILE).is_file())
}

/// What one factory holds, and what it may not: asked before a directory
/// becomes one of its projects.
pub struct Held {
    registry: Registry,
    /// The factory's own data folder, which is what it is known by here.
    data: PathBuf,
    /// What its profile is called; empty for one with none.
    profile: String,
}

impl Held {
    pub fn new(registry: Registry, data: &Path, profile: &str) -> Held {
        Held { registry, data: settled(data), profile: profile.to_string() }
    }

    fn refuse(&self, dir: &Path, what: &str, theirs: &str, data: &Path) -> Error {
        Error::conflict(format!(
            "{dir} {what} {holder} (its data is in {data}), and this is {this}. Two profiles never work in \
             the same directory: whatever one did there, the other would find done. Give this one a directory \
             of its own, such as a clone or a copy of that one",
            dir = dir.display(),
            holder = called(theirs),
            data = data.display(),
            this = called(&self.profile),
        ))
    }
}

impl studio::Holdings for Held {
    fn take(&self, dir: &Path) -> Result<()> {
        let dir = settled(dir);
        // What a factory keeps for itself is in its data folder. A
        // directory in another's is that factory's, and what profile
        // that is, its folder says.
        if let Some(data) = data_around(&dir).filter(|data| !whole::same(data, &self.data)) {
            let theirs = match crate::profile::Stamp::read(data) {
                Ok(stamp) => stamp.map(|stamp| stamp.profile).unwrap_or_default(),
                Err(err) => {
                    return Err(err.context(format!("{} is among another factory's data", dir.display())));
                }
            };
            if theirs != self.profile {
                return Err(self.refuse(&dir, "is among the data of", &theirs, data));
            }
        }
        // Only another profile's is refused. Two factories of one
        // profile are one profile's work, and are let be.
        let other = |data: &Path, profile: &str| data != self.data && profile != self.profile;
        let refusal = |contents: &Contents| {
            if let Some(held) = contents
                .directories
                .iter()
                .find(|held| other(&held.data, &held.profile) && overlap(&dir, &held.path))
            {
                let what =
                    if dir == held.path { "is a project of" } else { "shares files with a project of" };
                return Some(self.refuse(&dir, what, &held.profile, &held.data));
            }
            let around = contents
                .factories
                .iter()
                .find(|factory| other(&factory.data, &factory.name) && whole::within(&factory.data, &dir))?;
            Some(self.refuse(&dir, "holds the data of", &around.name, &around.data))
        };
        // A directory of the factory's own is nobody else's business, and
        // is asked about without anything being written down.
        let asked = if whole::within(&dir, &self.data) {
            self.registry.read().map(|contents| refusal(&contents))
        } else {
            self.registry.change(|contents| {
                if let Some(refusal) = refusal(contents) {
                    return Ok(Some(refusal));
                }
                let mine =
                    Directory { path: dir.clone(), data: self.data.clone(), profile: self.profile.clone() };
                if !contents.directories.contains(&mine) {
                    contents.directories.push(mine);
                }
                Ok(None)
            })
        };
        match asked {
            Ok(None) => Ok(()),
            Ok(Some(refusal)) => Err(refusal),
            // A factory with no profile is as it always was: where the
            // list cannot be kept, it works without one.
            Err(_) if self.profile.is_empty() => Ok(()),
            Err(err) => {
                Err(err.context(format!("could not find out whether {} is another profile's", dir.display())))
            }
        }
    }

    fn give_back(&self, dir: &Path) {
        let dir = settled(dir);
        let _ = self.registry.change(|contents| {
            contents.directories.retain(|held| !(held.data == self.data && held.path == dir));
            Ok(())
        });
    }
}
