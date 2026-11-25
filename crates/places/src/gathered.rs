//! Several pieces of work gathered into one place: an item's place of its
//! own (`.lanes/<item>`, on `lane/<item>`), cut from the project's branch,
//! into which the branches of other items, and any branch the repository
//! has, are merged one after another, each as a commit of its own (`--no-ff`),
//! in the order they are handed. What it is for, and what the gathered
//! work is called, is whoever asks: nothing here knows.
//!
//! It is taken whole or not at all. The merges are made with the place's
//! worktree let go of its branch (detached), and the branch is moved to
//! what they came to only once every one went in; where one does not go
//! with what was gathered before it, that merge is called off and the
//! place is put back on its branch as it was, and the branch that would
//! not merge is said with git's words, for whoever's work it is to put
//! right. So the branch never holds part of an asking.
//!
//! It is the same done twice. Asked again, what the branch holds already
//! is passed over (git says whether a branch is in it), and a gathering a
//! stop cut off is found by what it left: a merge under way in the place
//! is called off and a place let go of its branch is put back on it, which
//! leaves the branch where it was, or where the last asking had moved it.
//! So it comes to the same commits whichever moment a stop fell in, and
//! to the same answer.

use conductor::{ActionError, ActionResult};
use kernel::{Fields, git};
use serde_json::{Value, json};

use super::{Place, Said, follows, is_a_name, text};

/// The branches handed, in order: each other item's, then each named.
fn handed(args: &Fields, key: &str) -> Vec<String> {
    let each = args.get(key).and_then(Value::as_array).into_iter().flatten();
    each.filter_map(|one| one.as_str().map(str::to_string)).filter(|one| !one.is_empty()).collect()
}

/// One branch to gather: what it is called, and the item it is of where it
/// is an item's, which its merge says in its last line.
struct Gathered {
    branch: String,
    item: Option<String>,
}

