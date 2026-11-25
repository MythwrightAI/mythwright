//! Which pages link to a page: read from what the pages say, as they are
//! asked for, and kept nowhere.
//!
//! A link is the page's id in a Markdown link (`[words](topic:@-topics-49)`,
//! `kernel::topics::linked`), so it is found in the bodies and summaries
//! of the other pages as they are now: a link written, changed or taken
//! out is in the next reading with nothing kept to follow it, and a page
//! renamed or moved is linked to all the same. The bodies are read in one
//! query of two cells a row: two thousand pages of 4 KB each are read and
//! looked through in about 25 ms, about what listing them takes, so
//! nothing is kept for it and no table step is added.
//!
//! Each says whether it links only through a series box, and which (by
//! the box's title): every page of a series carries the same box, and the
//! client gathers those apart from the pages that link from what they say.
//!
//! Every page is read by whoever may read records, so whoever may read the
//! page asked of may be told the title of each that links to it. Where a
//! page comes to be kept from somebody, it is left out here as well.

use kernel::topics::{Linking, Tree, linking_of, title_key};
use kernel::{Principal, Result};
use serde_json::{Value, json};

use crate::{FACTORY, Topics};
use kernel::topics::TOPICS;

/// The cells of a page a link is looked for in.
const SAYS: &[&str] = &["body", "summary"];

impl Topics {
    /// The pages that link to the page `named` names, each its id, its
    /// title, the wiki it is in (its id and title; nothing for a wiki) and,
    /// where it links only through a series box, that box's title
    /// (`series`; nothing where it links from anywhere else), by their
    /// titles, whatever their capitals. A page's links to itself are not
    /// counted.
    pub async fn linked(&self, named: &str, by: &Principal) -> Result<Vec<Value>> {
        let record = self.find(named, by).await?;
        let table = self.studio.collections.get(FACTORY, TOPICS).await?;
        let says = self.studio.collections.cells(&table, SAYS).await?;
        let linking: Vec<(&str, Linking)> = says
            .iter()
            .filter(|(id, _, _)| *id != record.id)
            .map(|(id, _, cells)| {
                let how = SAYS
                    .iter()
                    .filter_map(|field| cells.get(*field).and_then(Value::as_str))
                    .map(|text| linking_of(text, &record.id))
                    .fold(Linking::default(), Linking::and);
                (id.as_str(), how)
            })
            .filter(|(_, how)| how.elsewhere || how.series.is_some())
            .collect();
        if linking.is_empty() {
            return Ok(Vec::new());
        }
        let (pages, lines) = self.pages(&table, false).await?;
        let tree = Tree::read(&lines);
        let title = |id: &str| {
            pages.iter().find(|page| page.id == id).map(crate::tree::Page::title).unwrap_or_default()
        };
        let mut linked: Vec<Value> = linking
            .into_iter()
            .map(|(id, how)| {
                let path = tree.path(id);
                let wiki = path
                    .first()
                    .filter(|wiki| **wiki != id)
                    .map(|wiki| json!({ "id": wiki, "title": title(wiki) }));
                json!({ "id": id, "title": title(id), "wiki": wiki, "series": how.series_only() })
            })
            .collect();
        linked.sort_by_cached_key(|page| title_key(page["title"].as_str().unwrap_or_default()));
        Ok(linked)
    }
}
