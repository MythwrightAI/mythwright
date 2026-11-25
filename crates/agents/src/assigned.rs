//! Who a card may be given to, as far as its workspace goes.
//!
//! A workspace is where the projects that belong together are, and a
//! card is worked in its own: it is given to an agent of its own
//! project, of another project of its workspace, or to one that is no
//! project's and works wherever it is asked. An agent of a project of
//! another workspace is refused, by whichever way the card is given:
//! the board asks this of every change of who a card is given to
//! (`Board::gives_with`), so the drawer, a terminal, a file read in and
//! a blueprint's step are all held to it.
//!
//! It is asked of the giving and of nothing else. A card that names
//! such an agent already (given before this was held, or whose project
//! was put in another workspace since) is left as it is and changed in
//! its other cells; a name that is no agent's is left to whatever reads
//! it, as it always was; and a person is given a card whatever
//! workspace anything is in. Nobody is let in to read anything by it,
//! and nobody is kept from reading anything.

use std::sync::Arc;

use journal::{Journal, optional};
use kernel::{Error, Result};
use studio::Studio;

/// Has the board ask, of each card that is given to somebody, whether
/// its workspace has that agent. What answers holds the journal and
/// where projects belong, and not the keeper of agents: the board is the
/// studio's, which that keeper holds, and the two would hold each other
/// for good.
pub(crate) fn install(journal: &Journal, studio: &Studio) {
    let (journal, workspaces) = (journal.clone(), studio.workspaces.clone());
    studio.board.gives_with(Arc::new(move |project, to| {
        let (journal, workspaces) = (journal.clone(), workspaces.clone());
        Box::pin(async move {
            let Some(name) = to.strip_prefix("agent:").map(str::to_string) else {
                return Ok(());
            };
            let Some(theirs) = works_in(&journal, &name).await? else {
                return Ok(());
            };
            if theirs == project {
                return Ok(());
            }
            match (workspaces.of_project(&project).await?, workspaces.of_project(&theirs).await?) {
                (Some(here), Some(there)) if here != there => {
                    Err(Error::invalid(refused(&name, &theirs, &project)))
                }
                _ => Ok(()),
            }
        })
    }));
}

/// The project an agent works in: nothing for a name that is no agent's,
/// and for an agent that is no project's.
async fn works_in(journal: &Journal, name: &str) -> Result<Option<String>> {
    let lookup = name.to_string();
    let found: Option<Option<String>> = journal
        .db()
        .call(move |connection| {
            optional(
                connection
                    .query_row("SELECT project FROM slots WHERE name = ?1", [&lookup], |row| row.get(0)),
            )
        })
        .await?;
    Ok(found.flatten().filter(|project| !project.is_empty()))
}

/// Why a card is not given to an agent of another workspace's project,
/// and what there is to do.
pub fn refused(agent: &str, theirs: &str, project: &str) -> String {
    format!(
        "`{agent}` works in `{theirs}`, which is in another workspace than `{project}`, whose card this is. \
         A card is given to an agent of its own workspace, or to one that is no project's: \
         `{cli} agent list` says where each works",
        cli = brand::CLI
    )
}
