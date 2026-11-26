//! An item's work put into the branch the project's folder has checked
//! out (`place.merge`), and, where that is asked, what is done when it no
//! longer goes in: the project's branch is merged into the item's, in the
//! item's place, so that whoever works the item has files to put right
//! and nothing else.
//!
//! An agent behind a wall reads the repository's own files and does not
//! write them: it can neither begin a merge nor end one. So the merge is
//! begun here, from outside the wall, and where something conflicts it is
//! left under way, each file that conflicts marked as git marks it. What
//! whoever works the item then leaves is kept by the one commit that keeps
//! its work (`place.keep`), which is the end of that merge, and the item's
//! branch has the project's in its history: merged into the project's
//! afterwards, it goes in. Where nothing conflicts that way the merge is
//! made at once, and the item's branch is merged into the project's again
//! (what does not merge one way and does the other is so for how the
//! folder it is merged in says a file is merged, `.gitattributes`; or the
//! project's branch has moved since).
//!
//! Asked again after a stop, a merge of the project's branch as it is now,
//! under way in the item's place with files still to put right, is found
//! and answered as it was begun, and what was put right is kept; a merge
//! of anything else under way there is called off and begun again.

use std::path::Path;

use conductor::{ActionError, ActionResult};
use kernel::{Fields, git};
use serde_json::json;

use super::{Place, Said, flag, follows, text};

/// How often at most the item's branch is brought up to date and merged
/// again, in one asking: more than once only where the project's branch
/// moved in between.
const AGAIN: usize = 3;

/// What putting the item's branch into the project's came to.
enum Merged {
    Made(String),
    /// It no longer goes in: called off, and what git said.
    Conflict(String),
    /// Something else in the way: what git said, and why.
    Stuck(String, String),
}

/// What merging the project's branch into the item's place came to.
enum Brought {
    /// Made: the item's branch has the project's.
    Made,
    /// Under way, with these files to put right.
    Begun(Vec<String>),
    /// Not begun, and why.
    Not(String),
}

pub(super) async fn merge(it: &Place, args: &Fields) -> ActionResult {
    let came = |how: &str, said: &str, why: &str| {
        Ok(json!({ "came": how, "said": said, "why": why, "files": [], "begun": false }))
    };
    let (project, branch) = (&it.project, it.branch());
    let under_way = ["rev-parse", "--verify", "--quiet", "MERGE_HEAD"];
    // One worked in the project itself has no branch of its own, and its
    // work is there already.
    if !it.ask(project, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).await?.ok() {
        return came("nothing", "", "");
    }
    let _turn = it.turn().await?;
    // A merge already under way in the project's folder is somebody
    // else's, and is not this one's to call off. But for one of this
    // item's own branch: nobody else merges that, so it is this merge,
    // cut off before it was committed. It is called off, and made again.
    let theirs = it.ask(project, &under_way).await?;
    if theirs.ok() {
        let own =
            it.ask(project, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).await?;
        let cut_off = own.ok() && own.out.trim() == theirs.out.trim();
        if !cut_off || !it.ask(project, &["merge", "--abort"]).await?.ok() {
            let why = "a merge is already under way in the project's folder: finish it, or call it off with \
                       `git merge --abort`";
            return came("stuck", "", why);
        }
        it.note(
            "a merge of this branch was cut off before it was committed; it was called off and is made again",
        );
    }
    // What somebody has put forward in the project's folder and not
    // committed is theirs, and git would refuse to merge over it. It is
    // said in words of our own, and is also what tells, after a merge
    // that failed, that whatever is then put forward is that merge's.
    if it.ask(project, &["diff", "--cached", "--quiet"]).await?.is(1) {
        let why = "changes are put forward in the project's folder and not committed (`git status` shows them): \
                   commit them, or take them back with `git restore --staged .`";
        return came("stuck", "", why);
    }
    let message = text(args, "message").unwrap_or_else(|| branch.clone());
    // Its last line says which item, for whatever follows its work up
    // later (`place.follows`): long after the branch is gone.
    let item = format!("{}: {}", follows::ITEM, it.item);
    let bring = flag(args, "bring", false);
    let mut brought = 0;
    loop {
        let said = match into_project(it, &message, &item, &branch).await? {
            Merged::Made(said) => return came("merged", &said, ""),
            Merged::Stuck(said, why) => return came("stuck", &said, &why),
            Merged::Conflict(said) if !bring => return came("conflict", &said, ""),
            Merged::Conflict(said) => said,
        };
        if brought == AGAIN {
            let why = format!(
                "{branch} was brought up to date with the project's branch {AGAIN} times, and the project's \
                 branch had moved on again each time"
            );
            return came("conflict", &said, &why);
        }
        brought += 1;
        match bring_up(it).await? {
            Brought::Made => it.note(&format!("{branch} has the project's branch now; it is merged again")),
            Brought::Begun(files) => {
                return Ok(
                    json!({ "came": "conflict", "said": said, "why": "", "files": files, "begun": true }),
                );
            }
            Brought::Not(why) => {
                let why = format!(
                    "it no longer merges, and the project's branch could not be merged into {branch} where it \
                     was worked: {why}"
                );
                return came("conflict", &said, &why);
            }
        }
    }
}

