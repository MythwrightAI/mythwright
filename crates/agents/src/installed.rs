//! The packages that are installed: adding one, replacing one, taking one
//! away, and knowing which are not what came with this installation.
//!
//! A package is installed as a copy of its one file. What comes with the
//! product goes on changing, and the copy says what the package said when
//! it was installed. Nothing is replaced unasked: the copy may be one its
//! owner changed.

use std::collections::BTreeMap;
use std::sync::Arc;

use kernel::{Error, Principal, Result, Source};
use serde_json::json;

use crate::Agents;
use crate::came::{self, Differs};
use crate::extension::{self, Extension, MANIFEST, REPLACED};

/// What taking the package that came with this installation came to.
pub struct Taken {
    /// The package as it is installed now.
    pub extension: Arc<Extension>,
    /// Where the one that came differed from the copy it replaced, and
    /// nothing where the copy was already the one that came.
    pub differs: Option<Differs>,
}

/// Where the manifest that `came` differs from the `installed` one, and
/// nothing where they say the same. By what each says, not by how it is
/// written: a remark changed beside a line is no other package.
fn differing(installed: &str, came: &str) -> Option<Differs> {
    if installed == came {
        return None;
    }
    let says = |source: &str| blueprint::syntax::parse(source).ok().map(|parsed| parsed.to_json());
    let (was, now) = (says(installed), says(came));
    if was == now {
        return None;
    }
    // One that does not read says nothing to compare: it differs, whole.
    let differs = match (was, now) {
        (Some(was), Some(now)) => came::differs(&was, &now),
        _ => Differs::default(),
    };
    Some(differs)
}

impl Agents {
    /// The packages that came with the product and are not installed. They
    /// are read from disk each time: it is a rare question, and the folder
    /// may have changed.
    pub fn offered(&self) -> Vec<Arc<Extension>> {
        let Some(dir) = &self.inner.config.offered else { return Vec::new() };
        let installed = self.inner.extensions.lock().expect("agents lock");
        let (found, _) = extension::load_all(dir);
        found.into_values().filter(|extension| !installed.contains_key(&extension.name)).collect()
    }

    /// The names of the packages that came with the product, installed or
    /// not: what tells one installed from a file of somebody's own from
    /// one that came. Read from disk each time, as [`Agents::offered`] is.
    pub fn came_names(&self) -> std::collections::BTreeSet<String> {
        let Some(dir) = &self.inner.config.offered else { return Default::default() };
        extension::load_all(dir).0.into_keys().collect()
    }

    /// The installed packages that are not the ones of their names that
    /// came with this installation. A package is installed as a copy, and
    /// what comes with the product goes on changing: a newer one says more
    /// of its program (what it needs behind a wall, how it is asked what
    /// it offers), and the copy says what it said when it was installed.
    /// Which of the two is wanted is the owner's to say, since the copy
    /// may be one they changed: this only says that they differ.
    pub fn behind_what_came(&self) -> Vec<String> {
        self.behind_by().into_keys().collect()
    }

    /// Those packages, each with where the one that came differs from
    /// the copy installed. Nothing here says which of the two is the
    /// earlier: a copy its owner changed reads as one an earlier program
    /// shipped does ([`crate::came`]).
    pub fn behind_by(&self) -> BTreeMap<String, Differs> {
        let Some(dir) = &self.inner.config.offered else { return BTreeMap::new() };
        let (came, _) = extension::load_all(dir);
        let installed = self.inner.extensions.lock().expect("agents lock");
        installed
            .values()
            .filter_map(|extension| {
                let came = came.get(&extension.name)?;
                Some((extension.name.clone(), differing(&extension.source, &came.source)?))
            })
            .collect()
    }

    /// The packages that came with the product, read as they are now, or
    /// why there is none called `name`.
    fn came(&self, name: &str) -> Result<Arc<Extension>> {
        let found =
            self.inner.config.offered.as_ref().map(|dir| extension::load_all(dir).0).unwrap_or_default();
        if let Some(extension) = found.get(name) {
            return Ok(extension.clone());
        }
        let known: Vec<&str> = found.keys().map(String::as_str).collect();
        Err(Error::not_found(if known.is_empty() {
            format!(
                "no package called `{name}` came with this installation; install it from its folder or file"
            )
        } else {
            format!("no package called `{name}` came with this installation; there are: {}", known.join(", "))
        }))
    }

    /// Installs one of the packages that came with the product.
    pub async fn add_offered(&self, name: &str, by: &Principal) -> Result<Arc<Extension>> {
        by.require("admin")?;
        let extension = self.came(name)?;
        self.add_extension(&extension.source.clone(), by).await
    }

