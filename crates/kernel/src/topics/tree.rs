//! Topics as wikis, pages in a tree, as plain values.
//!
//! A page is a topic with the page it is under (`parent`, a field the
//! keeper alone writes) and where it stands among the pages beside it
//! (the record's own `position`, which the keeper alone writes for a
//! topic). A wiki is a page with no page above it: there is no table of
//! wikis and no folder kind. What the tree is read as, whether a page may
//! go somewhere and where it lands there are decided here, case for case
//! with the table the client reads too (`ui/src/wikicases.json`).

use std::collections::{HashMap, HashSet};

/// The field of a topic that says the page it is under, by its id.
pub const PARENT: &str = "parent";

/// A page as the tree is read from: its id, the page above it as its row
/// says, where it stands among the pages beside it, and the workspace its
/// place is of (a project's being its project's), where that is known.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub id: String,
    pub parent: Option<String>,
    pub position: f64,
    pub space: Option<String>,
}

/// The pages as one tree. A `parent` that names no page there is, and
/// every page of a ring, which no write can make, are read as having none:
/// each is a wiki, so that a reader never goes round.
pub struct Tree<'a> {
    lines: &'a [Line],
    above: HashMap<&'a str, Option<&'a str>>,
}

impl<'a> Tree<'a> {
    pub fn read(lines: &'a [Line]) -> Tree<'a> {
        let said: HashMap<&str, Option<&str>> = lines
            .iter()
            .map(|line| (line.id.as_str(), line.parent.as_deref().filter(|parent| !parent.is_empty())))
            .collect();
        let mut above = HashMap::new();
        for line in lines {
            let id = line.id.as_str();
            let parent = said[id].filter(|parent| said.contains_key(parent));
            above.insert(id, parent.filter(|_| !in_a_ring(&said, id)));
        }
        Tree { lines, above }
    }

    /// Whether there is a page `id`.
    pub fn has(&self, id: &str) -> bool {
        self.above.contains_key(id)
    }

    /// The line of `id`.
    pub fn line(&self, id: &str) -> Option<&'a Line> {
        self.lines.iter().find(|line| line.id == id)
    }

    /// The page `id` is under, as the tree is read; nothing for a wiki.
    pub fn above(&self, id: &str) -> Option<&'a str> {
        self.above.get(id).copied().flatten()
    }

    /// The pages from its wiki down to `id`, its wiki first and `id` last.
    pub fn path(&self, id: &str) -> Vec<&'a str> {
        let mut path = Vec::new();
        let mut at = self.lines.iter().find(|line| line.id == id).map(|line| line.id.as_str());
        while let Some(page) = at {
            if path.contains(&page) {
                break;
            }
            path.push(page);
            at = self.above(page);
        }
        path.reverse();
        path
    }

    /// The wiki `id` is a page of: itself where it is one.
    pub fn wiki(&self, id: &str) -> Option<&'a str> {
        self.path(id).first().copied()
    }

    /// Whether `page` is `other`, or a page under it at any depth.
    pub fn under(&self, page: &str, other: &str) -> bool {
        self.path(page).contains(&other)
    }

    /// The pages directly under `parent` (the wikis, for nothing), in the
    /// order they stand.
    pub fn beside(&self, parent: Option<&str>) -> Vec<&'a Line> {
        let mut beside: Vec<&Line> =
            self.lines.iter().filter(|line| self.above(&line.id) == parent).collect();
        beside.sort_by(|one, other| {
            one.position.total_cmp(&other.position).then_with(|| one.id.cmp(&other.id))
        });
        beside
    }

    /// `id` and every page under it, each before the pages under it and in
    /// the order they stand.
    pub fn subtree(&self, id: &str) -> Vec<&'a str> {
        let mut found = Vec::new();
        if let Some(line) = self.line(id) {
            self.gather(&line.id, &mut found);
        }
        found
    }

    fn gather(&self, id: &'a str, found: &mut Vec<&'a str>) {
        found.push(id);
        for line in self.beside(Some(id)) {
            self.gather(&line.id, found);
        }
    }
}