/// The item's branch merged into the project's, as a commit of its own;
/// where it does not go in, the project's files and index are as they
/// were.
async fn into_project(it: &Place, message: &str, item: &str, branch: &str) -> Result<Merged, ActionError> {
    let project = &it.project;
    let under_way = ["rev-parse", "--verify", "--quiet", "MERGE_HEAD"];
    let mut waited = git::Waited::default();
    loop {
        let asked = ["merge", "--no-ff", "--no-edit", "-m", message, "-m", item, branch];
        let mut merged = it.ask_once(project, &asked).await?;
        if merged.ok() {
            return Ok(Merged::Made(merged.out));
        }
        // It stopped with the merge under way. Where that is on a conflict
        // (something is left unmerged) the two do not go together, and it
        // is called off for whoever worked the item to put right.
        let conflict = it.ask(project, &under_way).await?.ok()
            && !it.ask(project, &["ls-files", "--unmerged"]).await?.out.trim().is_empty();
        // Whatever stopped it, the project's files and its index are put
        // back as they were. Git writes the merged files and then makes the
        // commit, and may be refused between the two (a lock another git
        // took just then, a commit it cannot sign): left so, the next
        // merge here would be refused for changes nobody made.
        if it.ask(project, &under_way).await?.ok() {
            it.ask(project, &["merge", "--abort"]).await?;
        } else if it.ask(project, &["diff", "--cached", "--quiet"]).await?.is(1) {
            it.note("the merge that failed had changed the project's files: they are put back as they were");
            it.ask(project, &["reset", "--quiet", "--merge", "HEAD"]).await?;
        }
        if conflict {
            return Ok(Merged::Conflict(merged.out));
        }
        // Refused a lock that another git holds, it is made again when
        // that git has had a moment, for as long as such a lock is waited
        // for. Anything else in the way is a person's to put right.
        if let Some(lock) = git::lock_met(&merged.err) {
            if !waited.spent() {
                it.waits_for(&lock, &mut waited).await?;
                continue;
            }
            merged.err = format!("{}\n{}", merged.err, git::lock_kept(&lock));
        }
        return Ok(Merged::Stuck(merged.out.clone(), merged.words()));
    }
}

