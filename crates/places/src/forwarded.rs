//! A branch of the project brought forward to an item's branch, and sent:
//! what is gathered in an item's place (`place.gather`), once it is to be
//! kept, is put on the project's own branch as it is, with no merge of its
//! own, and that branch is sent to a remote.
//!
//! Only a fast-forward is made. Where the project's branch has work the
//! item's has not (somebody put something there since it was gathered),
//! nothing is done and that is said: whoever gathered it gathers it again
//! onto what is there now. Where the branch is the one the project's
//! folder has checked out, git moves the folder's files with it, and keeps
//! what somebody changed there and never committed where nothing it brings
//! touches it; where it would change such a file, git refuses before it
//! changes anything, and that is said with git's words: the person's file
//! is theirs, and is never put away for them. A branch nobody has checked
//! out is moved by one write that says where it was, so that it is never
//! moved from under somebody who moved it a moment before.
//!
//! It is the same done twice: a branch that holds the item's work already
//! is brought forward already, and a remote that has the commit already is
//! sent nothing. So asked again after any stop, it answers what it would
//! have, and does only what was left.

use conductor::{ActionError, ActionResult};
use kernel::{Fields, git};
use serde_json::json;

use super::{Place, text};

/// Whether `named` is a branch's name as git takes one.
async fn a_branch(it: &Place, named: &str) -> Result<bool, ActionError> {
    Ok(!named.starts_with('-') && it.ask(&it.project, &["check-ref-format", "--branch", named]).await?.ok())
}

pub(super) async fn forward(it: &Place, args: &Fields) -> ActionResult {
    let project = &it.project;
    let came = |how: &str, onto: &str, at: &str, sent: bool, remote: &str, key: &str, why: &str| {
        Ok(json!({
            "came": how, "branch": onto, "at": at, "sent": sent, "remote": remote, "key": key, "why": why,
        }))
    };
    let Some(theirs) = it.branch_of(&it.item).await? else {
        return came("nothing", "", "", false, "", "", "");
    };
    // The project's branch that is brought forward: the one said, or the
    // one its folder has checked out.
    let onto = match text(args, "onto") {
        Some(onto) => onto,
        None => {
            let here = it.does(project, &["rev-parse", "--abbrev-ref", "HEAD"]).await?.out.trim().to_string();
            if here == "HEAD" {
                return Err(ActionError::new(
                    "the project's folder has no branch checked out: say `onto`, the branch that is brought forward",
                ));
            }
            here
        }
    };
    if !a_branch(it, &onto).await? {
        return Err(ActionError::new(format!("`{onto}` is not the name of a branch")));
    }
    let head = format!("refs/heads/{onto}");
    let was = it.ask(project, &["rev-parse", "--verify", "--quiet", &head]).await?;
    if !was.ok() {
        return Err(ActionError::new(format!("the repository has no branch called {onto}")));
    }
    let remote = text(args, "remote");
    let to = text(args, "to").unwrap_or_else(|| onto.clone());
    if remote.is_some() && !a_branch(it, &to).await? {
        return Err(ActionError::new(format!("`{to}` is not the name of a branch")));
    }
    let _turn = it.turn().await?;
    // Read again with the turn: it may have moved while this waited for it.
    let was = it.does(project, &["rev-parse", "--verify", "--quiet", &head]).await?.out.trim().to_string();
    let has = it.ask(project, &["merge-base", "--is-ancestor", &theirs, &head]).await?;
    match has.code {
        // It holds the item's work already.
        Some(0) => {}
        Some(1) => {
            let ahead = it.ask(project, &["merge-base", "--is-ancestor", &head, &theirs]).await?;
            match ahead.code {
                Some(0) => {}
                Some(1) => {
                    let why = format!(
                        "{onto} has work that {theirs} has not, so it cannot be brought forward to it: gather it \
                         again onto what {onto} is now"
                    );
                    return came("diverged", &onto, &short(it, &was).await?, false, "", "", &why);
                }
                _ => return Err(ActionError::new(ahead.why())),
            }
            if let Some(why) = moved(it, &onto, &head, &was, &theirs).await? {
                return came("stuck", &onto, &short(it, &was).await?, false, "", "", &why);
            }
            it.note(&format!("{onto} is brought forward to {theirs}"));
        }
        _ => return Err(ActionError::new(has.why())),
    }
    let now = it.does(project, &["rev-parse", "--short", &head]).await?.out.trim().to_string();
    let Some(remote) = remote else { return came("forwarded", &onto, &now, false, "", "", "") };
    let key = send(it, &onto, &remote, &to).await?;
    came("forwarded", &onto, &now, true, &remote, key.as_deref().unwrap_or_default(), "")
}

