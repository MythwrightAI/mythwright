//! What an item's merge follows up, as git has it: the work of other
//! items, merged before it into the project's own branch, that it reverts
//! or changes the files of again.
//!
//! Each merge `place.merge` makes ends in a line that says which item it
//! merged (a trailer, as git reads `Signed-off-by:`: [`ITEM`]), so that
//! the item a commit came with is known long after its branch is gone. At
//! an item's merge what it brought is read back from git: each commit
//! that says it reverts another (`This reverts commit …`, as `git revert`
//! writes it) is taken back to the merge that brought that one in, and so
//! to its item; and each earlier merge within so many days that changed
//! any of the same files is an item whose files it changes again.
//!
//! Nothing is kept and nothing is changed: it is the same asked twice,
//! and asked again after the item's merge it finds that merge (the newest
//! that says the item) and answers what it answered. A merge made by a
//! program from before this says no item, and nothing is said of it; nor
//! of what was put on the project's branch without a merge of an item's.
//! Nothing here knows what an item is or what any of this means.

use conductor::{ActionError, ActionResult};
use kernel::Fields;
use serde_json::{Value, json};

use super::Place;

/// What the last line of a merge's commit is called, that says which item
/// it merged.
pub(super) const ITEM: &str = "Item";

/// How many of the merges within the days before an item's are looked
/// at, the newest first; and how many merges that may say the item are.
const LOOKED: &str = "1000";
const SAYING: &str = "20";

/// Days before an item's merge that an earlier one is looked at in, where
/// none are said.
const WITHIN: u64 = 14;

/// One merge into the project's own branch: its commit, when it was made,
/// the item it says it merged, and the files it changed against what the
/// branch had before it.
struct Merge {
    commit: String,
    at: i64,
    item: String,
    files: Vec<String>,
}

/// The format a merge is read in: where it begins, its commit, its time and its item.
fn format() -> String {
    format!("--format=%x01%H%x00%ct%x00%(trailers:key={ITEM},valueonly,separator=%x2C)")
}

/// The merges git listed in [`format`], with the files each changed.
fn merges(listed: &str) -> Vec<Merge> {
    let one = |read: &str| {
        let mut lines = read.lines();
        let mut head = lines.next()?.split('\0');
        let commit = head.next()?.trim().to_string();
        let at = head.next()?.trim().parse().ok()?;
        // One item: a merge that says two is taken as the first's.
        let item = head.next().unwrap_or_default().split(',').next().unwrap_or_default().trim().to_string();
        let files = lines.map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect();
        (!commit.is_empty()).then_some(Merge { commit, at, item, files })
    };
    listed.split('\u{1}').filter_map(one).collect()
}

/// The commits that messages say they revert, as `git revert` writes it,
/// each once.
pub(super) fn reverted(messages: &str) -> Vec<String> {
    const SAYS: &str = "This reverts commit ";
    let mut all: Vec<String> = Vec::new();
    for (at, _) in messages.match_indices(SAYS) {
        let commit: String =
            messages[at + SAYS.len()..].chars().take_while(char::is_ascii_hexdigit).collect();
        if commit.len() >= 7 && !all.contains(&commit) {
            all.push(commit);
        }
    }
    all
}

/// The item whose merge brought `commit` into the project's own branch, as
/// far back as `before`: the commit itself where it is such a merge, and
/// else the first merge after it on that branch that has it.
async fn whose(it: &Place, commit: &str, before: &str) -> Result<Option<String>, ActionError> {
    let project = &it.project;
    let item = |listed: &str| {
        merges(listed).into_iter().next().map(|merge| merge.item).filter(|item| !item.is_empty())
    };
    let itself = it.read(project, &["log", "--no-walk", "--merges", &format(), commit, "--"]).await?;
    if itself.ok()
        && let Some(item) = item(&itself.out)
    {
        return Ok(Some(item));
    }
    let after = format!("{commit}..{before}");
    let brought = ["rev-list", "--first-parent", "--ancestry-path", "--merges", "--reverse", &after, "--"];
    let brought = it.read(project, &brought).await?;
    let Some(merge) = brought.out.lines().next().filter(|_| brought.ok()) else { return Ok(None) };
    let said = it.read(project, &["log", "--no-walk", &format(), merge.trim(), "--"]).await?;
    Ok(if said.ok() { item(&said.out) } else { None })
}

/// What the item's merge follows up: the items whose work it reverts, and
/// those merged within `within` days before it whose files it changes
/// again. Nothing where the item has no merge that says it.
pub(super) async fn follows(it: &Place, args: &Fields) -> ActionResult {
    let within = args.get("within").and_then(Value::as_u64).unwrap_or(WITHIN);
    let none = || Ok(json!({ "merged": "", "reverts": [], "again": [] }));
    let project = &it.project;
    // The newest merge that says the item: looked for by its words, which
    // may be in another's (`shop-1` in `shop-12`), and then taken whole.
    let saying = format!("{ITEM}: {}", it.item);
    let asked = ["log", "--first-parent", "--merges", "-m", "--name-only", &format(), "--fixed-strings"];
    let asked = [&asked[..], &["--grep", &saying, "-n", SAYING, "HEAD", "--"]].concat();
    let found = it.read(project, &asked).await?;
    let Some(merged) =
        merges(if found.ok() { &found.out } else { "" }).into_iter().find(|one| one.item == it.item)
    else {
        return none();
    };
    let before = format!("{}^1", merged.commit);

    let brought =
        it.read(project, &["log", "--format=%B", &format!("{before}..{}", merged.commit), "--"]).await?;
    let mut reverts: Vec<String> = Vec::new();
    for commit in reverted(if brought.ok() { &brought.out } else { "" }) {
        if let Some(item) = whose(it, &commit, &before).await?
            && item != it.item
            && !reverts.contains(&item)
        {
            reverts.push(item);
        }
    }

    // Those merged within the days before it, as far back as git keeps
    // them in order: no more is read than that.
    let days = i64::try_from(within.saturating_mul(86_400)).unwrap_or(i64::MAX);
    let since = format!("--since=@{}", merged.at.saturating_sub(days).max(0));
    let asked = ["log", "--first-parent", "--merges", "-m", "--name-only", &format(), &since, "-n", LOOKED];
    let earlier = it.read(project, &[&asked[..], &[before.as_str(), "--"]].concat()).await?;
    let mut again: Vec<String> = Vec::new();
    for one in merges(if earlier.ok() && within > 0 { &earlier.out } else { "" }) {
        let shares = one.files.iter().any(|file| merged.files.contains(file));
        if shares && !one.item.is_empty() && one.item != it.item && !again.contains(&one.item) {
            again.push(one.item);
        }
    }
    it.note(&format!(
        "{} reverts the work of {} items, and changes again the files of {}",
        merged.commit,
        reverts.len(),
        again.len()
    ));
    Ok(json!({ "merged": merged.commit, "reverts": reverts, "again": again }))
}