/// The project's branch, as its folder has it checked out now, merged
/// into the item's place: made, or left under way where files conflict.
async fn bring_up(it: &Place) -> Result<Brought, ActionError> {
    let (project, place, branch) = (&it.project, it.place(), it.branch());
    let at = project.join(&place);
    let head = it.does(project, &["rev-parse", "HEAD"]).await?.out.trim().to_string();
    let named = it.ask(project, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    let onto = match named.out.trim() {
        name if named.ok() && name != "HEAD" => name.to_string(),
        _ => head.clone(),
    };
    // Its place is where it was worked; one tidied away, or that git was
    // cut off making, is made again on its branch.
    if let Some(kept) = it.half_made(&place).await? {
        it.retake(&place, &kept).await?;
    }
    if !it.ask(&at, &["rev-parse", "--show-prefix"]).await?.ok() {
        let made = it.ask(project, &["worktree", "add", "--quiet", &place, &branch]).await?;
        if !made.ok() {
            return Ok(Brought::Not(format!("its place {place} could not be made again ({})", made.words())));
        }
    }
    it.unlocked(&at).await?;
    let under_way = ["rev-parse", "--verify", "--quiet", "MERGE_HEAD"];
    let was = it.ask(&at, &under_way).await?;
    if was.ok() {
        let files = unmerged(it, &at).await?;
        if was.out.trim() == head && !files.is_empty() {
            it.note(&format!("the merge of {onto} into {branch} begun before is under way still"));
            return Ok(Brought::Begun(files));
        }
        it.note(
            "a merge under way in its place is not of the project's branch as it is now: it is begun again",
        );
        it.ask(&at, &["merge", "--abort"]).await?;
    }
    let left = it.does(&at, &["status", "--porcelain", "--untracked-files=no"]).await?;
    if !left.out.trim().is_empty() {
        return Ok(Brought::Not(format!("{place} holds changes that were never committed")));
    }
    let message = format!("Merge {onto} into {branch}");
    let mut waited = git::Waited::default();
    loop {
        let mut merged: Said = it.ask_once(&at, &["merge", "--no-edit", "-m", &message, &head]).await?;
        if merged.ok() {
            return Ok(Brought::Made);
        }
        let files = unmerged(it, &at).await?;
        let begun = it.ask(&at, &under_way).await?.ok();
        if begun && !files.is_empty() {
            it.note(&format!("{onto} is being merged into {branch}, and {} conflict", files.join(", ")));
            return Ok(Brought::Begun(files));
        }
        // Whatever else stopped it, the place is put back as it was.
        if begun {
            it.ask(&at, &["merge", "--abort"]).await?;
        } else {
            it.ask(&at, &["reset", "--quiet", "--merge", "HEAD"]).await?;
        }
        if let Some(lock) = git::lock_met(&merged.err) {
            if !waited.spent() {
                it.waits_for(&lock, &mut waited).await?;
                continue;
            }
            merged.err = format!("{}\n{}", merged.err, git::lock_kept(&lock));
        }
        return Ok(Brought::Not(merged.words().trim().to_string()));
    }
}

/// The files a merge under way in `at` has left unmerged.
async fn unmerged(it: &Place, at: &Path) -> Result<Vec<String>, ActionError> {
    let said = it.ask(at, &["diff", "--name-only", "--diff-filter=U"]).await?;
    Ok(said.out.lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect())
}

/// Of a merge under way in `at`, what git marked as conflicting and is
/// left in what is put forward to be committed: why, naming the files,
/// and the lines git says it is at. Nothing where none is left.
pub(super) async fn marked(it: &Place, at: &Path) -> Result<Option<(String, String)>, ActionError> {
    let checked = it.ask(at, &["diff", "--cached", "--check"]).await?;
    let lines: Vec<&str> =
        checked.out.lines().filter(|line| line.ends_with("leftover conflict marker")).collect();
    let mut files: Vec<&str> = lines.iter().filter_map(|line| line.split(':').next()).collect();
    files.dedup();
    if files.is_empty() {
        return Ok(None);
    }
    let why = format!("what git marked as conflicting is left in {}", files.join(", "));
    let said = format!(
        "{why}: put it right, keeping what both sides meant, and leave no marker\n{}",
        lines.join("\n")
    );
    Ok(Some((why, said)))
}
