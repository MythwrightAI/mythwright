//! A library: a collection whose records are the Markdown files of a folder.
//!
//! The other ways of keeping records make the files (or the rows) theirs:
//! numbered, with what is kept about each at its head. A library takes a
//! folder of documents as it finds it. A document is a file that ends in
//! `.md`, in the folder or in one under it, named whatever it is named;
//! its title is the heading it begins with, or its file's name where it
//! begins with none; its body is the rest. What its collection declares
//! beyond those is kept in a block of settings at its head, which is there
//! only for a document that has something in it, and whose other lines are
//! left as they are.
//!
//! A document is known by where it is: `<project>-<library>:<path>`, the
//! path without its ending and with `:` between its parts
//! (`shop-docs:guides:install` is `guides/install.md`). So it is the same
//! document on every device that has the same files, and after a restart;
//! and a file that is moved is another document, with a history of its own.
//!
//! Nothing here looks at the folder on a timer. It is read when it is
//! asked about, so what another program wrote is there the next time; and
//! who follows the folder to say that it changed is the node (`api`).

use std::path::{Component, Path, PathBuf};

use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};

use crate::collections::{Collection, Collections, Record, Store};
use crate::fields::Field;
use crate::projects::resolve;

pub use ::pages::document::{ENDING, folders, head, setting};
pub(crate) use ::pages::document::{
    is_document, line_of, named, nothing, parts, path_for, stamps, stem, texts,
};

/// What a line of the block says, as `field` keeps it ([`::pages::document::held`]).
pub(crate) fn held(field: &Field, written: &str) -> Value {
    ::pages::document::held(&field.kind, written)
}

/// The fields that are the file itself and not a line at its head.
const OF_THE_FILE: &[&str] = &["title", "body", "path"];

