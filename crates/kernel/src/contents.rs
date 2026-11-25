//! A library's contents as a reader is led through them: each folder's
//! documents and folders in the order its front page says, and those it
//! does not say after them by title. Functions of plain values, which
//! `studio::site` asks where a library is written out as pages and where
//! it is served as pages, so that both are in the one order.

use std::collections::{BTreeMap, BTreeSet};

/// A document is a Markdown file, and is named in an order with or
/// without its ending.
const ENDING: &str = ".md";

/// The setting of a front page's head that says the order of its folder.
pub const ORDER: &str = "order";

/// A document of a library: where it is, from the library's folder, and
/// what it is called (nothing where it says nothing, when it is called
/// what its file is).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub path: String,
    pub title: String,
}

/// A place in the contents: a document, or a folder with what is in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    /// From the library's folder: a document's file, or a folder ending
    /// in `/`.
    pub path: String,
    pub title: String,
    /// What is in a folder, in its order; nothing for a document.
    pub under: Vec<Place>,
}

impl Place {
    pub fn is_folder(&self) -> bool {
        self.path.ends_with('/')
    }

    /// Its own name, the last of its path.
    fn name(&self) -> &str {
        let path = self.path.trim_end_matches('/');
        path.rsplit('/').next().unwrap_or(path)
    }
}

/// The order the lines of a front page's head say (`order:`), each thing
/// as it is written: a list in brackets, the words with commas between,
/// or a line for each beginning with `-` under it. Nothing where it says
/// none.
pub fn order_in(head: &[String]) -> Vec<String> {
    let bare = |word: &str| word.trim().trim_matches(['"', '\'']).trim().to_string();
    let Some(at) =
        head.iter().position(|line| line.split_once(':').is_some_and(|(name, _)| name.trim() == ORDER))
    else {
        return Vec::new();
    };
    let (_, said) = head[at].split_once(':').unwrap_or_default();
    let said = said.trim();
    if !said.is_empty() {
        let inner = said.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or(said);
        return inner.split(',').map(bare).filter(|word| !word.is_empty()).collect();
    }
    head[at + 1..]
        .iter()
        .map_while(|line| line.trim_start().strip_prefix('-'))
        .map(bare)
        .filter(|word| !word.is_empty())
        .collect()
}

/// Whether a thing an order says is `place`: a document by its file's name
/// with or without its ending, a folder by its name with or without a `/`
/// after it. A name with a `/` after it is only ever a folder's, and one
/// that ends as a document does only ever a document's.
fn says(said: &str, place: &Place) -> bool {
    let said = said.strip_prefix("./").unwrap_or(said);
    let name = place.name();
    match place.is_folder() {
        true => said.strip_suffix('/').unwrap_or(said) == name && !said.ends_with(ENDING),
        false => !said.ends_with('/') && (said == name || format!("{said}{ENDING}") == name),
    }
}

/// The places of one folder in its order: those the order says first, as
/// it says them, a thing said that is not there or said again passed over;
/// then the rest, the documents before the folders, each by its title
/// (whatever its letters are written in), and by its path where two are
/// called the same. A word that names both a document and a folder is the
/// document.
pub fn ordered(order: &[String], mut places: Vec<Place>) -> Vec<Place> {
    let mut first = Vec::with_capacity(places.len());
    for said in order {
        let at = places
            .iter()
            .position(|place| !place.is_folder() && says(said, place))
            .or_else(|| places.iter().position(|place| place.is_folder() && says(said, place)));
        if let Some(at) = at {
            first.push(places.remove(at));
        }
    }
    places.sort_by(|a, b| {
        (a.is_folder(), a.title.to_lowercase(), &a.path).cmp(&(
            b.is_folder(),
            b.title.to_lowercase(),
            &b.path,
        ))
    });
    first.extend(places);
    first
}

/// The contents of a library: every folder a document is in, each with
/// its documents and folders in its order. `fronts` says, of a folder (by
/// its path ending in `/`, and the library's own as nothing), which of its
/// documents is its front page: that one stands for the folder, is not
/// listed in it, and gives it its title; a folder with none is called its
/// name. `orders` is what each folder's front page says of its order.
pub fn contents(
    documents: &[Document],
    fronts: &BTreeMap<String, String>,
    orders: &BTreeMap<String, Vec<String>>,
) -> Vec<Place> {
    folder("", documents, fronts, orders)
}

fn folder(
    within: &str,
    documents: &[Document],
    fronts: &BTreeMap<String, String>,
    orders: &BTreeMap<String, Vec<String>>,
) -> Vec<Place> {
    let front = fronts.get(within);
    let mut places = Vec::new();
    let mut folders = BTreeSet::new();
    for document in documents.iter().filter(|document| document.path.starts_with(within)) {
        match document.path[within.len()..].split_once('/') {
            Some((name, _)) => drop(folders.insert(name.to_string())),
            None if front == Some(&document.path) => {}
            None => {
                places.push(Place { path: document.path.clone(), title: called(document), under: Vec::new() })
            }
        }
    }
    for name in folders {
        let path = format!("{within}{name}/");
        let title = fronts
            .get(&path)
            .and_then(|front| documents.iter().find(|document| &document.path == front))
            .map_or_else(|| name.clone(), called);
        let under = folder(&path, documents, fronts, orders);
        places.push(Place { path, title, under });
    }
    ordered(orders.get(within).map_or(&[], Vec::as_slice), places)
}

/// What a document is called: its title, or its file's name where it
/// says none.
fn called(document: &Document) -> String {
    match document.title.trim() {
        "" => {
            let name = document.path.rsplit('/').next().unwrap_or(&document.path);
            name.strip_suffix(ENDING).unwrap_or(name).to_string()
        }
        title => title.to_string(),
    }
}

/// Every document of the contents in the order a reader is led through
/// them, each folder's front page where its folder is: the order a list of
/// the documents goes in.
pub fn in_order(contents: &[Place], fronts: &BTreeMap<String, String>) -> Vec<String> {
    let mut found: Vec<String> = fronts.get("").cloned().into_iter().collect();
    fn walk(places: &[Place], fronts: &BTreeMap<String, String>, found: &mut Vec<String>) {
        for place in places {
            match place.is_folder() {
                true => {
                    found.extend(fronts.get(&place.path).cloned());
                    walk(&place.under, fronts, found);
                }
                false => found.push(place.path.clone()),
            }
        }
    }
    walk(contents, fronts, &mut found);
    found
}

/// Which place of the contents a page is: a document's own, or for a
/// folder's front page the folder (the library's own as nothing).
pub fn here_of(path: &str, fronts: &BTreeMap<String, String>) -> String {
    fronts
        .iter()
        .find(|(_, front)| *front == path)
        .map_or_else(|| path.to_string(), |(folder, _)| folder.clone())
}
