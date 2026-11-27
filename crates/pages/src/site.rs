//! A library as pages: the ones the daemon serves, and the same written
//! out as plain files that anything can serve.
//!
//! A page is made one way ([`crate::made`]). What differs between a page
//! that is served and one that is written out is only how what it links to
//! is reached: the daemon answers for a document at the address of its
//! file, while a folder of files has nothing to answer and every link in
//! it has to lead to a file that is there. So what is shared is here: what
//! a front page is, the line that leads back to it, the list of a folder,
//! and the library's contents beside every page ([`Shelf`],
//! [`navigation`]): each folder's documents and folders in the order its
//! front page says in its head (`order:`), those it does not say after
//! them by title (`kernel::contents`, with its cases), the page showing
//! marked, and nothing in it but links.
//!
//! **Writing a library out** ([`build`], [`put`]; `studio::site::export`,
//! which asks who may and says what was written) fails closed. Only what was
//! asked for is written: the documents that say `public: true` of
//! themselves, unless all of them are asked for. A link from a page that
//! is written to something that is not (a document that is not published,
//! a file that is not there, somewhere outside the library) is written as
//! the words it was, and is named in the answer. A picture or a file is
//! copied only where a page that is written uses it. Nothing in what is
//! written runs, and nothing in it is fetched from anywhere else: each
//! page says so of itself, for whatever serves it to hold to.
//!
//! The folder is written whole or not at all: built beside where it is to
//! be, and put in its place when it is done. What an export wrote it lists
//! in the folder ([`WRITTEN`]), and that list is how the next one tells
//! what is its own to replace: a folder holding anything else is left
//! exactly as it is, and the export refused.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::SystemTime;

use kernel::contents::{self, Document, Place};
use kernel::{Error, Result};
use serde::Serialize;

pub use kernel::addressed::written;
use kernel::addressed::{between, folder_of, joined, saying, scheme, unwritten};

use projects::{MAX_RAW_BYTES, resolve};

use crate::document::{ENDING, head, setting, texts};
use crate::made::{self as pages, Leads};

/// What a document is called, from where it is in its library and its
/// text: what the library it is a record of reads its title as.
pub type Titled<'a> = &'a dyn Fn(&str, &str) -> String;

/// The files a front page is, where a folder has one, in the order the
/// daemon looks for them.
pub const FRONTS: &[&str] = &["README.md", "index.md", "readme.md"];

/// The files a folder's front page is where a library is written out: a
/// document called `index` is written as its folder's page, so it is that.
const WRITTEN_FRONTS: &[&str] = &["index.md", "README.md", "readme.md"];

/// A page that is written out says what it may do: show itself, with its
/// pictures and its style sheet from beside it, and nothing more. No
/// script runs in it and nothing is fetched from anywhere else. `file:`
/// is for a page opened straight from a disk, where a browser takes no
/// two files to be from the same place; served from an address it adds
/// nothing, since a page that was fetched cannot reach a disk.
const STANDING: &str = "default-src 'none'; img-src 'self' data: file:; style-src 'self' file:; base-uri 'none'; form-action 'none'";

/// The one style sheet of a library that is written out, at its top.
const SHEET: &str = "page.css";

/// The list an export keeps, in the folder, of every file it wrote there.
pub const WRITTEN: &str = ".exported";

/// The kinds of file that are copied where a page uses one: pictures, and
/// what is read or kept and never run. Anything else is left where it is.
const COPIED: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico", "svg", "pdf", "txt", "csv", "json", "toml",
    "yaml", "yml",
];

/// The line at the head of a page that leads back to the library's front,
/// which is at `to` from where the page is.
pub fn way_up(called: &str, to: &str) -> String {
    format!("<p class=\"page-up\"><a href=\"{}\">{}</a></p>\n", pages::escape(to), pages::escape(called))
}

/// A document as a folder's page lists it: where it is in the library,
/// and what it is called.
pub struct Listed {
    pub path: String,
    pub title: String,
}