/// Whether following what each page says is above it from `id` comes back
/// to `id`.
fn in_a_ring(said: &HashMap<&str, Option<&str>>, id: &str) -> bool {
    let mut seen = HashSet::new();
    let mut at = said.get(id).copied().flatten();
    while let Some(page) = at {
        if page == id {
            return true;
        }
        if !seen.insert(page) {
            return false;
        }
        at = said.get(page).copied().flatten();
    }
    false
}

/// Where a page lands between two that stand at `before` and `after`
/// (either of which may be none, for an end of its level, or both, for a
/// level of its own): their middle, one less than the first, one more
/// than the last. Nothing where there is no room between them, which says
/// that the pages of that level are numbered again.
pub fn between(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    let landed = match (before, after) {
        (None, None) => 1.0,
        (Some(before), None) => before + 1.0,
        (None, Some(after)) => after - 1.0,
        (Some(before), Some(after)) => before + (after - before) / 2.0,
    };
    let fits = before.is_none_or(|before| landed > before) && after.is_none_or(|after| landed < after);
    (landed.is_finite() && fits).then_some(landed)
}

/// Why a page cannot go where it was asked to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// What was named is no page there is.
    Gone(String),
    /// It was asked under itself or a page under it.
    Itself,
    /// It was asked into a wiki of another workspace.
    Elsewhere,
    /// It was asked before a page that is not under the page it goes under.
    Beside,
}

impl Refused {
    /// The word the table of cases says it by.
    pub fn word(&self) -> &'static str {
        match self {
            Refused::Gone(_) => "gone",
            Refused::Itself => "itself",
            Refused::Elsewhere => "elsewhere",
            Refused::Beside => "beside",
        }
    }
}

/// Where a page lands, asked under a page (nothing for the top, which
/// makes it a wiki) and before another (nothing for the end of that
/// level): nothing written where it stays where it is, and otherwise each
/// page whose position is to be another with that position, the page
/// moved first.
#[derive(Clone, Debug, PartialEq)]
pub struct Landing {
    pub stays: bool,
    pub positions: Vec<(String, f64)>,
}

/// Where `page` lands under `under` before `before`, or why it cannot.
pub fn landing(
    tree: &Tree<'_>,
    page: &str,
    under: Option<&str>,
    before: Option<&str>,
) -> Result<Landing, Refused> {
    for named in [Some(page), under, before].into_iter().flatten() {
        if !tree.has(named) {
            return Err(Refused::Gone(named.to_string()));
        }
    }
    if let Some(under) = under {
        if tree.under(under, page) {
            return Err(Refused::Itself);
        }
        let space = |id: &str| tree.line(id).and_then(|line| line.space.clone());
        if space(under) != space(page) {
            return Err(Refused::Elsewhere);
        }
    }
    let stays = Landing { stays: true, positions: Vec::new() };
    if before == Some(page) {
        return if tree.above(page) == under { Ok(stays) } else { Err(Refused::Beside) };
    }
    if let Some(before) = before
        && tree.above(before) != under
    {
        return Err(Refused::Beside);
    }
    if tree.above(page) == under {
        let level = tree.beside(under);
        let at = level.iter().position(|line| line.id == page);
        let next = at.and_then(|at| level.get(at + 1)).map(|line| line.id.as_str());
        if next == before {
            return Ok(stays);
        }
    }
    let level: Vec<&Line> = tree.beside(under).into_iter().filter(|line| line.id != page).collect();
    let at = before.and_then(|before| level.iter().position(|line| line.id == before)).unwrap_or(level.len());
    let previous = at.checked_sub(1).map(|at| level[at].position);
    let next = level.get(at).map(|line| line.position);
    if let Some(position) = between(previous, next) {
        return Ok(Landing { stays: false, positions: vec![(page.to_string(), position)] });
    }
    let mut order: Vec<&str> = level.iter().map(|line| line.id.as_str()).collect();
    order.insert(at, page);
    let mut positions = vec![(page.to_string(), (at + 1) as f64)];
    for (nth, id) in order.iter().enumerate() {
        let position = (nth + 1) as f64;
        if *id != page && tree.line(id).is_some_and(|line| line.position != position) {
            positions.push((id.to_string(), position));
        }
    }
    Ok(Landing { stays: false, positions })
}
