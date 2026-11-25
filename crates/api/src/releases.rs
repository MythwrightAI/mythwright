//! The notes of each release, which came with the program.
//!
//! They are one Markdown file of the repository (`docs/releases.md`),
//! built in: a copy that was installed has them with no checkout beside
//! it and no network, and nothing is ever fetched to say what is new. A
//! release is a heading of the second level there. Whatever lists the
//! releases reads those headings, so a release is written once, where its
//! notes are.

/// The notes as they were when this program was built.
pub const NOTES: &str = manual::RELEASES;

/// One release: what its heading says, and where on the page of the
/// notes it begins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub title: String,
    /// The id of its part of the page ([`page`]).
    pub at: String,
}

/// What stands before the first release, and then each release: what its
/// heading says and what is written under it. A line that only looks
/// like a heading, inside a fenced block of code, is not one.
fn parts(notes: &str) -> (String, Vec<(String, String)>) {
    let mut before = String::new();
    let mut each: Vec<(String, String)> = Vec::new();
    let mut fenced = false;
    for line in notes.lines() {
        let begins = line.trim_start();
        if begins.starts_with("```") || begins.starts_with("~~~") {
            fenced = !fenced;
        }
        match line.strip_prefix("## ").filter(|_| !fenced) {
            Some(title) => each.push((title.trim().to_string(), String::new())),
            None => {
                let under = each.last_mut().map_or(&mut before, |(_, under)| under);
                under.push_str(line);
                under.push('\n');
            }
        }
    }
    (before, each)
}

fn at(place: usize) -> String {
    format!("release-{}", place + 1)
}

/// The releases `notes` tells of, in the order they are written there:
/// the newest first, as the file is kept.
pub fn releases(notes: &str) -> Vec<Release> {
    let (_, each) = parts(notes);
    each.into_iter().enumerate().map(|(place, (title, _))| Release { title, at: at(place) }).collect()
}

/// The notes as a page, made as any document's page is
/// (`studio::pages`), with each release in a part of its own that is
/// known by where [`releases`] says it is.
pub fn page(notes: &str) -> String {
    let (before, each) = parts(notes);
    let mut inside = studio::pages::body(&before);
    for (place, (title, under)) in each.iter().enumerate() {
        let body = studio::pages::body(&format!("## {title}\n{under}"));
        inside.push_str(&format!("<section id=\"{}\">\n{body}</section>\n", at(place)));
    }
    let title = studio::pages::title(notes).unwrap_or_else(|| "Releases".to_string());
    studio::pages::shell(&title, &inside)
}