/// The page of a folder that has no front page of its own, from its
/// heading down: the documents in it and under it, by folder. `within` is
/// the folder from the library's, ending in `/` (nothing for the library
/// itself), and `lead` says how a document is linked to from this page,
/// given where it is from this folder.
pub fn listing(heading: &str, within: &str, documents: &[Listed], lead: &dyn Fn(&str) -> String) -> String {
    let mut inside = format!("<h1>{}</h1>\n", pages::escape(heading));
    let mut last: Option<String> = None;
    let mut open = false;
    for document in documents.iter().filter(|document| document.path.starts_with(within)) {
        let here = &document.path[within.len()..];
        let folder = here.rsplit_once('/').map(|(folder, _)| folder.to_string()).unwrap_or_default();
        if last.as_ref() != Some(&folder) {
            if open {
                inside.push_str("</ul>\n");
            }
            if !folder.is_empty() {
                inside.push_str(&format!("<h2>{}</h2>\n", pages::escape(&folder)));
            }
            inside.push_str("<ul>\n");
            open = true;
            last = Some(folder);
        }
        let title = if document.title.is_empty() { here } else { &document.title };
        inside.push_str(&format!("<li><a href=\"{}\">{}</a></li>\n", lead(here), pages::escape(title)));
    }
    if open {
        inside.push_str("</ul>\n");
    } else {
        inside.push_str("<p>There are no documents here yet.</p>\n");
    }
    inside
}

/// What a library holds as a reader is led through it: its contents in
/// order, and the document that is each folder's front page (by the
/// folder's path ending in `/`, the library's own as nothing).
pub struct Shelf {
    pub contents: Vec<Place>,
    pub fronts: BTreeMap<String, String>,
}

impl Shelf {
    /// The shelf of `documents` (where each is in the library, and its
    /// text), a folder's front page being the first of `fronts` it has,
    /// each called what `title` says.
    fn of(title: Titled<'_>, documents: &[(&str, &str)], fronts: &[&str]) -> Shelf {
        let mut folders = BTreeSet::from([String::new()]);
        for (path, _) in documents {
            let mut at = folder_of(path);
            while !at.is_empty() {
                folders.insert(format!("{at}/"));
                at = folder_of(at);
            }
        }
        let (mut fronted, mut orders) = (BTreeMap::new(), BTreeMap::new());
        for within in folders {
            let front = fronts.iter().find_map(|front| {
                let path = format!("{within}{front}");
                documents.iter().find(|(there, _)| *there == path)
            });
            if let Some((path, text)) = front {
                orders.insert(within.clone(), contents::order_in(&head(text)));
                fronted.insert(within, path.to_string());
            }
        }
        let listed: Vec<Document> = documents
            .iter()
            .map(|(path, text)| Document { path: path.to_string(), title: title(path, text) })
            .collect();
        Shelf { contents: contents::contents(&listed, &fronted, &orders), fronts: fronted }
    }

    /// Which place a page of the document at `path` is: its own, or its
    /// folder's where it is a front page.
    pub fn here(&self, path: &str) -> String {
        contents::here_of(path, &self.fronts)
    }
}

/// The shelf of every document of a library as its files are now, as the
/// daemon serves it: a folder's front page as [`FRONTS`] says.
pub fn shelf_from(root: &Path, title: Titled<'_>) -> Shelf {
    let read_now = texts(root);
    let documents: Vec<(&str, &str)> =
        read_now.iter().map(|(path, text, _)| (path.as_str(), text.as_str())).collect();
    Shelf::of(title, &documents, FRONTS)
}

/// At the head of a page beside its contents: the way to them, for where
/// they are under the page (a phone's width) and not beside it.
pub const TO_CONTENTS: &str = "<p class=\"page-to-nav\"><a href=\"#contents\">Contents</a></p>\n";

/// The contents of a library beside one of its pages: the library's front
/// at `top`, then its folders and documents in their order, each reached
/// as `lead` says from the page, and the place the page is (`here`)
/// marked. Links and nothing else.
pub fn navigation(
    called: &str,
    top: &str,
    shelf: &Shelf,
    here: &str,
    lead: &dyn Fn(&Place) -> String,
) -> String {
    let current = |path: &str| if path == here { " aria-current=\"page\"" } else { "" };
    let mut out = format!(
        "<nav class=\"page-nav\" id=\"contents\" aria-label=\"Contents\">\n<p class=\"page-nav-top\"><a href=\"{}\"{}>{}</a></p>\n",
        pages::escape(top),
        current(""),
        pages::escape(called)
    );
    fn list(places: &[Place], out: &mut String, link: &dyn Fn(&Place) -> String) {
        out.push_str("<ul>\n");
        for place in places {
            out.push_str("<li>");
            out.push_str(&link(place));
            if !place.under.is_empty() {
                out.push('\n');
                list(&place.under, out, link);
            }
            out.push_str("</li>\n");
        }
        out.push_str("</ul>\n");
    }
    let link = |place: &Place| {
        let class = if place.is_folder() { " class=\"page-nav-folder\"" } else { "" };
        format!(
            "<a href=\"{}\"{class}{}>{}</a>",
            pages::escape(&lead(place)),
            current(&place.path),
            pages::escape(&place.title)
        )
    };
    if !shelf.contents.is_empty() {
        list(&shelf.contents, &mut out, &link);
    }
    out.push_str("</nav>\n");
    out
}

