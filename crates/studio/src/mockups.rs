//! Mockups: a collection whose records are folders that each hold a page.
//!
//! A mockup is a page made to try an idea with: plain web files in a
//! folder of the project, written by a person, by an agent, or begun from
//! the client. The folder is the mockup. It is one for holding a file
//! called `index.html`, and whatever that page uses is beside it. Nothing
//! is added to the page for it to be a mockup.
//!
//! What its collection keeps about one beyond where it is (what it is
//! called, what it is a variant of, how far it has got, what is noted of
//! it) is in a small file of settings beside the page, `mockup.yaml`, a
//! line a thing, which is there only for a mockup that has something in
//! it and whose other lines are left as they are. A mockup that says
//! nothing of itself is called what its page's `<title>` says, or what
//! its folder is called.
//!
//! A mockup is known by where it is, as a document is:
//! `<project>-<collection>:<folder>` (`shop-mockups:pricing` is
//! `mockups/pricing/`). A variant is a mockup like any other that says
//! which one it is a variant of, by that one's folder: `of: pricing`.
//!
//! Nothing here looks at the folder on a timer. It is read when it is
//! asked about; who follows it to say that something changed is the node.

use std::path::PathBuf;

use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};

use crate::collections::{Collection, Collections, Record, Store};
use crate::documents::{folder, held, line_of, named, nothing};
use crate::projects::resolve;

use ::pages::mockup::{Found, all, blocking, copy, found, named_for, starting, titled};
pub use ::pages::mockup::{PAGE, SETTINGS, is_name};

/// The field that is the folder itself and not a line of its settings.
const OF_THE_FOLDER: &str = "path";

/// What a mockup is known by, from the folder it is.
pub fn id_of(collection: &Collection, name: &str) -> String {
    format!("{}-{}:{name}", collection.project, collection.name)
}

/// The folder of the mockup known by `id`, if `id` is one of this
/// collection's.
pub fn name_of(collection: &Collection, id: &str) -> Option<String> {
    let name = id.strip_prefix(&format!("{}-{}:", collection.project, collection.name))?;
    is_name(name).then(|| name.to_string())
}

/// A mockup's folder, from the top of its project.
pub fn folder_of(collection: &Collection, name: &str) -> String {
    match folder(collection) {
        "" => name.to_string(),
        folder => format!("{folder}/{name}"),
    }
}

/// A mockup as a record. `nth` is where it comes among the others.
fn read(collection: &Collection, found: &Found, nth: usize) -> Record {
    let lines: Vec<&str> = found.settings.as_deref().unwrap_or_default().lines().collect();
    let said = |name: &str| {
        lines
            .iter()
            .find(|line| named(line) == Some(name))
            .and_then(|line| line.split_once(':'))
            .map(|(_, said)| said)
    };
    let mut data = Map::new();
    for field in &collection.fields {
        let value = match (field.name.as_str(), said(&field.name)) {
            (OF_THE_FOLDER, _) => json!(found.name),
            ("title", written) => {
                let written = written.map(|written| held(field, written)).filter(|value| !nothing(value));
                written.unwrap_or_else(|| json!(titled(&found.page).unwrap_or_else(|| found.name.clone())))
            }
            // Another of these is named by its folder where a person
            // writes it, and by what it is known by everywhere else.
            (_, Some(written)) if field.points_at() == Some(collection.name.as_str()) => {
                match held(field, written) {
                    Value::String(other) if is_name(&other) => json!(id_of(collection, &other)),
                    _ => Value::Null,
                }
            }
            (_, Some(written)) => held(field, written),
            // A folder made by hand has said nothing of how far it has
            // got: it is at the first of the stages there are.
            (_, None) if field.kind == "choice" => field.of.first().map_or(Value::Null, |first| json!(first)),
            (_, None) => Value::Null,
        };
        data.insert(field.name.clone(), value);
    }
    Record {
        id: id_of(collection, &found.name),
        project: collection.project.clone(),
        collection: collection.name.clone(),
        number: nth as i64,
        data,
        position: nth as f64,
        // A folder does not say who made it.
        created_by: "system".to_string(),
        created: found.at.0,
        updated: found.at.1,
    }
}