async fn short(it: &Place, commit: &str) -> Result<String, ActionError> {
    Ok(it.does(&it.project, &["rev-parse", "--short", commit]).await?.out.trim().to_string())
}

/// Moves `onto` from `was` to `theirs`, where that is a fast-forward:
/// nothing where it was moved, and why where it was not.
async fn moved(
    it: &Place,
    onto: &str,
    head: &str,
    was: &str,
    theirs: &str,
) -> Result<Option<String>, ActionError> {
    let project = &it.project;
    // Where it is checked out, its worktree's files move with it.
    let listed = it.does(project, &["worktree", "list", "--porcelain"]).await?;
    let mut checked_out = None;
    let mut path = None;
    for line in listed.out.lines() {
        if let Some(at) = line.strip_prefix("worktree ") {
            path = Some(at.to_string());
        } else if line.strip_prefix("branch ") == Some(head) {
            checked_out = path.clone();
        }
    }
    match checked_out {
        None => {
            // Moved only from where it was read to be.
            let new =
                it.does(project, &["rev-parse", "--verify", "--quiet", theirs]).await?.out.trim().to_string();
            let moved = it.ask(project, &["update-ref", head, &new, was]).await?;
            Ok((!moved.ok()).then(|| moved.words()))
        }
        Some(at) if same(&at, project) => {
            if it.ask(project, &["rev-parse", "--verify", "--quiet", "MERGE_HEAD"]).await?.ok() {
                return Ok(Some(
                    "a merge is under way in the project's folder: finish it, or call it off with `git merge \
                     --abort`"
                        .to_string(),
                ));
            }
            let moved = it.ask(project, &["merge", "--quiet", "--ff-only", theirs]).await?;
            Ok((!moved.ok()).then(|| moved.words()))
        }
        Some(at) => Ok(Some(format!(
            "{onto} is checked out at {at}, which is not the project's folder, and is not moved from under it there"
        ))),
    }
}

/// Whether the worktree git lists at `at` is the project's own folder.
fn same(at: &str, project: &std::path::Path) -> bool {
    let at = std::path::Path::new(at);
    at == project
        || kernel::whole::whole(at)
            .ok()
            .zip(kernel::whole::whole(project).ok())
            .is_some_and(|(at, it)| at == it)
}

/// Sends `onto` to the remote's `to`: as the key the step was handed,
/// where it was handed one, and otherwise as whoever runs the daemon.
async fn send(it: &Place, onto: &str, remote: &str, to: &str) -> Result<Option<String>, ActionError> {
    let project = &it.project;
    let known = it.ask(project, &["remote", "get-url", "--push", remote]).await?;
    if !known.ok() {
        return Err(ActionError::new(format!("the repository has no remote called {remote}")));
    }
    let address = known.out.trim().to_string();
    let key = it.key().map(str::to_string);
    if let Some(key) = &key
        && !git::takes_a_key(&address)
    {
        return Err(ActionError::new(format!(
            "{remote} is not reached over https, and the key {key} is only given to a remote that is. Give it an \
             https address (git remote set-url {remote} https://...), or take `key` away and it is sent as \
             whoever runs the daemon"
        )));
    }
    let what = format!("refs/heads/{onto}:refs/heads/{to}");
    let sent = it
        .ask_remote(project, key.as_ref().map(|_| address.as_str()), &["push", "--quiet", remote, &what])
        .await?;
    if !sent.ok() {
        return Err(ActionError::new(format!("{onto} could not be sent to {remote}: {}", sent.why())));
    }
    it.note(&format!("{onto} is on {remote} as {to}"));
    Ok(key)
}