/// The contents beside a page the daemon serves, which is at the address
/// of its file, a folder's at the folder's (`page` is the path the page is
/// at from the library's, a folder's ending in `/`). Each link says `said`
/// in its address as well, as the page was asked (`addressed::saying`).
pub fn served_navigation(called: &str, shelf: &Shelf, page: &str, here: &str, said: &str) -> String {
    let from = page.rsplit_once('/').map_or("", |(folder, _)| folder);
    let deep = page.matches('/').count();
    let top = if deep == 0 { "./".to_string() } else { "../".repeat(deep) };
    let lead = |place: &Place| match between(from, &place.path) {
        empty if empty.is_empty() => saying("./", said),
        to => saying(&to, said),
    };
    navigation(called, &saying(&top, said), shelf, here, &lead)
}

/// A link or a picture of a page that was written as its words, and why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LeftOut {
    /// The document it is in, from the library's folder.
    pub page: String,
    /// What it led to, as it is written in the document.
    pub link: String,
    pub why: String,
}

/// What came of writing a library out.
#[derive(Clone, Debug, Serialize)]
pub struct Exported {
    /// The folder it was written to, from the top of the project.
    pub to: String,
    /// The library's own folder, from the top of the project.
    pub from: String,
    /// How many pages there are: a document each, and a front page for
    /// each folder that has one.
    pub pages: usize,
    /// How many other files: the style sheet, and what the pages use.
    pub files: usize,
    /// Whether anything was written. What was there already and is what
    /// would have been written is left as it is.
    pub changed: bool,
    pub left_out: Vec<LeftOut>,
}

/// The page a document is written as.
fn page_of(path: &str) -> String {
    format!("{}.html", path.strip_suffix(ENDING).unwrap_or(path))
}

/// A library being made into the files of a site.
struct Site<'a> {
    root: &'a Path,
    called: &'a str,
    /// Every document there is, and whether it is to be written.
    documents: BTreeMap<String, bool>,
    /// Every folder that has a page: one with something written in or under it.
    folders: BTreeSet<String>,
    /// What the pages use, copied: where it goes, and what it is.
    copied: BTreeMap<String, Vec<u8>>,
    left_out: Vec<LeftOut>,
}