/// The folder a library called `name` is to be the documents of: the one
/// `written` for it, else the one it had, else the one called what it is.
/// `.` is the project's own folder.
pub(crate) fn place(name: &str, written: Option<&str>, was: Option<&Collection>) -> Result<String> {
    let said = written.map(str::trim).filter(|said| !said.is_empty());
    let Some(folder) = said else {
        return Ok(was.and_then(|was| was.at.clone()).unwrap_or_else(|| name.to_string()));
    };
    let refuse = |why: &str| {
        Error::invalid(format!(
            "`at: {folder}` is not a folder of the project for `{name}` to be the documents of: {why}. Say one as it is from the top of the project, like `docs` or `notes/design`, or `.` for the project itself"
        ))
    };
    let mut parts = Vec::new();
    for part in Path::new(folder).components() {
        match part {
            Component::Normal(part) if part == ".git" => {
                return Err(refuse("the repository's own files are off limits"));
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => return Err(refuse("it may not go up with `..`")),
            Component::RootDir | Component::Prefix(_) => return Err(refuse("it must be inside the project")),
        }
    }
    Ok(if parts.is_empty() { ".".to_string() } else { parts.join("/") })
}

/// The folder of the project a library's documents are in, from the top
/// of the project; nothing for the project's own folder.
pub fn folder(collection: &Collection) -> &str {
    match collection.at.as_deref() {
        Some(".") => "",
        Some(folder) => folder,
        None => &collection.name,
    }
}

/// A document's file, from the top of its project.
pub fn file_of(collection: &Collection, path: &str) -> String {
    match folder(collection) {
        "" => path.to_string(),
        folder => format!("{folder}/{path}"),
    }
}

/// What a document is known by, from where it is in its library.
pub fn id_of(collection: &Collection, path: &str) -> String {
    let named = path.strip_suffix(ENDING).unwrap_or(path).replace('/', ":");
    format!("{}-{}:{named}", collection.project, collection.name)
}

/// Where in its library the document known by `id` is, if `id` is one of
/// this library's: `guides/install.md`.
pub fn path_of(collection: &Collection, id: &str) -> Option<String> {
    let named = id.strip_prefix(&format!("{}-{}:", collection.project, collection.name))?;
    let parts: Vec<&str> = named.split(':').collect();
    let sound =
        |part: &&str| !part.is_empty() && *part != "." && *part != ".." && !part.contains(['/', '\\']);
    parts.iter().all(sound).then(|| format!("{}{ENDING}", parts.join("/")))
}

/// A document as a record: `path` is where it is in its library, `nth`
/// where it comes among the others when they are listed.
pub(crate) fn read(collection: &Collection, path: &str, text: &str, at: (i64, i64), nth: usize) -> Record {
    let parts = parts(text);
    let mut data = Map::new();
    for field in &collection.fields {
        let value = match field.name.as_str() {
            "title" => json!(parts.heading.clone().unwrap_or_else(|| stem(path).to_string())),
            "body" => json!(parts.body),
            "path" => json!(path),
            name => parts
                .head
                .iter()
                .find(|line| named(line) == Some(name))
                .and_then(|line| line.split_once(':'))
                .map_or(Value::Null, |(_, written)| held(field, written)),
        };
        data.insert(field.name.clone(), value);
    }
    Record {
        id: id_of(collection, path),
        project: collection.project.clone(),
        collection: collection.name.clone(),
        number: nth as i64,
        data,
        position: nth as f64,
        // A file does not say who wrote it.
        created_by: "system".to_string(),
        created: at.0,
        updated: at.1,
    }
}

/// The text of a document's file. `was` is what the file holds now, for
/// one that is there: the lines at its head that are nobody's field here
/// are kept, and a document that began with no heading is given none for
/// being called what its file is.
pub(crate) fn text_of(collection: &Collection, record: &Record, path: &str, was: Option<&str>) -> String {
    let before = was.map(parts);
    let mut head: Vec<String> = before.as_ref().map(|before| before.head.clone()).unwrap_or_default();
    for field in collection.fields.iter().filter(|field| !OF_THE_FILE.contains(&field.name.as_str())) {
        let value = record.data.get(&field.name).cloned().unwrap_or(Value::Null);
        let at = head.iter().position(|line| named(line) == Some(field.name.as_str()));
        match (nothing(&value), at) {
            (true, Some(at)) => {
                head.remove(at);
            }
            (true, None) => {}
            (false, Some(at)) => head[at] = line_of(&field.name, &value),
            (false, None) => head.push(line_of(&field.name, &value)),
        }
    }
    let said = |name: &str| record.data.get(name).and_then(Value::as_str).unwrap_or_default().trim();
    let title = if said("title").is_empty() { stem(path) } else { said("title") };
    let headed = match &before {
        Some(before) => before.heading.is_some() || title != stem(path),
        None => true,
    };
    let mut text = String::new();
    if !head.is_empty() {
        text.push_str(&format!("---\n{}\n---\n\n", head.join("\n")));
    }
    if headed {
        text.push_str(&format!("# {title}\n"));
    }
    if !said("body").is_empty() {
        if headed {
            text.push('\n');
        }
        text.push_str(said("body"));
        text.push('\n');
    }
    text
}

impl Collection {
    /// Whether `id` is how one of this collection's records is known.
    pub fn owns(&self, id: &str) -> bool {
        if self.store == Store::Documents {
            return path_of(self, id).is_some();
        }
        if self.store == Store::Mockups {
            return crate::mockups::name_of(self, id).is_some();
        }
        id.rsplit_once('-')
            .and_then(|(_, number)| number.parse::<i64>().ok())
            .is_some_and(|number| crate::collections::record_id(&self.project, &self.name, number) == id)
    }
}

impl Collections {
    fn library_root(&self, collection: &Collection) -> Result<PathBuf> {
        let dir = self
            .projects()
            .dir_of(&collection.project)
            .ok_or_else(|| Error::not_found(format!("there is no project `{}`", collection.project)))?;
        Ok(match folder(collection) {
            "" => dir,
            folder => dir.join(folder),
        })
    }

    /// Every document of a library, as the files are now, by where they are.
    pub(crate) async fn documents(&self, collection: &Collection) -> Result<Vec<Record>> {
        let (root, about) = (self.library_root(collection)?, collection.clone());
        tokio::task::spawn_blocking(move || {
            let mut found = Vec::new();
            for (path, text, meta) in texts(&root) {
                found.push(read(&about, &path, &text, stamps(&meta), found.len() + 1));
            }
            found
        })
        .await
        .map_err(|err| Error::internal(format!("reading the documents crashed: {err}")))
    }

    /// The document known by `id`, with the library it is in, if `id` is
    /// how a document of some library here is known.
    pub(crate) async fn document(&self, id: &str) -> Result<Option<(Collection, Record)>> {
        let Some((before, _)) = id.split_once(':') else { return Ok(None) };
        // A project's name and a library's may each have dashes in them.
        for (at, _) in before.match_indices('-') {
            let (project, name) = (&before[..at], &before[at + 1..]);
            let Some(collection) = self.find(project, name).await? else { continue };
            let Some(path) = path_of(&collection, id).filter(|_| collection.store == Store::Documents) else {
                continue;
            };
            let (root, about) = (self.library_root(&collection)?, collection.clone());
            let found = tokio::task::spawn_blocking(move || {
                let file = resolve(&root, &path).ok()?;
                let meta = std::fs::metadata(&file).ok().filter(|meta| meta.is_file())?;
                let text = std::fs::read_to_string(&file).ok()?;
                Some(read(&about, &path, &text, stamps(&meta), 0))
            })
            .await
            .map_err(|err| Error::internal(format!("reading the document crashed: {err}")))?;
            return Ok(found.map(|mut record| {
                record.data.retain(|name, _| collection.field(name).is_some());
                (collection, record)
            }));
        }
        Ok(None)
    }

    /// Writes a new document: a file where its `path` says, or named by
    /// its title. One that is there already is not written over.
    pub(crate) async fn add_document(
        &self,
        collection: &Collection,
        mut data: Map<String, Value>,
        by: &Principal,
    ) -> Result<Record> {
        let path = path_for(&data)?;
        let file = file_of(collection, &path);
        if self.projects().stat(&collection.project, &file).await.is_ok() {
            return Err(Error::conflict(format!(
                "there is already a document at `{file}`; give this one another title, or say where it goes with `path`"
            )));
        }
        if collection.field("path").is_some() {
            data.insert("path".into(), json!(path));
        }
        let now = self.factory().now_ms();
        let record = Record {
            id: id_of(collection, &path),
            project: collection.project.clone(),
            collection: collection.name.clone(),
            number: 0,
            data,
            position: 0.0,
            created_by: by.actor(),
            created: now,
            updated: now,
        };
        let text = text_of(collection, &record, &path, None);
        self.projects().write(&collection.project, &file, &text, &Principal::system()).await?;
        Ok(record)
    }

    /// Writes a document that is there back, changed. It stays where it is.
    pub(crate) async fn save_document(&self, collection: &Collection, record: &Record) -> Result<()> {
        let path = path_of(collection, &record.id)
            .ok_or_else(|| Error::not_found(format!("there is no record `{}`", record.id)))?;
        let said = record.data.get("path").and_then(Value::as_str).unwrap_or_default().trim();
        if !said.is_empty() && said != path {
            return Err(Error::invalid(format!(
                "`{}` is the file `{}`, and a document is known by where its file is: it cannot be moved to `{said}` by saying so. Move the file itself; where it then is, it is another document, with a history of its own",
                record.id,
                file_of(collection, &path)
            )));
        }
        let file = file_of(collection, &path);
        let was = self.projects().read(&collection.project, &file).await.ok();
        let text = text_of(collection, record, &path, was.as_deref());
        self.projects().write(&collection.project, &file, &text, &Principal::system()).await?;
        Ok(())
    }

    pub(crate) async fn remove_document(&self, collection: &Collection, record: &Record) -> Result<()> {
        let path = path_of(collection, &record.id)
            .ok_or_else(|| Error::not_found(format!("there is no record `{}`", record.id)))?;
        self.projects().remove(&collection.project, &file_of(collection, &path), &Principal::system()).await
    }

    /// The library a file of a project is a document of, and what the
    /// document is known by, if it is one.
    pub async fn document_at(&self, project: &str, file: &str) -> Result<Option<(Collection, String)>> {
        for collection in self.list(Some(project)).await? {
            if collection.store != Store::Documents {
                continue;
            }
            let within = match folder(&collection) {
                "" => Some(file),
                folder => file.strip_prefix(folder).and_then(|rest| rest.strip_prefix('/')),
            };
            if let Some(path) = within.filter(|path| is_document(path)) {
                let id = id_of(&collection, path);
                return Ok(Some((collection, id)));
            }
        }
        Ok(None)
    }
}

/// Which of a document's fields differ between two texts of its file.
pub(crate) fn changed(collection: &Collection, path: &str, was: &str, now: &str) -> Vec<String> {
    let (was, now) = (read(collection, path, was, (0, 0), 0), read(collection, path, now, (0, 0), 0));
    collection
        .fields
        .iter()
        .map(|field| field.name.clone())
        .filter(|name| was.data.get(name) != now.data.get(name))
        .collect()
}