pub(super) async fn gather(it: &Place, args: &Fields) -> ActionResult {
    let project = &it.project;
    let (place, branch) = (it.place(), it.branch());
    let came = |how: &str, merged: &[String], had: &[String], nothing: &[String], with: &str, said: &str| {
        Ok(json!({
            "came": how, "dir": place, "branch": branch, "merged": merged, "had": had, "nothing": nothing,
            "with": with, "said": said,
        }))
    };
    // What is gathered: the branches of the items handed, in order, then
    // the branches named, in order.
    let mut wanted: Vec<Gathered> = Vec::new();
    let mut nothing = Vec::new();
    for other in handed(args, "items") {
        if !is_a_name(&other) {
            return Err(ActionError::new(format!(
                "`items` names other pieces of work by what each is known by, such as a card's id, `shop-1`: \
                 `{other}` cannot be the name of a folder or a branch"
            )));
        }
        if other == it.item {
            return Err(ActionError::new(format!(
                "`items` names {other}, whose place the others are gathered into: an item is not gathered into itself"
            )));
        }
        match it.branch_of(&other).await? {
            Some(branch) => wanted.push(Gathered { branch, item: Some(other) }),
            // Worked in the project itself, or never worked: nothing of its
            // own to gather.
            None => nothing.push(other),
        }
    }
    for named in handed(args, "branches") {
        let format = it.ask(project, &["check-ref-format", "--branch", &named]).await?;
        if named.starts_with('-') || !format.ok() {
            return Err(ActionError::new(format!(
                "`{named}` is not the name of a branch: a branch is named as the repository knows it, such as \
                 `feature` or `origin/feature`"
            )));
        }
        let commit = format!("{named}^{{commit}}");
        if !it.ask(project, &["rev-parse", "--verify", "--quiet", &commit]).await?.ok() {
            return Err(ActionError::new(format!("the repository has no branch called {named}")));
        }
        wanted.push(Gathered { branch: named, item: None });
    }
    // A project that is no repository with history has no branch to gather
    // anything on.
    if !it.ask(project, &["rev-parse", "--verify", "--quiet", "HEAD"]).await?.ok() {
        return came("none", &[], &[], &nothing, "", "");
    }
    it.ignored().await;
    let _turn = it.turn().await?;
    if let Some(kept) = it.half_made(&place).await? {
        it.retake(&place, &kept).await?;
    }
    let at = project.join(&place);
    if !it.ask(&at, &["rev-parse", "--show-prefix"]).await?.ok() {
        let head = format!("refs/heads/{branch}");
        let made = match it.ask(project, &["rev-parse", "--verify", "--quiet", &head]).await?.code {
            Some(0) => it.ask(project, &["worktree", "add", "--quiet", &place, &branch]).await?,
            Some(1) => {
                // Cut from where it is said to be, or the project's own branch.
                let onto = text(args, "onto").unwrap_or_else(|| "HEAD".to_string());
                if onto.starts_with('-') {
                    return Err(ActionError::new(format!("`{onto}` is not the name of a branch")));
                }
                it.ask(project, &["worktree", "add", "--quiet", "--no-track", "-b", &branch, &place, &onto])
                    .await?
            }
            _ => return Err(ActionError::new(format!("{branch} could not be looked for"))),
        };
        if !made.ok() {
            return Err(ActionError::new(format!("its place {place} could not be made ({})", made.words())));
        }
    }
    it.unlocked(&at).await?;
    // What a gathering a stop cut off left: a merge under way, and the
    // place let go of its branch. Both are this action's own doing, in the
    // item's own place, which nobody else works in.
    if it.ask(&at, &["rev-parse", "--verify", "--quiet", "MERGE_HEAD"]).await?.ok() {
        it.note("a gathering was cut off in the middle of a merge; it is called off and made again");
        it.ask(&at, &["merge", "--abort"]).await?;
    }
    let on = it.does(&at, &["rev-parse", "--abbrev-ref", "HEAD"]).await?.out.trim().to_string();
    if on == "HEAD" {
        it.note("a gathering was cut off before its branch had what it gathered; it is begun again");
        it.does(&at, &["checkout", "--quiet", &branch]).await?;
    } else if on != branch {
        return Err(ActionError::new(format!(
            "{place} has {on} checked out and not {branch}: somebody is at work there. Check {branch} out there \
             again, and ask again"
        )));
    }
    let left = it.does(&at, &["status", "--porcelain", "--untracked-files=no"]).await?;
    if !left.out.trim().is_empty() {
        return Err(ActionError::new(format!(
            "{place} holds changes that were never committed, and nothing is gathered over them: commit them or \
             take them away, and ask again"
        )));
    }
    // What the branch holds already is passed over.
    let (mut had, mut owed) = (Vec::new(), Vec::new());
    for one in wanted {
        let holds = it.ask(&at, &["merge-base", "--is-ancestor", &one.branch, "HEAD"]).await?;
        match holds.code {
            Some(0) => had.push(one.branch),
            Some(1) => owed.push(one),
            _ => return Err(ActionError::new(holds.why())),
        }
    }
    if owed.is_empty() {
        return came("gathered", &[], &had, &nothing, "", "");
    }
    // Merged with the branch let go of, so that it moves once, to all of it.
    it.does(&at, &["checkout", "--quiet", "--detach"]).await?;
    let mut merged = Vec::new();
    for one in &owed {
        let message = format!("Gather {}", one.branch);
        let trailer = one.item.as_ref().map(|item| format!("{}: {item}", follows::ITEM));
        if let Err((said, why)) = merge(it, &at, &one.branch, &message, trailer.as_deref()).await? {
            // Put back on its branch as it was: none of this asking is kept.
            it.does(&at, &["checkout", "--quiet", &branch]).await?;
            if why.is_empty() {
                it.note(&format!(
                    "{} does not go with what is gathered; nothing of this is kept",
                    one.branch
                ));
                return came("conflict", &[], &had, &nothing, &one.branch, &said);
            }
            return Err(ActionError::new(format!("{} could not be gathered: {why}", one.branch)));
        }
        merged.push(one.branch.clone());
    }
    // The branch is moved to what the merges came to, and checked out there.
    it.does(&at, &["checkout", "--quiet", "-B", &branch]).await?;
    it.note(&format!("{branch} has {} gathered", merged.join(", ")));
    came("gathered", &merged, &had, &nothing, "", "")
}

/// Merges `branch` into what the place has checked out, with nothing left
/// behind where it does not go in: answered with what git said and why
/// (nothing for a conflict, git's words for anything else).
async fn merge(
    it: &Place,
    at: &std::path::Path,
    branch: &str,
    message: &str,
    trailer: Option<&str>,
) -> Result<Result<(), (String, String)>, ActionError> {
    let mut asked = vec!["merge", "--no-ff", "--no-edit", "-m", message];
    if let Some(trailer) = trailer {
        asked.extend(["-m", trailer]);
    }
    asked.push(branch);
    let under_way = ["rev-parse", "--verify", "--quiet", "MERGE_HEAD"];
    let mut waited = git::Waited::default();
    loop {
        let mut merged: Said = it.ask_once(at, &asked).await?;
        if merged.ok() {
            return Ok(Ok(()));
        }
        let conflict = it.ask(at, &under_way).await?.ok()
            && !it.ask(at, &["ls-files", "--unmerged"]).await?.out.trim().is_empty();
        // Whatever stopped it, the place is put back as it was.
        if it.ask(at, &under_way).await?.ok() {
            it.ask(at, &["merge", "--abort"]).await?;
        } else {
            it.ask(at, &["reset", "--quiet", "--merge", "HEAD"]).await?;
        }
        if conflict {
            return Ok(Err((merged.out.clone(), String::new())));
        }
        match git::lock_met(&merged.err) {
            Some(lock) if !waited.spent() => it.waits_for(&lock, &mut waited).await?,
            Some(lock) => {
                merged.err = format!("{}\n{}", merged.err, git::lock_kept(&lock));
                return Ok(Err((merged.out.clone(), merged.words())));
            }
            None => return Ok(Err((merged.out.clone(), merged.words()))),
        }
    }
}