impl Site<'_> {
    fn leave(&mut self, page: &str, link: &str, why: &str) -> Leads {
        let left = LeftOut { page: page.to_string(), link: link.to_string(), why: why.to_string() };
        if !self.left_out.contains(&left) {
            self.left_out.push(left);
        }
        Leads::Nowhere
    }

    /// Where a link or a picture of the document at `page` leads from the
    /// page it is written as, which is in the same folder.
    fn leads(&mut self, page: &str, picture: bool, dest: &str) -> Leads {
        let from = folder_of(page);
        if dest.is_empty() || dest.starts_with('#') {
            return Leads::To(dest.to_string());
        }
        if dest.starts_with("//") || scheme(dest).is_some() {
            let known = scheme(dest).map(str::to_ascii_lowercase);
            return match (picture, known.as_deref()) {
                (true, Some("data")) => Leads::To(dest.to_string()),
                (true, _) => self.leave(page, dest, "a picture kept somewhere else is not fetched"),
                (false, None | Some("http" | "https" | "mailto" | "tel")) => Leads::To(dest.to_string()),
                (false, Some(_)) => self.leave(page, dest, "a link of that kind is not written out"),
            };
        }
        if dest.starts_with('/') {
            return self.leave(
                page,
                dest,
                "it is written from the top of an address, which a folder of pages does not have",
            );
        }
        // What follows the path (`#part`, `?asked`) goes with it as it is.
        let end = dest.find(['#', '?']).unwrap_or(dest.len());
        let (path, rest) = (unwritten(&dest[..end]), &dest[end..]);
        let Some(target) = joined(from, &path) else {
            return self.leave(page, dest, "it leads out of the library");
        };
        if path.ends_with('/') || target.is_empty() || self.folders.contains(&target) {
            if picture || !self.folders.contains(&target) {
                return self.leave(page, dest, "nothing in that folder is written out");
            }
            let index =
                if target.is_empty() { "index.html".to_string() } else { format!("{target}/index.html") };
            return Leads::To(format!("{}{rest}", between(from, &index)));
        }
        if target.ends_with(ENDING) {
            return match (picture, self.documents.get(&target)) {
                (false, Some(true)) => Leads::To(format!("{}{rest}", between(from, &page_of(&target)))),
                (false, Some(false)) => self.leave(page, dest, "that document is not published"),
                _ => self.leave(page, dest, "there is no such document"),
            };
        }
        match self.copy(&target) {
            Ok(()) => Leads::To(format!("{}{rest}", between(from, &target))),
            Err(why) => self.leave(page, dest, why),
        }
    }

    /// Takes a file of the library that a page uses into what is written.
    fn copy(&mut self, target: &str) -> std::result::Result<(), &'static str> {
        if self.copied.contains_key(target) {
            return Ok(());
        }
        let ending = target.rsplit('.').next().unwrap_or_default().to_ascii_lowercase();
        if !COPIED.contains(&ending.as_str()) {
            return Err("a file of that kind is not written out");
        }
        // Through `resolve`, as every file of a library is read: a link in
        // the folder that leads out of it is not a file of it.
        let file = resolve(self.root, target).map_err(|_| "it leads out of the library")?;
        let meta = std::fs::metadata(&file).map_err(|_| "there is no such file")?;
        if !meta.is_file() {
            return Err("there is no such file");
        }
        if meta.len() > MAX_RAW_BYTES {
            return Err("that file is too large to write out");
        }
        let bytes = std::fs::read(&file).map_err(|_| "that file could not be read")?;
        // A drawing of this kind can hold script, which runs where the
        // drawing is opened by itself. One that holds any is left out.
        if ending == "svg" {
            let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
            let on = text.split(|c: char| c.is_whitespace() || c == '"' || c == '\'').any(|word| {
                word.strip_prefix("on").and_then(|rest| rest.split_once('=')).is_some_and(|(name, _)| {
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic())
                })
            });
            if on || text.contains("<script") || text.contains("javascript:") {
                return Err("that drawing holds script, and nothing that is written out may run");
            }
        }
        self.copied.insert(target.to_string(), bytes);
        Ok(())
    }

    /// A document as the page at `at` (from the top of what is written),
    /// with the library's contents beside it.
    fn document(&mut self, path: &str, text: &str, at: &str, shelf: &Shelf) -> String {
        let (deep, called) = (at.matches('/').count(), self.called);
        let name = path.rsplit('/').next().unwrap_or(path);
        let fallback = match at {
            "index.html" => called,
            _ => name.strip_suffix(ENDING).unwrap_or(name),
        };
        let title = pages::title(text).unwrap_or_else(|| fallback.to_string());
        let up = match at {
            "index.html" => String::new(),
            _ => way_up(called, &format!("{}index.html", "../".repeat(deep))),
        };
        let body = pages::body_with(text, &mut |picture, dest| self.leads(path, picture, dest));
        let beside = written_navigation(called, shelf, at, &shelf.here(path));
        let sheet = format!("{}{SHEET}", "../".repeat(deep));
        pages::standing(&title, &sheet, STANDING, &format!("{TO_CONTENTS}{up}{body}"), &beside)
    }
}

/// The contents beside a page that is written out at `at` (from the top
/// of what is written): a document is its page, a folder its `index.html`.
fn written_navigation(called: &str, shelf: &Shelf, at: &str, here: &str) -> String {
    let from = folder_of(at);
    let lead = |place: &Place| match place.is_folder() {
        true => between(from, &format!("{}index.html", place.path)),
        false => between(from, &page_of(&place.path)),
    };
    navigation(called, &between(from, "index.html"), shelf, here, &lead)
}

