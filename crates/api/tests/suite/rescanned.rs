//! What changed in a project's files behind the daemon's back is looked
//! for when a run of the project ends (`Studio::watch_runs`), and that a
//! run ended is passed on as it is said and kept only where a text hears
//! it: so a follower of it that falls behind, and can read back from the
//! journal only what was kept, looks at every project instead.

use kernel::Source;
use serde_json::json;

use crate::Rig;

#[tokio::test]
async fn a_look_for_what_runs_wrote_that_fell_behind_what_was_said_looks_at_every_project() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let app = &rig.node().app;
    let dir = app.studio.projects.dir_of("shop").unwrap();
    // Following from here, and not read until far more has been said than
    // is held for whoever is slow: what ended meanwhile is in nothing it
    // can read back.
    let follow = app.journal.follow_every(app.journal.now(), "run.finished");
    std::fs::write(dir.join("written.txt"), "by a run nothing kept the end of").unwrap();
    let mark = rig.mark();
    let said = app
        .journal
        .write(|writing| {
            for at in 0..5000 {
                writing.pass_on("test.said", Source::default(), json!({ "at": at }))?;
            }
            Ok(())
        })
        .await;
    said.unwrap();
    let looking = app.studio.watch_runs_from(&app.engine, follow);
    rig.event("project.file.changed", |event| {
        event.seq > mark && event.data["path"] == "written.txt" && event.data["change"] == "added"
    })
    .await;
    looking.abort();
}