    /// Takes the package that came with this installation in place of the
    /// installed copy of its name, because somebody asked: nothing else
    /// ever does. It is installing the one that came over the copy, by
    /// the door that does (so what that asks, keeps beside and settles is
    /// what this does), and besides says where the two differed. One that
    /// is already the one that came is left as it is, with nothing
    /// written and nothing said.
    pub async fn take_what_came(&self, name: &str, by: &Principal) -> Result<Taken> {
        by.require("admin")?;
        let came = self.came(name)?;
        let installed = self.inner.extensions.lock().expect("agents lock").get(name).cloned();
        let Some(installed) = installed else {
            return Err(Error::not_found(format!(
                "`{name}` came with this installation and is not installed, so there is nothing to update; \
                 `{} extension add {name}` installs it",
                brand::CLI
            )));
        };
        let Some(differs) = differing(&installed.source, &came.source) else {
            return Ok(Taken { extension: installed, differs: None });
        };
        let extension = self.add_extension(&came.source.clone(), by).await?;
        Ok(Taken { extension, differs: Some(differs) })
    }

    /// Installs a package from its manifest, or replaces the one of that name.
    pub async fn add_extension(&self, source: &str, by: &Principal) -> Result<Arc<Extension>> {
        // A package says what program this daemon runs: the owner's call.
        by.require("admin")?;
        let extension = Extension::load(source).map_err(|problems| {
            Error::invalid(format!("the package does not check: {}", problems.join("; ")))
        })?;
        let taken = self.inner.adapters.lock().expect("agents lock").contains_key(&extension.name)
            && !self.inner.extensions.lock().expect("agents lock").contains_key(&extension.name);
        if taken {
            return Err(Error::conflict(format!(
                "`{}` is already the name of something that runs agents here, which a package cannot replace",
                extension.name
            )));
        }
        // `board.add` is the application's. A package called `board` that
        // brought an action `add` would be saying the same name.
        if !extension.actions.is_empty() && self.is_taken(&extension.name) {
            return Err(Error::conflict(format!(
                "`{}` is already what actions of this device's own are called under (`{}.<name>`), so a \
                 package that brings actions cannot have that name; call the package something else",
                extension.name, extension.name
            )));
        }
        let dir = self.inner.config.extensions.join(&extension.name);
        std::fs::create_dir_all(&dir)?;
        // The one it replaces is kept beside it: it may be one its owner
        // changed, and what is installed over it does not know.
        if let Ok(was) = std::fs::read_to_string(dir.join(MANIFEST))
            && was != source
        {
            std::fs::write(dir.join(REPLACED), was)?;
        }
        std::fs::write(dir.join(MANIFEST), source)?;
        let extension = Arc::new(extension);
        let name = extension.name.clone();
        self.inner.extensions.lock().expect("agents lock").insert(name.clone(), extension.clone());
        self.inner.models.lock().expect("agents lock").remove(&name);
        if extension.agent {
            self.add_adapter(&name, Arc::new(extension::Packaged(extension.clone())));
        } else {
            // It may be replacing one of its name that did bring an agent.
            self.inner.adapters.lock().expect("agents lock").remove(&name);
        }
        let blueprints: Vec<&String> = extension.blueprints.keys().collect();
        let actions: Vec<&String> = extension.actions.values().map(|action| &action.spec.name).collect();
        let data = json!({
            "extension": name, "program": extension.program, "blueprints": blueprints, "actions": actions,
        });
        let _ = self.inner.journal.emit("extension.added", Source::actor(by.actor()), data).await;
        // One that says less of its program's sign-in than the one it
        // replaces: its agents may be in no kept channel any more.
        self.settle_kept().await;
        Ok(extension)
    }

    /// Removes a package. Slots that ran on it keep their settings and say,
    /// when next asked to work, that there is nothing to run them. The
    /// blueprints it brought can no longer be applied or called; an instance
    /// of one keeps the definition it was given and runs on.
    pub async fn remove_extension(&self, name: &str, by: &Principal) -> Result<()> {
        by.require("admin")?;
        if self.inner.extensions.lock().expect("agents lock").remove(name).is_none() {
            return Err(Error::not_found(format!("there is no extension called `{name}`")));
        }
        self.inner.adapters.lock().expect("agents lock").remove(name);
        self.inner.models.lock().expect("agents lock").remove(name);
        let _ = std::fs::remove_dir_all(self.inner.config.extensions.join(name));
        let data = json!({ "extension": name });
        let _ = self.inner.journal.emit("extension.removed", Source::actor(by.actor()), data).await;
        Ok(())
    }
}
