//! Topics as wikis, pages in a tree: a page made under another,
//! a page moved, a wiki put somewhere else with every page under it, and
//! every page brought to its wiki's place.
//!
//! What the tree is read as and where a page lands are the kernel's
//! (`kernel::topics::tree`); what is done here is the reading and the one
//! write. **Every page is of its wiki's place**, written so as it is made
//! under a page, as it is moved and as its wiki is put somewhere else, so
//! that whatever reads a topic's place (whose an event is, the panel's
//! workspace) is right for a page with nothing walked.
//!
//! **One turn arranges the topics** (`ARRANGING`, a turn of the table's
//! records' own): two moves that each look fine alone can make a ring
//! together, which a turn by record cannot see. Within it each record that
//! is written is held by its own turn, as any change of a record is, so
//! that a change of what a page says made meanwhile is not written over
//! with what it held before the move.

use std::collections::{BTreeMap, HashMap};

use kernel::topics::{Landing, Line, PARENT, Place, Refused, TOPICS, Tree};
use kernel::{Error, Principal, Result, Source};
use serde_json::{Map, Value, json};
use studio::collections::Say;
use studio::{Collection, Record};

use crate::{FACTORY, Topics, revised, said_place, shown};

/// The turn every change of the topics' tree takes, among the turns the
/// table's records are changed under.
pub(crate) const ARRANGING: &str = "the tree of the factory's topics";

/// The cells of a topic the tree is read from, and nothing of what it says.
const CELLS: &[&str] = &["title", "status", PARENT, "workspace", "project"];

/// A page as the tree reads it.
pub(crate) struct Page {
    pub id: String,
    pub position: f64,
    pub cells: Map<String, Value>,
}

impl Page {
    fn text(&self, field: &str) -> Option<String> {
        self.cells.get(field).and_then(Value::as_str).filter(|said| !said.is_empty()).map(str::to_string)
    }

    pub fn title(&self) -> String {
        self.text("title").unwrap_or_default()
    }

    fn place(&self) -> Option<Place> {
        Place::of(&self.cells)
    }
}

/// What a write of the tree changes of one page.
#[derive(Default)]
struct Change {
    parent: Option<Option<String>>,
    position: Option<f64>,
    place: Option<Place>,
}

impl Topics {
    /// Every page, with what the tree is read from, and its lines, the
    /// workspace of each page's place said where `spaced`.
    pub(crate) async fn pages(&self, table: &Collection, spaced: bool) -> Result<(Vec<Page>, Vec<Line>)> {
        let cells = self.studio.collections.cells(table, CELLS).await?;
        let pages: Vec<Page> =
            cells.into_iter().map(|(id, position, cells)| Page { id, position, cells }).collect();
        let mut spaces: HashMap<String, Option<String>> = HashMap::new();
        let mut lines = Vec::with_capacity(pages.len());
        for page in &pages {
            let space = match page.place() {
                _ if !spaced => None,
                Some(Place::Workspace(workspace)) => Some(workspace),
                Some(Place::Project(project)) => match spaces.get(&project) {
                    Some(space) => space.clone(),
                    None => {
                        let space = self.studio.workspaces.of_project(&project).await?;
                        spaces.insert(project, space.clone());
                        space
                    }
                },
                None => None,
            };
            lines.push(Line {
                id: page.id.clone(),
                parent: page.text(PARENT),
                position: page.position,
                space,
            });
        }
        Ok((pages, lines))
    }

    /// The table of topics, held to whoever asks being able to change it.
    async fn changed_by(&self, by: &Principal) -> Result<Collection> {
        by.require("records:write")?;
        let table = self.studio.collections.get(FACTORY, TOPICS).await.map_err(|_| {
            Error::not_found(format!(
                "this factory has no topics yet: `{} topic make` makes the first",
                brand::CLI
            ))
        })?;
        table.may_change(by)?;
        Ok(table)
    }

    /// Makes a page called `title`, with `body`, under the page `under`
    /// names: of that page's wiki's place, after every page there is. It
    /// has the `status` said, and none where none is.
    pub async fn make_under(
        &self,
        title: &str,
        body: &str,
        under: &str,
        status: Option<&str>,
        by: &Principal,
    ) -> Result<Value> {
        let title = kernel::topics::titled(title).map_err(Error::invalid)?;
        let status = kernel::topics::made_status(status).map_err(Error::invalid)?;
        let table = self.changed_by(by).await?;
        let above = self.find(under, by).await?;
        let _turn = self.studio.collections.changing(ARRANGING).await;
        let (_, above) = self.studio.collections.record(&above.id).await?;
        let place = Place::of(&above.data).ok_or_else(|| {
            Error::invalid(format!(
                "`{}` is nowhere yet, so nothing can be made under it: it is put somewhere first",
                studio::models::Topic::read(&above.data).title
            ))
        })?;
        crate::make_in(self, &table, (title, body, status), place, Some(&above.id), by).await
    }

