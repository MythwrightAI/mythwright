//! Work landed by a pipeline of the project's own, stopped anywhere and
//! started again: two items' branches gathered into one place
//! (`place.gather`), the project's branch brought forward to it and sent to
//! the remote (`place.forward`), and that said. Wherever the daemon goes,
//! it comes back with the project's branch, the remote's and the gathered
//! branch each at the same commits as a play nobody stopped, and it said
//! once.

use std::path::Path;

use kernel::Event;
use serde_json::{Value, json};

use crate::found::Grain;
use crate::gathered::{shop, subjects};
use crate::lanes::git;
use crate::observed::node_with_stand_ins;
use crate::simulation::{Came, Place, Play, Scenario, simulate};

/// The pipeline: everything it does is the actions', and it says when.
const LANDING: &str = "\
blueprint: landing
permissions: [shell, emit:landing.done]
steps:
  gathered:
    do: place.gather
    with: { item: train-1, items: [shop-1, shop-2] }
  forwarded:
    needs: [gathered]
    do: place.forward
    with: { item: train-1, remote: origin }
  done:
    needs: [forwarded]
    do: emit
    with: { topic: landing.done }
";

/// The owner runs the project's landing.
#[derive(Clone)]
struct Landed;

impl Scenario for Landed {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-landing").await;
        let (dir, remote) = shop(&rig, &[("shop-1", "one.md", "one"), ("shop-2", "two.md", "two")]).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.to_string_lossy().into_owned());
        play.notes.insert("remote", remote.to_string_lossy().into_owned());
        play
    }

    /// Applied by its key, looking first, as a person does: a landing the
    /// daemon went away in the middle of applying is there to be seen once
    /// it is back, under way, taken up or over, and is applied only where it
    /// is not. Looked for by what it said, a landing taken up as the daemon
    /// came back and not yet done was applied again, and once it was over
    /// that made another, which landed the work a second time.
    async fn act(&self, _: usize, play: &mut Play) {
        let there = play.rig.read("/api/instances?project=shop&blueprint=landing").await;
        if there["instances"].as_array().is_some_and(|all| !all.is_empty()) {
            return;
        }
        let apply = json!({ "project": "shop", "key": "landing", "source": LANDING });
        play.rig.send("/api/apply", apply).await;
    }

    fn heard(&self, _: usize, _: &Play, event: &Event) -> bool {
        event.topic == "landing.done"
    }

    /// What the play came to, read whatever it has come to so far: what it
    /// has come to is read before it is asked whether anything is still under
    /// way, so the gathered branch and its place may not be there yet, and
    /// are then nothing.
    async fn came_to(&self, play: &Play) -> Came {
        let (dir, remote) = (Path::new(play.note("dir")), Path::new(play.note("remote")));
        let tree = |at: &Path, branch: &str| git(at, &["rev-parse", &format!("{branch}^{{tree}}")]);
        let said = play.rig.count_of("landing.done").await;
        let gathered = branched(dir, "lane/train-1").then(|| subjects(dir, "lane/train-1"));
        let place = dir.join(".lanes/train-1");
        let on = place.exists().then(|| git(&place, &["rev-parse", "--abbrev-ref", "HEAD"]));
        Came::default()
            .with("main", json!(subjects(dir, "main")))
            .with("gathered", json!(gathered))
            .with("theirs", json!(tree(remote, "main") == tree(dir, "main")))
            .with("on its branch", json!(on))
            .with("said", Value::from(said))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn work_landed_by_a_pipeline_comes_to_the_same_commits_wherever_the_daemon_is_stopped() {
    fn within(place: &Place) -> bool {
        place.within("landing")
    }
    let found = simulate("work landed", Landed, Grain::asked_or(Grain::Where(within))).await;
    found.holds();
}

/// Whether `dir`'s repository has `branch`, asked as the tests ask git.
fn branched(dir: &Path, branch: &str) -> bool {
    crate::lanes::asking(dir)
        .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .output()
        .is_ok_and(|ran| ran.status.success())
}
