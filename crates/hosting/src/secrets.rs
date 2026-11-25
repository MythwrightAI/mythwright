//! Secrets: values a program needs and nothing else should see.
//!
//! A program that takes payments needs the payment service's key; one that
//! sends mail, a password. Written in a blueprint, either would be in the
//! text, in every instance's params, in the database and in whatever
//! copies of those there are. So the factory keeps them apart, each under
//! a name, and a blueprint says the name:
//!
//! ```yaml
//! env:
//!   STRIPE_KEY: { secret: stripe }
//! ```
//!
//! The value is kept in a file of its own, sealed with a key made from
//! this installation's secret ([`link::kept`]), readable by its owner
//! alone, in a folder of the data folder that no copy of the factory
//! carries. It is never in the database, an event, a measurement, a log or
//! an argument. It goes from that file into the environment of the program
//! a step or a service runs, as that program is started, and what is kept
//! of a service so that it can be started again is the name.
//!
//! A file that is not sealed is one written by a build that did not seal,
//! or put there by hand: it is read as it is, and sealed where it is as
//! soon as it has been read.
//!
//! A project's environment has secrets of its own, kept here too, apart
//! from the factory's (`@projects/<project>/<environment>/<name>`) and
//! sealed under the name the three make together, which is what they are
//! sought by (`environments::project::sought`): the environment says
//! which it has, and this keeps their values.
//!
//! What a program does with what it is given is the program's: one that
//! prints its environment has printed its secret.

use std::path::PathBuf;

use kernel::{Error, Result};
use serde::Serialize;

/// The secrets a factory keeps: a folder with a file for each, and the
/// key each is sealed with.
#[derive(Clone)]
pub struct Secrets {
    dir: PathBuf,
    seal: link::kept::Seal,
}

/// A secret as it is listed. It never has the value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Kept {
    pub name: String,
    /// When it was last set, in milliseconds since 1970.
    pub set: i64,
}

/// Why `name` may not be a secret's name, if it may not
/// (`environments::project::secret_refused`, where the rule is, so that
/// the runner can refuse a name before a step runs).
pub fn refused(name: &str) -> Option<String> {
    environments::project::secret_refused(name)
}

/// The folder a project's environments' secrets are kept in, beside the
/// factory's own: no factory secret can be called it, so it is never
/// listed with them.
const PROJECTS: &str = "@projects";

impl Secrets {
    pub(crate) fn new(dir: PathBuf, seal: link::kept::Seal) -> Secrets {
        Secrets { dir, seal }
    }

    fn file(&self, name: &str) -> Result<PathBuf> {
        match refused(name) {
            Some(why) => Err(Error::invalid(why)),
            None => Ok(self.dir.join(name)),
        }
    }

    /// Keeps `value` under `name`, in place of whatever was kept there.
    pub fn set(&self, name: &str, value: &str) -> Result<()> {
        self.file(name)?;
        self.keep(name, value)
    }

    /// Keeps `value` under the name it is sought by, sealed under that.
    fn keep(&self, name: &str, value: &str) -> Result<()> {
        let file = self.sought(name)?;
        if value.is_empty() {
            return Err(Error::invalid(format!("`{name}` was given nothing to keep")));
        }
        let dir = file.parent().map(PathBuf::from).unwrap_or_else(|| self.dir.clone());
        std::fs::create_dir_all(&dir)?;
        for folder in dir.ancestors().take_while(|folder| folder.starts_with(&self.dir)) {
            only_owner(folder, true);
        }
        // Written beside it and put in its place, so that what reads it as
        // it is set reads the whole of one or the other; and readable by
        // its owner alone before anything is in it.
        let sealed = self.seal.seal(name, value.as_bytes()).map_err(|err| Error::internal(err.message))?;
        let leaf = file.file_name().map(|leaf| leaf.to_string_lossy().into_owned()).unwrap_or_default();
        let beside = dir.join(format!(".{leaf}.{:x}", kernel::id::random_below(u64::MAX)));
        std::fs::write(&beside, "")?;
        only_owner(&beside, false);
        std::fs::write(&beside, sealed)?;
        std::fs::rename(&beside, &file).inspect_err(|_| {
            let _ = std::fs::remove_file(&beside);
        })?;
        Ok(())
    }