/// Everything a library is when it is written out: each file, by where it
/// goes, with how many of them are pages and what was left out.
pub struct Built {
    pub files: BTreeMap<String, Vec<u8>>,
    pub pages: usize,
    pub left_out: Vec<LeftOut>,
}

pub fn build(root: &Path, called: &str, all: bool, title: Titled<'_>) -> Built {
    let read_now = texts(root);
    let public =
        |text: &str| all || setting(text, "public").is_some_and(|said| said.eq_ignore_ascii_case("true"));
    let documents: BTreeMap<String, bool> =
        read_now.iter().map(|(path, text, _)| (path.clone(), public(text))).collect();
    let mut folders = BTreeSet::from([String::new()]);
    for path in documents.iter().filter(|(_, published)| **published).map(|(path, _)| path) {
        let mut at = folder_of(path);
        while !at.is_empty() {
            folders.insert(at.to_string());
            at = folder_of(at);
        }
    }
    let mut site = Site { root, called, documents, folders, copied: BTreeMap::new(), left_out: Vec::new() };
    let published: Vec<(&str, &str)> = read_now
        .iter()
        .filter(|(path, _, _)| site.documents.get(path) == Some(&true))
        .map(|(path, text, _)| (path.as_str(), text.as_str()))
        .collect();
    let shelf = Shelf::of(title, &published, WRITTEN_FRONTS);
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for &(path, text) in &published {
        let at = page_of(path);
        let page = site.document(path, text, &at, &shelf);
        files.insert(at, page.into_bytes());
    }
    // A folder's list is in the order its contents are.
    let order = contents::in_order(&shelf.contents, &shelf.fronts);
    let place = |path: &str| order.iter().position(|one| one == path).unwrap_or(usize::MAX);
    let mut published = published;
    published.sort_by_key(|(path, _)| place(path));
    let listed: Vec<Listed> = published
        .iter()
        .map(|&(path, text)| Listed { path: path.to_string(), title: title(path, text) })
        .collect();
    for folder in site.folders.clone() {
        let within = if folder.is_empty() { String::new() } else { format!("{folder}/") };
        let index = format!("{within}index.html");
        // A document called `index` is its folder's page already.
        if files.contains_key(&index) {
            continue;
        }
        let front = FRONTS
            .iter()
            .map(|front| format!("{within}{front}"))
            .find_map(|front| published.iter().copied().find(|(path, _)| *path == front));
        let page = match front {
            Some((path, text)) => site.document(path, text, &index, &shelf),
            None => {
                let deep = index.matches('/').count();
                let heading = if folder.is_empty() { called } else { folder.as_str() };
                let up = match deep {
                    0 => String::new(),
                    _ => way_up(called, &format!("{}index.html", "../".repeat(deep))),
                };
                let lead = |here: &str| written(&page_of(here));
                let inside = format!("{TO_CONTENTS}{up}{}", listing(heading, &within, &listed, &lead));
                let beside = written_navigation(called, &shelf, &index, &within);
                let sheet = format!("{}{SHEET}", "../".repeat(deep));
                pages::standing(heading, &sheet, STANDING, &inside, &beside)
            }
        };
        files.insert(index, page.into_bytes());
    }
    let pages = files.len();
    files.insert(SHEET.to_string(), pages::STYLE.as_bytes().to_vec());
    files.extend(std::mem::take(&mut site.copied));
    Built { files, pages, left_out: site.left_out }
}

/// Every file under `dir`, by where it is from there, with when it was
/// last written; and the first thing found that is neither a file nor a
/// folder (a link, say), which nothing here made.
fn held(dir: &Path) -> std::io::Result<(BTreeMap<String, SystemTime>, Option<String>)> {
    let (mut files, mut odd) = (BTreeMap::new(), None);
    let mut pending = vec![(dir.to_path_buf(), String::new())];
    while let Some((folder, within)) = pending.pop() {
        for item in std::fs::read_dir(&folder)? {
            let item = item?;
            let name = format!("{within}{}", item.file_name().to_string_lossy());
            let meta = item.path().symlink_metadata()?;
            if meta.is_dir() {
                pending.push((item.path(), format!("{name}/")));
            } else if meta.is_file() {
                files.insert(name, meta.modified().unwrap_or(SystemTime::UNIX_EPOCH));
            } else {
                odd.get_or_insert(name);
            }
        }
    }
    Ok((files, odd))
}

