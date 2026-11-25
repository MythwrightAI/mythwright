//! What the search index reads of the projects and of the board, which
//! are this crate's, as plain values (`search::Indexed`): the index came
//! out of this crate and stands below it, so it asks for them back.

use std::collections::BTreeMap;

use search::{Asked, Card};

use crate::board::{Board, Item};
use crate::projects::{Entry, Projects, blocking};

pub(crate) struct Indexed {
    pub(crate) projects: Projects,
    pub(crate) board: Board,
}

fn card(item: &Item) -> Card {
    Card {
        id: item.id.clone(),
        project: item.project.clone(),
        title: item.title.clone(),
        status: item.status.clone(),
        body: item.body.clone(),
        updated: item.updated,
    }
}

impl search::Indexed for Indexed {
    fn projects(&self) -> Asked<'_, Vec<String>> {
        Box::pin(
            async move { Ok(self.projects.list().await?.into_iter().map(|project| project.id).collect()) },
        )
    }

    fn files<'a>(&'a self, project: &'a str) -> Asked<'a, BTreeMap<String, (i64, u64)>> {
        Box::pin(async move {
            let roots = self.projects.roots(project)?;
            blocking(move || Ok(roots.stamps())).await
        })
    }

    fn read<'a>(&'a self, project: &'a str, path: &'a str) -> Asked<'a, String> {
        Box::pin(self.projects.read(project, path))
    }

    fn stat<'a>(&'a self, project: &'a str, path: &'a str) -> Asked<'a, Entry> {
        Box::pin(self.projects.stat(project, path))
    }

    fn cards(&self) -> Asked<'_, Vec<Card>> {
        Box::pin(async move { Ok(self.board.all().await?.iter().map(card).collect()) })
    }

    fn card<'a>(&'a self, id: &'a str) -> Asked<'a, Card> {
        Box::pin(async move { Ok(card(&self.board.get(id).await?)) })
    }
}