    /// Moves the page `named` names under the page `under` names, or to the
    /// top, where it is a wiki of its own; before the page `before` names,
    /// or at the end of that level. One write: the page, and where the
    /// place it comes to is another every page under it, said as
    /// `topic.moved` and `topic.placed`. A page that stays where it is
    /// writes nothing and says nothing.
    pub async fn move_to(
        &self,
        named: &str,
        under: Option<&str>,
        before: Option<&str>,
        by: &Principal,
    ) -> Result<Value> {
        let table = self.changed_by(by).await?;
        let page = self.find(named, by).await?.id;
        let under = match under {
            Some(under) => Some(self.find(under, by).await?.id),
            None => None,
        };
        let before = match before {
            Some(before) => Some(self.find(before, by).await?.id),
            None => None,
        };
        let _turn = self.studio.collections.changing(ARRANGING).await;
        let (pages, lines) = self.pages(&table, true).await?;
        let tree = Tree::read(&lines);
        let of = |id: &str| pages.iter().find(|page| page.id == id);
        let landed = kernel::topics::landing(&tree, &page, under.as_deref(), before.as_deref())
            .map_err(|refused| self.refused(refused, &pages, (&page, under.as_deref(), before.as_deref())))?;
        let Landing { stays, positions } = landed;
        if stays {
            return Ok(shown(&self.studio.collections.record(&page).await?.1));
        }
        let mut changes: BTreeMap<String, Change> = BTreeMap::new();
        for (id, position) in positions {
            changes.entry(id).or_default().position = Some(position);
        }
        changes.entry(page.clone()).or_default().parent = Some(under.clone());
        let to = match &under {
            Some(under) => of(under).and_then(Page::place),
            None => of(&page).and_then(Page::place),
        };
        if let Some(to) = to {
            for id in tree.subtree(&page) {
                if of(id).and_then(Page::place).as_ref() != Some(&to) {
                    changes.entry(id.to_string()).or_default().place = Some(to.clone());
                }
            }
        }
        let was = of(&page).map(|one| json!({ "parent": one.text(PARENT), "position": one.position }));
        let written = self.write(&table, changes, Some((&page, was.unwrap_or(Value::Null))), by).await?;
        let moved = written.into_iter().find(|record| record.id == page).ok_or_else(|| crate::none(named))?;
        Ok(shown(&moved))
    }

    /// Puts the wiki `named` names, with every page under it, in a
    /// workspace of its own or in a project; a page under another is where
    /// its wiki is, and is refused.
    pub async fn place(
        &self,
        named: &str,
        (workspace, project): (Option<&str>, Option<&str>),
        by: &Principal,
    ) -> Result<Value> {
        let to = self.place_said(workspace, project).await?;
        let table = self.changed_by(by).await?;
        let wiki = self.find(named, by).await?.id;
        let _turn = self.studio.collections.changing(ARRANGING).await;
        let (pages, lines) = self.pages(&table, false).await?;
        let tree = Tree::read(&lines);
        if let Some(of) = tree.wiki(&wiki).filter(|of| *of != wiki) {
            let title =
                |id: &str| pages.iter().find(|page| page.id == id).map(Page::title).unwrap_or_default();
            return Err(Error::invalid(format!(
                "`{}` is a page of the wiki `{}`, and is where its wiki is: the wiki is put somewhere else (`{cli} \
                 topic move <wiki> --workspace W`), or the page is moved to the top first (`{cli} topic move \
                 <page> --top`)",
                title(&wiki),
                title(of),
                cli = brand::CLI
            )));
        }
        let mut changes = BTreeMap::new();
        for id in tree.subtree(&wiki) {
            if pages.iter().find(|page| page.id == id).and_then(Page::place).as_ref() != Some(&to) {
                changes.insert(id.to_string(), Change { place: Some(to.clone()), ..Change::default() });
            }
        }
        if changes.is_empty() {
            return Ok(shown(&self.studio.collections.record(&wiki).await?.1));
        }
        let written = self.write(&table, changes, None, by).await?;
        Ok(shown(&written.into_iter().find(|record| record.id == wiki).ok_or_else(|| crate::none(named))?))
    }