    /// The folder of `project`'s `environment`, refused where either is
    /// not a name that can be.
    fn environment_dir(&self, project: &str, environment: &str) -> Result<PathBuf> {
        let plain = |part: &str| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part.contains(['/', '\\', ':'])
                && part.len() <= 100
        };
        if !plain(project) {
            return Err(Error::invalid(format!("`{project}` cannot be a project's id")));
        }
        environments::project::check_name(environment)?;
        Ok(self.dir.join(PROJECTS).join(project).join(environment))
    }

    /// Where a secret is, by the name it is sought by: one of the
    /// factory's, or `<project>/<environment>/<name>` for one of a
    /// project's environment's (`environments::project::sought`).
    fn sought(&self, sought: &str) -> Result<PathBuf> {
        match sought.split('/').collect::<Vec<_>>()[..] {
            [name] => self.file(name),
            [project, environment, name] => match refused(name) {
                Some(why) => Err(Error::invalid(why)),
                None => Ok(self.environment_dir(project, environment)?.join(name)),
            },
            _ => Err(Error::invalid(format!("`{sought}` is not the name of a secret"))),
        }
    }

    /// Keeps `value` as the secret `name` of `project`'s `environment`,
    /// sealed under the name it is sought by.
    pub fn set_of(&self, project: &str, environment: &str, name: &str, value: &str) -> Result<()> {
        self.keep(&environments::project::sought(project, environment, name), value)
    }

    /// Whether `project`'s `environment` has a value for its secret `name`.
    pub fn has_of(&self, project: &str, environment: &str, name: &str) -> bool {
        let sought = environments::project::sought(project, environment, name);
        self.sought(&sought).is_ok_and(|file| file.is_file())
    }

    /// Takes away the value of `project`'s `environment`'s secret `name`.
    pub fn remove_of(&self, project: &str, environment: &str, name: &str) -> Result<bool> {
        let file = self.sought(&environments::project::sought(project, environment, name))?;
        Ok(std::fs::remove_file(file).is_ok())
    }

    /// Takes away every value of `project`'s `environment`.
    pub fn forget_environment(&self, project: &str, environment: &str) -> Result<()> {
        let _ = std::fs::remove_dir_all(self.environment_dir(project, environment)?);
        Ok(())
    }

    /// Takes away every value of every environment of `project`.
    pub fn forget_project(&self, project: &str) -> Result<()> {
        self.environment_dir(project, "x")?;
        let _ = std::fs::remove_dir_all(self.dir.join(PROJECTS).join(project));
        Ok(())
    }

    /// The value of the factory's secret `name`. Only ever one of the
    /// factory's: a name is what a blueprint says, and may be worked out,
    /// so one that reads as a project's environment's is refused here, and
    /// whoever hands one of those on asks [`Secrets::get_of`] for it by its
    /// three parts, having said which project it is for.
    pub fn get(&self, name: &str) -> Result<String> {
        self.file(name)?;
        self.open(name)
    }

    /// The value of a secret by the one name the runner hands on for it:
    /// the factory's own by its name, or a project's environment's by
    /// the name it is sought by (`project/environment/name`), which the
    /// runner works out from what the environment lists, having refused
    /// a `/` in any name a text writes (`environments::held::Held::sought`).
    pub fn handed(&self, name: &str) -> Result<String> {
        self.open(name)
    }

    /// The value of the secret `name` of `project`'s `environment`.
    pub fn get_of(&self, project: &str, environment: &str, name: &str) -> Result<String> {
        self.open(&environments::project::sought(project, environment, name))
    }

    /// The value kept under `sought`: a name of the factory's, or the name
    /// a project's environment's secret is sought by.
    fn open(&self, sought: &str) -> Result<String> {
        let (file, name) = (self.sought(sought)?, sought);
        let kept = std::fs::read(file).map_err(|_| match sought.split('/').collect::<Vec<_>>()[..] {
            [project, environment, name] => Error::not_found(format!(
                "the environment `{environment}` of `{project}` lists the secret `{name}` and has no value for \
                 it (a copy of a factory carries none): set it again with `{} secret set {name} --project \
                 {project} --environment {environment}`",
                brand::CLI
            )),
            _ => Error::not_found(format!(
                "there is no secret called `{name}`; whoever owns the factory sets it with `{} secret set {name}`",
                brand::CLI
            )),
        })?;
        let sealed = link::kept::Seal::is_sealed(&kept);
        let plain = match sealed {
            true => self.seal.open(name, &kept).map_err(|err| Error::invalid(err.message))?,
            false => kept,
        };
        let value = String::from_utf8(plain)
            .map_err(|_| Error::invalid(format!("what is kept as `{name}` is not text")))?;
        // Written as it is, by a build that did not seal or by hand: it is
        // sealed where it is, now that it has been read.
        if !sealed {
            let _ = self.keep(name, &value);
        }
        Ok(value)
    }

    /// The names there are, in order, and when each was set.
    pub fn list(&self) -> Vec<Kept> {
        let mut kept: Vec<Kept> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let meta = entry.metadata().ok().filter(|meta| meta.is_file() && refused(&name).is_none())?;
                let set = meta
                    .modified()
                    .ok()
                    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |since| since.as_millis() as i64);
                Some(Kept { name, set })
            })
            .collect();
        kept.sort_by(|one, other| one.name.cmp(&other.name));
        kept
    }

    /// Takes away what is kept under `name`. True when there was something.
    pub fn remove(&self, name: &str) -> Result<bool> {
        Ok(std::fs::remove_file(self.file(name)?).is_ok())
    }
}

/// Lets only its owner read `path`, where the system has such a thing to
/// say. Where it has not, the folder is as private as the data folder is.
fn only_owner(path: &std::path::Path, folder: bool) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if folder { 0o700 } else { 0o600 };
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, folder);
    }
}