/// Takes away a folder left beside by an export that did not finish. It is
/// one only if it is empty or holds an export's own list.
fn clear(left: &Path, to: &str) -> Result<()> {
    let Ok(meta) = left.symlink_metadata() else { return Ok(()) };
    let ours = meta.is_dir()
        && (left.join(WRITTEN).is_file()
            || std::fs::read_dir(left).is_ok_and(|mut inside| inside.next().is_none()));
    if !ours {
        return Err(Error::conflict(format!(
            "`{}` is in the way of writing `{to}` out, and no export put it there. Move it, and ask again",
            left.display()
        )));
    }
    Ok(std::fs::remove_dir_all(left)?)
}

/// Puts `files` where `to` says, under the folder `dir` (a project's), as
/// a whole: built beside it, and put in its place when done, and only
/// over a folder an export wrote ([`WRITTEN`]). Says whether anything was
/// written. The pages the program writes out of itself are put the same
/// way (`api::documented`).
pub fn put(dir: &Path, to: &str, files: &BTreeMap<String, Vec<u8>>) -> Result<bool> {
    let target = resolve(dir, to)?;
    let name = target.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let parent = target.parent().map(Path::to_path_buf).unwrap_or_else(|| dir.to_path_buf());
    let mut list: Vec<&str> = files.keys().map(String::as_str).collect();
    list.sort_unstable();
    let list = format!("{}\n", list.join("\n"));
    let mut wanted: BTreeMap<&str, &[u8]> =
        files.iter().map(|(path, bytes)| (path.as_str(), bytes.as_slice())).collect();
    wanted.insert(WRITTEN, list.as_bytes());

    // What is there now, which has to be an export's own to be replaced.
    let mut same: BTreeMap<String, SystemTime> = BTreeMap::new();
    let there = target.symlink_metadata().ok();
    if let Some(meta) = &there {
        if !meta.is_dir() {
            return Err(Error::conflict(format!(
                "`{to}` is not a folder, so nothing can be written out to it. Say another folder"
            )));
        }
        let (now, odd) = held(&target)?;
        let wrote: BTreeSet<String> = std::fs::read_to_string(target.join(WRITTEN))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect();
        let strange =
            odd.or_else(|| now.keys().find(|path| *path != WRITTEN && !wrote.contains(*path)).cloned());
        if let Some(strange) = strange {
            return Err(Error::conflict(format!(
                "`{to}` holds `{strange}`, which no export put there (what one wrote is listed in `{to}/{WRITTEN}`), and writing it out again would take it away. Move it out, or say another folder"
            )));
        }
        for (path, at) in now {
            let kept = std::fs::read(target.join(&path)).ok();
            if wanted.get(path.as_str()).is_some_and(|bytes| kept.as_deref() == Some(*bytes)) {
                same.insert(path, at);
            }
        }
        // All of it is there as it would be written, and nothing else is
        // (anything more would be in the list, which would then differ).
        if same.len() == wanted.len() {
            return Ok(false);
        }
    }

    let (beside, was) = (parent.join(format!(".{name}.writing")), parent.join(format!(".{name}.was")));
    clear(&beside, to)?;
    clear(&was, to)?;
    std::fs::create_dir_all(&beside)?;
    let done = (|| -> std::io::Result<()> {
        for (path, bytes) in &wanted {
            let file = beside.join(path);
            if let Some(folder) = file.parent() {
                std::fs::create_dir_all(folder)?;
            }
            std::fs::write(&file, bytes)?;
            // A file that is what it was keeps when it was written, so
            // that nothing looking at the folder takes it for changed.
            if let Some(at) = same.get(*path) {
                std::fs::File::options().write(true).open(&file)?.set_modified(*at)?;
            }
        }
        if there.is_some() {
            std::fs::rename(&target, &was)?;
        }
        if let Err(err) = std::fs::rename(&beside, &target) {
            // What was there is put back: it is the old pages or none.
            let _ = std::fs::rename(&was, &target);
            return Err(err);
        }
        Ok(())
    })();
    if let Err(err) = done {
        let _ = std::fs::remove_dir_all(&beside);
        return Err(Error::from(err).context(format!("writing `{to}` out")));
    }
    let _ = std::fs::remove_dir_all(&was);
    Ok(true)
}