    /// Brings about that every topic is somewhere there is: a wiki that
    /// says a project or a workspace that is gone, or nothing, is put with
    /// every page under it in the workspace `gone` says that workspace's
    /// things went to, or in the usual one; and a page that is not where
    /// its wiki is is put there. Each said as the system; where everything
    /// is where it is to be, nothing is written and nothing said. How many
    /// pages were put somewhere.
    pub async fn settle(&self, gone: Option<(&str, &str)>) -> Result<usize> {
        let Ok(table) = self.studio.collections.get(FACTORY, TOPICS).await else {
            return Ok(0);
        };
        let Some(usual) = self.studio.workspaces.usual().await? else {
            return Ok(0);
        };
        let workspaces: Vec<String> =
            self.studio.workspaces.list().await?.into_iter().map(|one| one.id).collect();
        let projects: Vec<String> =
            self.studio.projects.list().await?.into_iter().map(|one| one.id).collect();
        let _turn = self.studio.collections.changing(ARRANGING).await;
        let (pages, lines) = self.pages(&table, false).await?;
        let tree = Tree::read(&lines);
        let place_of = |id: &str| pages.iter().find(|page| page.id == id).and_then(Page::place);
        let mut changes = BTreeMap::new();
        for wiki in tree.beside(None) {
            let into = match place_of(&wiki.id) {
                Some(Place::Project(project)) if projects.contains(&project) => Place::Project(project),
                Some(Place::Workspace(workspace)) if workspaces.contains(&workspace) => {
                    Place::Workspace(workspace)
                }
                Some(Place::Workspace(workspace)) => match gone {
                    Some((was, to)) if was == workspace && workspaces.iter().any(|one| one == to) => {
                        Place::Workspace(to.to_string())
                    }
                    _ => Place::Workspace(usual.clone()),
                },
                _ => Place::Workspace(usual.clone()),
            };
            for id in tree.subtree(&wiki.id) {
                if place_of(id).as_ref() != Some(&into) {
                    changes.insert(id.to_string(), Change { place: Some(into.clone()), ..Change::default() });
                }
            }
        }
        let settled = changes.len();
        if settled > 0 {
            self.write(&table, changes, None, &Principal::system()).await?;
        }
        Ok(settled)
    }

    /// Writes what changes of each page, in one write, holding each page's
    /// own turn: `topic.placed` of each whose place changed, and
    /// `topic.moved` of the page `moved` names, with where it was.
    async fn write(
        &self,
        table: &Collection,
        changes: BTreeMap<String, Change>,
        moved: Option<(&str, Value)>,
        by: &Principal,
    ) -> Result<Vec<Record>> {
        let collections = &self.studio.collections;
        // Several pages may share a turn, and one of them the arranging turn held already.
        let ids: Vec<String> = changes.keys().cloned().collect();
        let turns = collections.changing_all(&[ARRANGING], &ids).await;
        let mut records = Vec::with_capacity(changes.len());
        let mut placed = Vec::new();
        for (id, change) in changes {
            let (_, mut record) = collections.record(&id).await?;
            if let Some(parent) = change.parent {
                record.data.insert(PARENT.into(), parent.map_or(Value::Null, Value::String));
            }
            if let Some(position) = change.position {
                record.position = position;
            }
            if let Some(to) = change.place {
                let was = Place::of(&record.data);
                if was.as_ref() != Some(&to) {
                    let (workspace, project) = to.cells();
                    record.data.insert("workspace".into(), workspace);
                    record.data.insert("project".into(), project);
                    placed.push((records.len(), was));
                }
            }
            records.push(record);
        }
        let source = Source::actor(by.actor()).taking(table.reads.as_deref());
        let actor = by.actor();
        let said = move |written: &[Record]| {
            let mut said = Vec::new();
            let about = |record: &Record| {
                let mut data = revised::about(&record.id, &record.data);
                data["by"] = json!(actor);
                data
            };
            if let Some((page, was)) = &moved
                && let Some(at) = written.iter().position(|record| record.id == *page)
            {
                let record = &written[at];
                let mut data = about(record);
                data["parent"] = record.data.get(PARENT).cloned().unwrap_or(Value::Null);
                data["position"] = json!(record.position);
                data["was"] = was.clone();
                said.push((at, Say::new("topic.moved", (source.clone(), data))));
            }
            for (at, was) in placed {
                let mut data = about(&written[at]);
                data["was"] = json!(was.map(|was| said_place(&was)));
                said.push((at, Say::new("topic.placed", (source.clone(), data))));
            }
            said
        };
        let written = collections.put_many_saying(table, records, said).await?;
        drop(turns);
        Ok(written)
    }

    /// What a move that cannot be made is said as.
    fn refused(
        &self,
        refused: Refused,
        pages: &[Page],
        (page, under, before): (&str, Option<&str>, Option<&str>),
    ) -> Error {
        let title =
            |id: &str| pages.iter().find(|one| one.id == id).map_or_else(|| id.to_string(), Page::title);
        match refused {
            Refused::Gone(id) => crate::none(&id),
            Refused::Itself => {
                Error::invalid(format!("`{}` cannot go under itself or a page that is under it", title(page)))
            }
            Refused::Elsewhere => Error::invalid(format!(
                "`{}` is a page of a wiki of another workspace: a page moves among the wikis of its own workspace, \
                 and a whole wiki is put in another with `{} topic move <wiki> --workspace W`",
                under.map(title).unwrap_or_default(),
                brand::CLI
            )),
            Refused::Beside => Error::invalid(format!(
                "`{}` is not beside where `{}` is to go: it is not under {}",
                before.map(title).unwrap_or_default(),
                title(page),
                under.map_or_else(|| "the top".to_string(), |under| format!("`{}`", title(under)))
            )),
        }
    }
}
