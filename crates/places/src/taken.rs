//! One item's place given to another: where several pieces of work were
//! done on one item, each in a place of its own (`.lanes/<item>--<way>`),
//! the one that is kept becomes the item's own (`.lanes/<item>`, on
//! `lane/<item>`), so that whatever goes by the item's name afterwards
//! (keeping, sending, merging, clearing) finds it there with nothing else
//! changed. Nothing here knows why one was chosen.
//!
//! It is several things asked of git one after another, and a stop may
//! fall between any two, so each is found done or not by looking, and what
//! was half done is finished: the item's own worktree taken away and the
//! other's not yet moved; moved and its branch not yet renamed; all of it
//! done. Nothing that is the only copy of somebody's work is taken away:
//! a worktree of the item's own holding what was never committed, or a
//! branch of its own with work the other's has not, is refused.

use conductor::{ActionError, ActionResult};
use kernel::Fields;
use serde_json::json;

use super::{BRANCHES, Place, Said, UNDER, named};

/// The place of `from` made the item's: its worktree moved under the
/// item's name and its branch renamed after it.
pub(super) async fn take(it: &Place, args: &Fields) -> ActionResult {
    let from = match named(args, "from")? {
        Some(from) if from != it.item => from,
        Some(_) => {
            return Err(ActionError::new("`from` names the item itself: a place is taken from another's"));
        }
        None => return Err(ActionError::new("`from` is said: the piece of work whose place the item takes")),
    };
    let project = &it.project;
    // A project that is no repository, or one with no history to branch
    // from, has no place for any piece of work: each was worked in the
    // project itself (`place.make` says `none`), and there is nothing to take.
    if !it.ask(project, &["rev-parse", "--verify", "--quiet", "HEAD"]).await?.ok() {
        return Err(ActionError::new(format!(
            "{from}'s place cannot be given to {}: the project is no repository with history, so no piece of              work on it had a place of its own",
            it.item
        )));
    }
    let (place, branch) = (it.place(), it.branch());
    let (theirs, their_branch) = (format!("{UNDER}/{from}"), format!("{BRANCHES}/{from}"));
    let _turn = it.turn().await?;
    let came = |how: &str, at: &Said| {
        Ok(json!({ "came": how, "dir": place, "branch": branch, "at": at.out.trim() }))
    };
    let checked_out = |said: Said| said.ok().then(|| said.out.trim().to_string());
    let here = checked_out(it.ask(&project.join(&place), &["rev-parse", "--abbrev-ref", "HEAD"]).await?);
    let Some(theirs_at) = it.branch_of(&from).await? else {
        // Nothing of the other's is left: taken already, where the item's
        // own place is there on its own branch.
        if here.as_deref() == Some(branch.as_str()) {
            return came("there", &it.does(project, &["rev-parse", "--short", &branch]).await?);
        }
        return Err(ActionError::new(format!(
            "{from} has no place to give: there is no branch {their_branch}"
        )));
    };
    // Moved, and stopped before its branch was renamed: only that is left.
    if here.as_deref() != Some(theirs_at.as_str()) {
        if here.is_some() {
            // The item's own worktree goes; what was never committed in it
            // is the only copy there is, and is not thrown away.
            let left = it.ask(&project.join(&place), &["status", "--porcelain"]).await?;
            if !left.ok() || !left.out.trim().is_empty() {
                return Err(ActionError::new(format!(
                    "{place} holds what was never committed, and taking {theirs} in its place would lose it: \
                     commit it or take it away first"
                )));
            }
            let removed = it.ask(project, &["worktree", "remove", &place]).await?;
            if !removed.ok() {
                return Err(ActionError::new(format!(
                    "{place} could not be taken away ({})",
                    removed.words()
                )));
            }
        }
        // Its branch is replaced by the other's only where the other has
        // everything on it: otherwise work would go.
        if it.branch_of(&it.item).await?.is_some() {
            let has = it.ask(project, &["merge-base", "--is-ancestor", &branch, &their_branch]).await?;
            if has.is(1) {
                return Err(ActionError::new(format!(
                    "{branch} has work that {their_branch} has not, and would lose it: {from}'s place is taken \
                     only by an item whose work it has"
                )));
            }
            if !has.ok() {
                return Err(ActionError::new(has.why()));
            }
        }
        let theirs_there = it.ask(&project.join(&theirs), &["rev-parse", "--show-prefix"]).await?.ok();
        let moved = match theirs_there {
            true => it.ask(project, &["worktree", "move", &theirs, &place]).await?,
            // The other's work is on its branch alone: the item is given a
            // worktree on it.
            false => it.ask(project, &["worktree", "add", "--quiet", &place, &their_branch]).await?,
        };
        if !moved.ok() {
            return Err(ActionError::new(format!("{theirs} could not be made {place} ({})", moved.words())));
        }
    }
    let renamed = it.ask(project, &["branch", "-M", &their_branch, &branch]).await?;
    if !renamed.ok() {
        return Err(ActionError::new(format!(
            "{their_branch} could not be renamed {branch} ({})",
            renamed.words()
        )));
    }
    it.note(&format!("{place} is {from}'s work now, on {branch}"));
    came("taken", &it.does(project, &["rev-parse", "--short", &branch]).await?)
}
