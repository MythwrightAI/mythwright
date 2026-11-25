//! The actions of places as a blueprint sees them: what each is called,
//! what it takes and gives and what it says of itself, in the order the
//! kept schema lists them. What each does is beside the item's place it
//! asks of (`super`, and a file of its own where one is long).

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionResult, Registry};
use kernel::{BoxFut, Fields};
use serde_json::json;

use super::{
    Place, Places, changed, clear, follows, forwarded, gathered, head, keep, make, merge, send, taken,
};

fn add<F, Fut>(registry: &mut Registry, places: &Places, spec: ActionSpec, run: F)
where
    F: Fn(Place, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    let places = places.clone();
    let run = std::sync::Arc::new(run);
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let (places, run) = (places.clone(), run.clone());
        Box::pin(async move { run(Place::of(&places, cx, &args)?, args).await })
    });
}

/// Adds the actions of places, in the order the kept schema lists them.
pub fn register(registry: &mut Registry, places: &Places) {
    // What these ask of git is asked by the daemon in a project's folder,
    // as a `shell` step's command is: whoever the work is for holds that.
    let of_a_place = |name: &str, summary: &str| {
        ActionSpec::new(name, summary)
            .required(
                "item",
                T::String,
                "what the work is known by, such as a card's id: its place is `.lanes/<item>`, its branch `lane/<item>`",
            )
            .capability("shell")
            .idempotent()
    };
    add(
        registry,
        places,
        of_a_place("place.make", "Make an item's place to work in, a worktree on a branch of its own, or find the one it has.")
            .making("a place to work in, on a branch of its own, for the item", "item")
            .arg("worktree", T::Bool, "false to work the item in the project itself (default true)")
            .arg("dir", T::String, "a place the item already has, from the top of the project: it is used, and none is made")
            .arg("fetch", T::Bool, "ask the remote first: a branch it has for the item is pulled down, and a new one is cut from its `base` where that has everything here")
            .arg("remote", T::String, "the remote that is asked, by the name the repository knows it by (default origin)")
            .arg("base", T::String, "with `fetch`: the remote's branch new work is cut from (default: the one checked out here)")
            .arg("from", T::String, "another item whose branch a new one is cut from, where it has one, such as the item one of several pieces of work on it is done for: what that item had so far is where this begins")
            .output("dir", "where the item is worked, from the top of the project; empty for the project itself")
            .output("came", "how: `there`, `rejoined`, `pulled`, `fresh`, `given`, or `none` where the project is no repository to branch; a place that could not be made is why it fails")
            .output("unasked", "why the remote could not be asked, when it was to be and could not; empty otherwise")
            // Stood in, no repository is asked anything: the item is
            // worked as in a project that is none.
            .stands(json!({ "dir": "", "came": "none", "unasked": "" })),
        |it, args| async move { make(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place("place.keep", "Commit what an agent left uncommitted in an item's place, as the agent's.")
            .arg(
                "dir",
                T::String,
                "the item's place, from the top of the project; with none there is nothing to commit",
            )
            .arg("by", T::String, "the agent whose work it is: the commit's author")
            .arg("message", T::String, "what the commit says (default: the item's id)")
            .arg("unkept_ok", T::Bool, "a commit that cannot be made is no failure: it is answered, with `unkept` saying why")
            .output("placed", "whether the item has a worktree of its own")
            .output("committed", "whether anything was left to commit, and was committed")
            .output("at", "where its branch is now")
            .output("unkept", "with `unkept_ok`: why, in a line, where what was left could not be committed; empty otherwise")
            .output("said", "and all that git said of it, for whoever is to put it right")
            .output("named", "whether git knows who to commit as there, which is not whoever did the work's to put right")
            // Stood in: nothing was left to commit, and nothing is wrong.
            .stands(json!({ "placed": false, "committed": false, "at": "", "unkept": "", "said": "", "named": true })),
        |it, args| async move { keep(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.send",
            "Send an item's branch to a remote, as the key the step is handed if it is handed one.",
        )
        .arg("dir", T::String, "the item's place, from the top of the project")
        .arg("remote", T::String, "the remote it is sent to (default origin)")
        .output("branch", "the branch that was sent")
        .output("remote", "where to")
        .output("key", "the secret it was sent with; empty when it went as whoever runs the daemon"),
        |it, args| async move { send(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place("place.merge", "Merge an item's branch into the branch the project's folder has checked out.")
            .arg("message", T::String, "what the merge's commit says (default: the branch); a last line that says which item is put after it, `Item: <item>`")
            .output("came", "`merged`; `nothing`, the item having no branch of its own; `conflict`, called off and the project's branch as it was; or `stuck`")
            .output("said", "what git said of the merge")
            .output("why", "what is in the way, when it is `stuck`")
            // Stood in: the item has no branch of its own to merge.
            .stands(json!({ "came": "nothing", "said": "", "why": "" })),
        |it, args| async move { merge(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place("place.clear", "Remove an item's worktree, and its branch when its work is somewhere else.")
            .arg("remote", T::String, "the remote the branch may have been sent to (default origin)")
            .arg("discard", T::Bool, "throw its work away: the worktree goes with what was never committed in it, and the branch whether or not its work is anywhere else")
            .output("came", "`cleared`; `nothing`, there being none; `place_kept`, the worktree holding what was never committed; `branch_kept`, its work being neither merged nor on the remote; or `untidied`")
            .output("why", "what git said, when something was kept or could not be tidied")
            // Stood in: there is no place to clear.
            .stands(json!({ "came": "nothing", "why": "" })),
        |it, args| async move { clear(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.changed",
            "Where an item was worked, and what was changed there against the project's own branch.",
        )
        .output(
            "dir",
            "the item's place, from the top of the project; empty where it was worked in the project itself",
        )
        .arg("against", T::String, "another item whose branch it is held against, where it has one, such as the item it was cut from: what was done beside what that had")
        .arg("patch", T::Int, "how many bytes at most of what changed to give as a patch, cut at the end of a line (a megabyte at most); none where nothing is said")
        .output("changed", "what changed, file by file, as git counts it")
        .output("patch", "what changed, as a patch, as far as `patch` says")
        .output("cut", "whether the patch was cut short")
        .output(
            "there",
            "whether that is there: not where the project is a repository with history and the item has no \
             place of its own, it not being worked yet or its place having been tidied away",
        ),
        |it, args| async move { changed(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.head",
            "The commit a folder of the project is at, where it is in a repository with history.",
        )
        .arg("dir", T::String, "the folder, from the top of the project; the project's own where not said")
        .output(
            "commit",
            "the commit it is at; empty where it is in no repository, or one with nothing committed",
        )
        // Stood in: there is no repository.
        .stands(json!({ "commit": "" })),
        |it, args| async move { head(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.take",
            "Give an item the place of another piece of work on it: that worktree and branch become the item's own.",
        )
        .required("from", T::String, "the piece of work whose place the item takes, by what it is known by: its worktree `.lanes/<from>` is moved to the item's and its branch renamed after the item. Refused where the item's own place holds what was never committed, or its branch has work the other's has not")
        .output("came", "`taken`, or `there` where it was taken already")
        .output("dir", "the item's place now, from the top of the project")
        .output("branch", "the item's branch")
        .output("at", "where that branch is")
        // Stood in, nothing is moved.
        .stands(json!({ "came": "there", "dir": "", "branch": "", "at": "" })),
        |it, args| async move { taken::take(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.follows",
            "What an item's merge follows up: the earlier items' work it reverts, and those whose files it changes again soon after.",
        )
        .arg("within", T::Int, "how many days before the item's merge an earlier one is looked at in, for files changed again (default 14)")
        .output("merged", "the commit of the item's merge, the newest that says the item; empty where there is none, and nothing else is said")
        .output("reverts", "the items whose work the merge reverts: each commit it brought that says it reverts another (as `git revert` writes it), taken back to the merge of an item's that brought that one")
        .output("again", "the items merged within those days before it whose files it changes again (of the thousand newest merges then)")
        // Stood in: the item has no merge that says it.
        .stands(json!({ "merged": "", "reverts": [], "again": [] })),
        |it, args| async move { follows::follows(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.gather",
            "Gather other items' branches, and any branch named, into an item's place, each merged in turn, taking all of them or none.",
        )
        .example(GATHER_ASKED)
        .making("a place on a branch of its own, for the item, holding what is gathered", "item")
        .arg("items", T::List, "other items whose branches are gathered, by what each is known by, in the order they are merged; one with no branch of its own has nothing to gather")
        .arg("branches", T::List, "branches of the repository gathered after them, named as the repository knows them, such as `feature` or `origin/feature`")
        .arg("onto", T::String, "where a place the item does not have yet is cut from (default: the branch the project's folder has checked out)")
        .output("came", "`gathered`, every one being on the item's branch now; `conflict`, one not going with what was gathered before it, and nothing of this asking kept; or `none` where the project is no repository with history")
        .output("dir", "the item's place, from the top of the project")
        .output("branch", "the item's branch, which holds what was gathered")
        .output("merged", "the branches merged by this asking, in order")
        .output("had", "the branches the item's branch held already")
        .output("nothing", "the items that have no branch of their own")
        .output("with", "the branch that did not merge, where it is `conflict`")
        .output("said", "what git said of it")
        // Stood in, nothing is merged: everything is gathered already.
        .stands(json!({ "came": "gathered", "dir": "", "branch": "", "merged": [], "had": [], "nothing": [], "with": "", "said": "" })),
        |it, args| async move { gathered::gather(&it, &args).await },
    );
    add(
        registry,
        places,
        of_a_place(
            "place.forward",
            "Bring a branch of the project forward to an item's branch where that is a fast-forward, and send it to a remote, as the key the step is handed if it is handed one.",
        )
        .example(FORWARD_ASKED)
        .arg("onto", T::String, "the project's branch brought forward (default: the one its folder has checked out)")
        .arg("remote", T::String, "the remote it is sent to once it is there; nothing is sent where none is said")
        .arg("to", T::String, "the remote's branch it is sent as (default: the same name)")
        .output("came", "`forwarded`, the branch holding the item's work now; `diverged`, it having work the item's has not; `stuck`, git refusing to move it (`why`); or `nothing`, the item having no branch of its own")
        .output("branch", "the project's branch")
        .output("at", "where it is")
        .output("sent", "whether it was sent")
        .output("remote", "where to")
        .output("key", "the secret it was sent with; empty when it went as whoever runs the daemon")
        .output("why", "what is in the way, when it is `diverged` or `stuck`")
        // Stood in, nothing is moved or sent.
        .stands(json!({ "came": "nothing", "branch": "", "at": "", "sent": false, "remote": "", "key": "", "why": "" })),
        |it, args| async move { forwarded::forward(&it, &args).await },
    );
    super::branches::register(registry, places.branches());
}

// What the two a person writes in a pipeline of their own are shown by in
// the Reference and the editor, each a whole blueprint that asks it.
const GATHER_ASKED: &str = "\
blueprint: gather-a-release
permissions: [shell]

params:
  items: { type: list, description: The items whose work goes out together. }

steps:
  gathered:
    do: place.gather
    with: { item: release, items: \"${params.items}\" }
";

const FORWARD_ASKED: &str = "\
blueprint: release-what-was-gathered
permissions: [shell]

steps:
  forwarded:
    do: place.forward
    with: { item: release, onto: main, remote: origin }
";