/// The text of a mockup's settings. `was` is what the file holds now,
/// where there is one: lines that are nobody's field here are kept.
fn settings_of(collection: &Collection, record: &Record, was: Option<&str>) -> String {
    let mut lines: Vec<String> = was.unwrap_or_default().lines().map(str::to_string).collect();
    for field in collection.fields.iter().filter(|field| field.name != OF_THE_FOLDER) {
        let mut value = record.data.get(&field.name).cloned().unwrap_or(Value::Null);
        if field.points_at() == Some(collection.name.as_str())
            && let Some(other) = value.as_str().and_then(|id| name_of(collection, id))
        {
            value = json!(other);
        }
        let at = lines.iter().position(|line| named(line) == Some(field.name.as_str()));
        match (nothing(&value), at) {
            (true, Some(at)) => {
                lines.remove(at);
            }
            (true, None) => {}
            (false, Some(at)) => lines[at] = line_of(&field.name, &value),
            (false, None) => lines.push(line_of(&field.name, &value)),
        }
    }
    let mut text = lines.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

impl Collections {
    fn mockups_root(&self, collection: &Collection) -> Result<PathBuf> {
        let dir = self
            .projects()
            .dir_of(&collection.project)
            .ok_or_else(|| Error::not_found(format!("there is no project `{}`", collection.project)))?;
        Ok(match folder(collection) {
            "" => dir,
            folder => dir.join(folder),
        })
    }

    /// Every mockup of a collection, as the folders are now, by name.
    pub(crate) async fn mockups(&self, collection: &Collection) -> Result<Vec<Record>> {
        let (root, about) = (self.mockups_root(collection)?, collection.clone());
        blocking(move || {
            Ok(all(&root).iter().enumerate().map(|(nth, found)| read(&about, found, nth + 1)).collect())
        })
        .await
    }

    /// The mockup known by `id`, with its collection, if `id` is how a
    /// mockup of some collection here is known.
    pub(crate) async fn mockup(&self, id: &str) -> Result<Option<(Collection, Record)>> {
        let Some((before, _)) = id.split_once(':') else { return Ok(None) };
        // A project's name and a collection's may each have dashes in them.
        for (at, _) in before.match_indices('-') {
            let (project, name) = (&before[..at], &before[at + 1..]);
            let Some(collection) = self.find(project, name).await? else { continue };
            if collection.store != Store::Mockups {
                continue;
            }
            let Some(folder) = name_of(&collection, id) else { continue };
            let (root, about) = (self.mockups_root(&collection)?, collection.clone());
            let record =
                blocking(move || Ok(found(&root, &folder).map(|found| read(&about, &found, 0)))).await?;
            return Ok(record.map(|record| (collection, record)));
        }
        Ok(None)
    }

    /// Makes a mockup: a folder with a page to begin from, or, for a
    /// variant, with a copy of what it is a variant of. A folder that is
    /// there already is never written over.
    pub(crate) async fn add_mockup(
        &self,
        collection: &Collection,
        data: Map<String, Value>,
        _by: &Principal,
    ) -> Result<Record> {
        let text = |name: &str| data.get(name).and_then(Value::as_str).map(str::trim).unwrap_or_default();
        let title = text("title").to_string();
        let name = match text(OF_THE_FOLDER) {
            "" => named_for(&title),
            said => said.to_string(),
        };
        if !is_name(&name) {
            return Err(Error::invalid(format!(
                "`{name}` cannot be a mockup's folder: it is one name, with no `/`, `:` or a dot in front. Say another as `path`, or a title it can be named by"
            )));
        }
        let variant_of = collection
            .fields
            .iter()
            .filter(|field| field.points_at() == Some(collection.name.as_str()))
            .find_map(|field| data.get(&field.name).and_then(Value::as_str))
            .and_then(|id| name_of(collection, id));
        let (root, about, made) = (self.mockups_root(collection)?, collection.clone(), name.clone());
        let in_project = folder_of(collection, &name);
        let page = starting(if title.is_empty() { &name } else { &title });
        let begun = blocking(move || {
            // The first mockup of a project is what makes their folder.
            std::fs::create_dir_all(&root)?;
            let to = resolve(&root, &made)?;
            if to.exists() {
                return Err(Error::conflict(format!(
                    "there is already `{in_project}` in the project; a mockup is never written over. Say another folder as `path`"
                )));
            }
            match &variant_of {
                Some(other) => {
                    let from = resolve(&root, other)?;
                    if !from.join(PAGE).is_file() {
                        return Err(Error::not_found(format!(
                            "there is no mockup `{other}` to make a variant of"
                        )));
                    }
                    copy(&from, &to).inspect_err(|_| {
                        let _ = std::fs::remove_dir_all(&to);
                    })?;
                }
                None => {
                    std::fs::create_dir_all(&to)?;
                    std::fs::write(to.join(PAGE), page)?;
                }
            }
            Ok(())
        });
        begun.await?;
        let record = Record {
            id: id_of(&about, &name),
            project: about.project.clone(),
            collection: about.name.clone(),
            number: 0,
            data,
            position: 0.0,
            created_by: "system".to_string(),
            created: 0,
            updated: 0,
        };
        self.save_mockup(&about, &record).await?;
        // What was put there is found, and said, as files are.
        self.projects().scan(&about.project).await?;
        let missing = || Error::internal(format!("the mockup `{name}` was made and cannot be read back"));
        Ok(self.mockup(&record.id).await?.ok_or_else(missing)?.1)
    }

    /// Writes what is kept about a mockup that is there. It stays where it is.
    pub(crate) async fn save_mockup(&self, collection: &Collection, record: &Record) -> Result<()> {
        let name = name_of(collection, &record.id)
            .ok_or_else(|| Error::not_found(format!("there is no record `{}`", record.id)))?;
        let said = record.data.get(OF_THE_FOLDER).and_then(Value::as_str).unwrap_or_default().trim();
        if !said.is_empty() && said != name {
            return Err(Error::invalid(format!(
                "`{}` is the folder `{}`, and a mockup is known by where it is: it cannot be moved to `{said}` by saying so. Move the folder itself; where it then is, it is another mockup",
                record.id,
                folder_of(collection, &name)
            )));
        }
        let file = format!("{}/{SETTINGS}", folder_of(collection, &name));
        let was = self.projects().read(&collection.project, &file).await.ok();
        let text = settings_of(collection, record, was.as_deref());
        if was.as_deref() != Some(text.as_str()) && !(was.is_none() && text.is_empty()) {
            self.projects().write(&collection.project, &file, &text, &Principal::system()).await?;
        }
        Ok(())
    }

    /// Takes a mockup away: its folder, and everything in it.
    pub(crate) async fn remove_mockup(&self, collection: &Collection, record: &Record) -> Result<()> {
        let name = name_of(collection, &record.id)
            .ok_or_else(|| Error::not_found(format!("there is no record `{}`", record.id)))?;
        self.projects().remove(&collection.project, &folder_of(collection, &name), &Principal::system()).await
    }

    /// The mockup a file of a project is part of, if it is part of one:
    /// its collection and what the mockup is known by.
    pub async fn mockup_at(&self, project: &str, file: &str) -> Result<Option<(Collection, String)>> {
        for collection in self.list(Some(project)).await? {
            if collection.store != Store::Mockups {
                continue;
            }
            let within = match folder(&collection) {
                "" => Some(file),
                folder => file.strip_prefix(folder).and_then(|rest| rest.strip_prefix('/')),
            };
            if let Some(name) = within.and_then(|rest| rest.split('/').next()).filter(|name| is_name(name)) {
                let id = id_of(&collection, name);
                return Ok(Some((collection, id)));
            }
        }
        Ok(None)
    }
}
