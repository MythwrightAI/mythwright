//! Where a project keeps blueprints of its own, over HTTP: where the
//! factory keeps them, in its own data folder and never the project's,
//! named by paths that begin `@blueprints/`; and before that, where it
//! says one, a folder of its repository, a name with a file in both being
//! refused; or none. One that keeps none runs what ships whatever there is
//! and has no fork; one that said nothing reads nothing of its repository,
//! as every project of a factory from before that said nothing does from
//! this program on. What is running goes on with what was vouched for
//! when it was applied. The place goes into a copy, into the Trash with
//! its project and back, and away with it for good; and what a folder of
//! the repository holds is moved into it by one asking.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

const KEPT: &str = "/api/projects/shop/blueprints";
const OWN: &str = "/api/projects/shop/own";

/// A package whose one blueprint says its version, and goes on hearing
/// something once its steps have run, so that it is still there to be
/// said to be behind.
fn package(version: u32) -> String {
    format!(
        "extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    version: {version}\n    \
         on:\n      poke:\n        event: reach.poke\n        steps:\n          heard: {{ do: noop }}\n    \
         steps:\n      made: {{ do: noop }}\n"
    )
}

/// What a project's own file of `reach/round` says: another version, with a step more.
fn forked() -> String {
    "blueprint: reach/round\nversion: 7\non:\n  poke:\n    event: reach.poke\n    steps:\n      heard: { do: noop }\n\
     steps:\n  made: { do: noop }\n  also: { do: noop }\n"
        .to_string()
}

/// A factory with the package installed and a project, Shop, the factory made.
async fn shop() -> Rig {
    let rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": package(1) })).await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig
}

/// Writes a file of Shop's through the daemon, as its owner: vouched for.
async fn write(rig: &Rig, path: &str, text: &str) {
    let body = json!({ "path": path, "content": text });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

/// The keys of the steps the main run of what `reach/round` applied for Shop ran.
async fn ran(rig: &Rig) -> Vec<String> {
    let body = json!({ "blueprint": "reach/round", "project": "shop", "wait": true });
    let applied = rig.send("/api/apply", body).await;
    let instance = applied["instance"].as_str().unwrap_or_else(|| panic!("{applied}")).to_string();
    let runs = rig.read(&format!("/api/runs?instance={instance}&limit=10")).await;
    let main = runs["runs"].as_array().unwrap().iter().find(|run| run["flow"] == "main").cloned();
    let main = main.unwrap_or_else(|| panic!("{applied} {runs}"));
    let run = rig.read(&format!("/api/runs/{}", main["id"].as_str().unwrap())).await;
    let steps = run["steps"].as_array().unwrap_or_else(|| panic!("{run}"));
    let mut keys: Vec<String> = steps
        .iter()
        .map(|step| step["step"].as_str().unwrap_or_else(|| panic!("{step}")).to_string())
        .collect();
    keys.sort();
    keys
}

/// Every line of a summary, those under others among them.
fn said(summary: &Value) -> Vec<String> {
    fn all(lines: &Value, found: &mut Vec<String>) {
        for line in lines.as_array().into_iter().flatten() {
            found.push(line["text"].as_str().unwrap_or_default().to_string());
            all(&line["under"], found);
        }
    }
    let mut found = Vec::new();
    for part in summary["parts"].as_array().into_iter().flatten() {
        all(&part["lines"], &mut found);
    }
    found
}

async fn summary(rig: &Rig) -> Vec<String> {
    let asked =
        rig.send("/api/blueprints/summary", json!({ "name": "reach/round", "project": "shop" })).await;
    said(&asked["summary"])
}

/// Where a file `path` of Shop's own is on the disk: in the factory's place
/// for them, and the project's folder, as `(place, folder)`.
fn on_disk(rig: &Rig, path: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let projects = &rig.node().app.studio.projects;
    (projects.place_of("shop").join(path), projects.dir_of("shop").unwrap().join(path))
}

/// The file a project's own text of `reach/round` is, where the factory keeps it.
fn placed() -> String {
    format!("@blueprints/{}", brand::blueprint_file("reach/round"))
}

/// What `/api/blueprints/stands` says of Shop's own files.
async fn stands(rig: &Rig) -> Vec<Value> {
    let all = rig.read("/api/blueprints/stands").await;
    all["own"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|one| one["project"] == "shop")
        .collect()
}

#[tokio::test]
async fn a_project_that_says_nothing_keeps_its_own_where_the_factory_keeps_them_and_reads_none_of_its_repository()
 {
    let rig = shop().await;
    let kept = rig.read(KEPT).await;
    assert_eq!(
        kept["blueprints"],
        json!({ "said": "usual", "folder": null, "written": "@blueprints" }),
        "{kept}"
    );
    // A file of its repository where a project from before kept its own is
    // nothing of its own, and nothing is said of it: this program's source
    // holds every blueprint that ships so, and each was taken for a fork.
    write(&rig, &format!("blueprints/{}", brand::blueprint_file("reach/round")), &forked()).await;
    let library = rig.node().app.engine.library().clone();
    assert!(!library.is_own("reach/round", Some("shop")));
    assert_eq!(library.passed_over("reach/round", Some("shop")), None);
    assert_eq!(ran(&rig).await, ["made"], "what ships ran");
    assert!(stands(&rig).await.is_empty());
    // Where the factory keeps them, it stands in for what ships, and is a file
    // of the factory's data folder and not of the project's.
    write(&rig, &placed(), &forked()).await;
    assert_eq!(ran(&rig).await, ["also", "made"]);
    let stood = stands(&rig).await;
    assert_eq!((stood.len(), &stood[0]["file"]), (1, &json!(placed())), "{stood:?}");
    let (place, _) = on_disk(&rig, brand::blueprint_file("reach/round").as_str());
    assert_eq!(std::fs::read_to_string(&place).unwrap(), forked());
    assert!(place.starts_with(rig.node().app.place.data.join("blueprints").join("projects").join("shop")));
    let tree = rig.read("/api/projects/shop/tree").await;
    let paths: Vec<&str> =
        tree["entries"].as_array().unwrap().iter().filter_map(|e| e["path"].as_str()).collect();
    assert!(
        !paths.iter().any(|path| path.starts_with('@')),
        "the project's own folder lists none of it: {paths:?}"
    );
}

#[tokio::test]
async fn a_fork_and_a_projects_fleet_are_written_where_the_factory_keeps_them_and_never_in_the_project() {
    let rig = shop().await;
    let made = rig.send(OWN, json!({ "blueprint": "reach/round" })).await;
    assert_eq!(made["path"], json!(placed()), "{made}");
    let (place, folder) = on_disk(&rig, brand::blueprint_file("reach/round").as_str());
    assert!(place.is_file() && !folder.exists(), "{place:?} {folder:?}");
    let (_, in_repo) = on_disk(&rig, &format!("blueprints/{}", brand::blueprint_file("reach/round")));
    assert!(!in_repo.exists());
    assert!(rig.node().app.engine.library().is_own("reach/round", Some("shop")));
    let origin = rig.node().app.engine.forked_from("shop", "reach/round").await.unwrap();
    assert_eq!(origin.path, placed());
    // A project that is not trusted is forked all the same: nothing in that place is passed over.
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("README.md"), "# Theirs\n").unwrap();
    rig.send("/api/projects", json!({ "name": "Yard", "path": theirs })).await;
    let made = rig.send("/api/projects/yard/own", json!({ "blueprint": "reach/round" })).await;
    assert_eq!(made["path"], json!(placed()), "{made}");
    assert!(rig.node().app.engine.library().is_own("reach/round", Some("yard")));
    let there: Vec<_> =
        std::fs::read_dir(&theirs).unwrap().flatten().map(|entry| entry.file_name()).collect();
    assert_eq!(there, ["README.md"], "nothing was written in its folder");
    // The fleet's blueprint is kept the same way, and the project's folder has no folder of blueprints.
    let fleet = rig.send("/api/projects/shop/fleet", json!({})).await;
    assert_eq!(fleet["path"], json!(format!("@blueprints/{}", brand::blueprint_file("fleet"))), "{fleet}");
    let (_, folder) = on_disk(&rig, "blueprints");
    assert!(!folder.exists() && !on_disk(&rig, &brand::blueprint_file("fleet")).1.exists());
    assert!(rig.node().app.engine.library().is_own("fleet", Some("shop")));
}

#[tokio::test]
async fn a_project_that_keeps_none_runs_what_ships_and_says_nothing_of_a_file_kept_for_it() {
    let rig = shop().await;
    write(&rig, &placed(), &forked()).await;
    let mark = rig.mark();
    let kept = rig.send(KEPT, json!({ "none": true })).await;
    assert_eq!((&kept["blueprints"]["said"], &kept["changed"]), (&json!("none"), &json!(true)), "{kept}");
    let event = rig.event("project.blueprints", |event| event.data["project"] == "shop").await;
    assert!(event.seq > mark);
    assert_eq!((&event.data["said"], &event.data["was_said"]), (&json!("none"), &json!("usual")));
    assert!(event.data.get("folder").is_none() && event.data.get("was").is_none(), "{:?}", event.data);
    // Said again as it is, nothing is written or said.
    assert_eq!(rig.send(KEPT, json!({ "none": true })).await["changed"], false);

    let library = rig.node().app.engine.library().clone();
    assert!(!library.is_own("reach/round", Some("shop")));
    assert_eq!(library.passed_over("reach/round", Some("shop")), None, "nothing is said of the file");
    let own: Vec<String> = library.own(Some("shop")).into_iter().map(|(name, _)| name).collect();
    assert!(own.is_empty(), "{own:?}");
    assert_eq!(ran(&rig).await, ["made"], "what ships ran");
    assert!(stands(&rig).await.is_empty());
    let lines = summary(&rig).await;
    assert!(
        lines.iter().any(|line| line.starts_with("This project keeps no blueprints of its own")),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|line| line.starts_with("Fork it")), "{lines:?}");

    // No fork is made, and the sentence says how to have one.
    let (status, refused) = rig.post(OWN, json!({ "blueprint": "reach/round" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let refused = refused["error"]["message"].as_str().unwrap_or_default().to_string();
    assert!(
        refused.contains("keeps no blueprints of its own")
            && refused.contains("project blueprints shop --usual"),
        "{refused}"
    );
    // Nor is an example put among blueprints it does not keep.
    let (status, refused) = rig.post("/api/projects/shop/examples", json!({ "example": "research" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(!on_disk(&rig, "research").0.exists());

    // Its fleet's blueprint, at its top or where the factory keeps it, is read whatever it says.
    let fleet = brand::blueprint_file("fleet");
    write(&rig, &fleet, "blueprint: fleet\nsteps:\n  one: { do: noop }\n").await;
    assert!(library.is_own("fleet", Some("shop")));
    let own: Vec<String> = library.own(Some("shop")).into_iter().map(|(name, _)| name).collect();
    assert_eq!(own, ["fleet"]);
}

#[tokio::test]
async fn a_folder_of_its_repository_is_read_before_where_the_factory_keeps_them_and_a_name_in_both_is_read_from_neither()
 {
    let rig = shop().await;
    let file = format!("tools/bp/{}", brand::blueprint_file("reach/round"));
    write(&rig, &file, &forked()).await;
    assert_eq!(ran(&rig).await, ["made"], "nothing of the repository is read before it is said");
    let kept = rig.send(KEPT, json!({ "folder": "tools/bp/" })).await;
    assert_eq!(
        kept["blueprints"],
        json!({ "said": "folder", "folder": "tools/bp", "written": "@blueprints" }),
        "{kept}"
    );
    // Written there through the daemon, it is vouched for and runs.
    write(&rig, &file, &forked()).await;
    assert_eq!(ran(&rig).await, ["also", "made"]);
    let stood = stands(&rig).await;
    assert_eq!((stood.len(), &stood[0]["file"]), (1, &json!(file)), "{stood:?}");
    // A fork is not made over it, and would be written where the factory keeps them.
    let (status, refused) = rig.post(OWN, json!({ "blueprint": "reach/round" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap_or_default().contains(&file), "{refused}");

    // One where the factory keeps them too: neither is read, and both are named.
    write(&rig, &placed(), &forked()).await;
    let library = rig.node().app.engine.library().clone();
    let why = library.refused("reach/round", Some("shop")).unwrap_or_default();
    assert!(why.contains(&file) && why.contains(&placed()), "{why}");
    assert!(library.load("reach/round", Some("shop")).is_none());
    let body = json!({ "blueprint": "reach/round", "project": "shop", "wait": true });
    let (status, refused) = rig.post("/api/apply", body).await;
    assert_ne!(status, StatusCode::OK, "{refused}");
    // The repository's taken away, the factory's is read.
    rig.call(Method::DELETE, &format!("/api/projects/shop/file?path={file}"), Some(&rig.token), None).await;
    assert_eq!(
        library.called("reach/round", Some("shop")).as_deref(),
        Some(brand::blueprint_file("reach/round").as_str())
    );
    assert_eq!(ran(&rig).await, ["also", "made"]);
    // Going back to the factory's place alone reads it alone.
    rig.send(KEPT, json!({ "usual": true })).await;
    assert_eq!(rig.read(KEPT).await["blueprints"]["said"], "usual");
    assert_eq!(stands(&rig).await[0]["file"], json!(placed()));
}

#[tokio::test]
async fn what_is_running_goes_on_with_what_was_vouched_for_whatever_the_project_says_of_its_folder_since() {
    let rig = shop().await;
    write(&rig, &placed(), &forked()).await;
    let body = json!({ "blueprint": "reach/round", "project": "shop", "wait": true });
    let instance = rig.send("/api/apply", body).await["instance"].as_str().unwrap().to_string();
    let before = rig.read(&format!("/api/instances/{instance}")).await;
    assert!(before["source"].as_str().unwrap_or_default().contains("also:"), "it runs the fork: {before}");
    let mark = rig.mark();
    rig.send(KEPT, json!({ "none": true })).await;
    rig.event("project.blueprints", |event| event.seq > mark).await;
    // Nothing is taken or run again by the change itself: it is the fork's text still.
    let after = rig.read(&format!("/api/instances/{instance}")).await;
    assert_eq!(
        (&after["instance"]["revision"], &after["source"]),
        (&before["instance"]["revision"], &before["source"]),
        "{after}"
    );
    // And it is behind what is there now, which is what ships.
    assert_eq!(after["instance"]["behind"]["version"], "1", "{after}");
    assert!(before["instance"]["behind"].is_null(), "{before}");
    let taken = rig
        .said(journal::EventQuery {
            after: mark,
            topics: "instance.*,run.started".into(),
            ..Default::default()
        })
        .await;
    assert!(taken.is_empty(), "{:?}", taken.iter().map(|event| &event.topic).collect::<Vec<_>>());
    assert!(stands(&rig).await.is_empty(), "nothing of the file is said as the project's own");
    // Applied again, what ships is what is there to run.
    assert_eq!(ran(&rig).await, ["made"]);
}

#[tokio::test]
async fn a_folder_is_said_within_the_project_and_by_whoever_may_take_one_up() {
    let rig = shop().await;
    for (asked, says) in [
        (json!({ "folder": "../elsewhere" }), "does not stay within the project"),
        (json!({ "folder": "/etc" }), "is not a path within the project"),
        (json!({ "folder": "@blueprints" }), "is not a path within the project"),
        (json!({ "folder": "." }), "is no folder of blueprints"),
        (json!({ "folder": "a", "none": true }), "say one of"),
        (json!({}), "say one of"),
    ] {
        let (status, refused) = rig.post(KEPT, asked.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{asked}: {refused}");
        assert!(
            refused["error"]["message"].as_str().unwrap_or_default().contains(says),
            "{asked}: {refused}"
        );
    }
    // A file, and a link that leads out of the project's folder.
    write(&rig, "notes.md", "words").await;
    let (status, refused) = rig.post(KEPT, json!({ "folder": "notes.md" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    #[cfg(unix)]
    {
        let dir = std::path::PathBuf::from(rig.read("/api/projects/shop").await["path"].as_str().unwrap());
        let outside = dir.parent().unwrap().join("elsewhere");
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, dir.join("out")).unwrap();
        let (status, refused) = rig.post(KEPT, json!({ "folder": "out/bp" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert!(
            refused["error"]["message"].as_str().unwrap_or_default().contains("leads out of"),
            "{refused}"
        );
    }
    // A path of the factory's place is no folder of the project's to leave by.
    let (status, refused) = rig
        .call(Method::GET, "/api/projects/shop/file?path=@blueprints/../../x", Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    // A member may read where they are, and not change it.
    let made = rig
        .send("/api/credentials", json!({ "name": "member", "scopes": kernel::principal::MEMBER_SCOPES }))
        .await;
    let token = made["token"].as_str().unwrap().to_string();
    let (status, read) = rig.call(Method::GET, KEPT, Some(&token), None).await;
    assert_eq!((status, &read["blueprints"]["said"]), (StatusCode::OK, &json!("usual")), "{read}");
    let (status, refused) = rig.call(Method::POST, KEPT, Some(&token), Some(json!({ "none": true }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) =
        rig.call(Method::POST, KEPT, Some(&token), Some(json!({ "move_in": true }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(rig.read(KEPT).await["blueprints"]["said"], "usual");
}

#[tokio::test]
async fn where_a_project_keeps_its_own_blueprints_and_what_the_factory_keeps_for_it_go_into_a_copy_of_the_factory()
 {
    let shared = crate::snapshots::Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    prod.send(KEPT, json!({ "folder": "tools/bp" })).await;
    let body =
        json!({ "path": "@blueprints/kept.bp", "content": "blueprint: kept\nsteps:\n  one: { do: noop }\n" });
    let (status, answer) =
        prod.call(Method::PUT, "/api/projects/shop/file", Some(&prod.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let to = shared.snapshot("first");
    crate::snapshots::taken(&prod, &to).await;
    let copy = shared.copy("staging", &to).await;
    assert_eq!(
        copy.read(KEPT).await["blueprints"],
        json!({ "said": "folder", "folder": "tools/bp", "written": "@blueprints" })
    );
    let read = copy.read("/api/projects/shop/file?path=@blueprints/kept.bp").await;
    assert!(read["content"].as_str().unwrap_or_default().contains("blueprint: kept"), "{read}");
    assert!(copy.node().app.engine.library().is_own("kept", Some("shop")));
    // What the copy holds was looked at as it arrived, the place with the rest: a scan finds nothing new.
    let scanned = copy.send("/api/projects/shop/scan", json!({})).await;
    assert_eq!(scanned["changes"], json!([]), "{scanned}");
    crate::apart::held_to_the_lists(&copy.node().app.journal).await;
}

/// What the factory's journal keeps of where `project` keeps its own, and what is held of it in memory.
async fn kept_of(rig: &Rig, project: &str) -> (Option<Value>, Value) {
    let app = &rig.node().app;
    let kept = app.journal.kv_get(conductor::library::folders::NAMESPACE, project).await.unwrap();
    (kept, app.studio.projects.folders().of(project).shown())
}

/// What a project that said nothing is shown as.
fn usual() -> Value {
    json!({ "said": "usual", "folder": null, "written": "@blueprints" })
}

#[tokio::test]
async fn a_project_in_the_trash_keeps_what_it_said_and_what_is_kept_for_it_and_one_deleted_for_good_takes_both()
 {
    let rig = shop().await;
    rig.send(KEPT, json!({ "folder": "tools/bp" })).await;
    write(&rig, &placed(), &forked()).await;
    let place = rig.node().app.studio.projects.place_of("shop");
    let trashed = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    let trash = trashed["trash"].as_str().unwrap().to_string();
    // Kept in the Trash, with what the factory keeps for it, and given back saying what it said.
    assert!(kept_of(&rig, "shop").await.0.is_some());
    assert!(!place.exists(), "carried into the Trash with its project");
    rig.send(&format!("/api/trash/{trash}/restore"), json!({})).await;
    assert_eq!(rig.read(KEPT).await["blueprints"]["folder"], "tools/bp");
    assert_eq!(ran(&rig).await, ["also", "made"], "its own is back with it");

    // Deleted for good, what it said goes in the same write, and what is kept for it with its entry.
    let trashed = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    let trash = trashed["trash"].as_str().unwrap().to_string();
    let (status, gone) =
        rig.call(Method::DELETE, &format!("/api/trash/{trash}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    assert_eq!(kept_of(&rig, "shop").await, (None, usual()));
    assert!(!place.exists());
    // Made again under its name, it is a project that said nothing and keeps nothing.
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(rig.read(KEPT).await["blueprints"]["said"], "usual");
    assert_eq!(ran(&rig).await, ["made"]);
}

#[tokio::test]
async fn what_is_read_of_a_projects_own_where_the_factory_keeps_none_yet_makes_no_place_for_them() {
    let rig = shop().await;
    let place = rig.node().app.studio.projects.place_of("shop");
    assert!(!place.exists(), "nothing of its own is written yet");
    let (status, read) = rig
        .call(Method::GET, "/api/projects/shop/file?path=@blueprints/none.bp", Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{read}");
    let (status, refused) = rig
        .call(Method::GET, "/api/projects/shop/file?path=@blueprints/../up.bp", Some(&rig.token), None)
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    // A reading made it, and made it again under a project that had just gone into the Trash with it.
    assert!(!place.exists(), "a reading makes no folder");
    write(&rig, &placed(), &forked()).await;
    assert!(place.exists(), "what writes there makes it");
}

#[tokio::test]
async fn a_project_deleted_at_once_where_nothing_is_kept_in_the_trash_takes_what_it_said_and_what_is_kept_for_it()
 {
    let rig = shop().await;
    rig.send("/api/system/trash", json!({ "days": 0 })).await;
    write(&rig, &placed(), &forked()).await;
    rig.send(KEPT, json!({ "none": true })).await;
    let forgot = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await.1;
    assert!(forgot["trash"].is_null(), "{forgot}");
    assert_eq!(kept_of(&rig, "shop").await, (None, usual()));
    assert!(!rig.node().app.studio.projects.place_of("shop").exists());
}

#[tokio::test]
async fn a_project_made_by_the_assistant_that_is_built_in_begins_with_nothing_said_whatever_was_kept_under_its_name()
 {
    let rig = Rig::new().await;
    // Something said under its name before, as nothing this program writes leaves behind.
    let app = &rig.node().app;
    let stale = json!({ "folder": null });
    app.journal.kv_set(conductor::library::folders::NAMESPACE, "riverside-shop", &stale).await.unwrap();
    app.studio.projects.folders().put("riverside-shop", conductor::library::folders::Folder::Nowhere);
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    rig.send(
        "/api/channels/general/messages",
        json!({ "body": "Please set up a new project Riverside Shop" }),
    )
    .await;
    rig.count("agent.turn.finished", 1).await;
    assert_eq!(rig.read("/api/projects").await["projects"][0]["id"], "riverside-shop");
    assert_eq!(kept_of(&rig, "riverside-shop").await, (None, usual()));
}

#[tokio::test]
async fn a_project_made_through_the_route_begins_with_nothing_said_whatever_was_kept_under_its_name() {
    let rig = Rig::new().await;
    let app = &rig.node().app;
    app.journal
        .kv_set(conductor::library::folders::NAMESPACE, "shop", &json!({ "folder": "old" }))
        .await
        .unwrap();
    app.studio.projects.folders().put("shop", conductor::library::folders::Folder::In("old".into()));
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(kept_of(&rig, "shop").await, (None, usual()));
}

#[tokio::test]
async fn what_a_folder_of_its_repository_holds_is_moved_in_with_what_its_forks_were_made_from_and_left_where_it_was()
 {
    let rig = shop().await;
    let app = rig.node().app.clone();
    // A fork an earlier program made in the project's repository, with its record saying so.
    let file = format!("blueprints/{}", brand::blueprint_file("reach/round"));
    let words = "blueprints/reach/prompts/ask.md";
    write(&rig, &file, &forked()).await;
    write(&rig, words, "Ask it plainly.\n").await;
    write(&rig, &brand::blueprint_file("deploy"), "blueprint: deploy\nsteps:\n  one: { do: noop }\n").await;
    // One deeper in the repository, which is read by its whole path; and one where nothing is looked for.
    let deeper = format!("tools/{}", brand::blueprint_file("ship"));
    write(&rig, &deeper, "blueprint: tools/ship\nsteps:\n  one: { do: noop }\n").await;
    write(&rig, &format!(".hidden/{}", brand::blueprint_file("away")), "blueprint: away\n").await;
    let who = kernel::Principal::system();
    let origin = conductor::forked::Origin::of("reach/round", &forked(), &file, &who, 1);
    app.engine.fork_made("shop", origin, kernel::Source::actor(who.actor())).await.unwrap();
    rig.send(KEPT, json!({ "folder": "blueprints" })).await;
    assert_eq!(ran(&rig).await, ["also", "made"]);
    let library = app.engine.library().clone();
    assert!(library.is_own("tools/ship", Some("shop")), "read by its whole path while the folder is said");

    // Where the factory keeps them holds one of the names saying something else: nothing is moved.
    write(&rig, "@blueprints/deploy.bp", "blueprint: deploy\nsteps:\n  two: { do: noop }\n").await;
    let (status, refused) = rig.post(KEPT, json!({ "move_in": true })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(
        refused["error"]["message"].as_str().unwrap_or_default().contains("@blueprints/deploy.bp"),
        "{refused}"
    );
    assert!(!on_disk(&rig, &brand::blueprint_file("reach/round")).0.exists());
    assert_eq!(rig.read(KEPT).await["blueprints"]["said"], "folder");
    rig.call(Method::DELETE, "/api/projects/shop/file?path=@blueprints/deploy.bp", Some(&rig.token), None)
        .await;

    let moved = rig.send(KEPT, json!({ "move_in": true })).await;
    let to: Vec<&str> =
        moved["moved"].as_array().unwrap().iter().filter_map(|one| one["to"].as_str()).collect();
    assert_eq!(
        to,
        [
            "@blueprints/deploy.bp",
            "@blueprints/reach/prompts/ask.md",
            placed().as_str(),
            &format!("@blueprints/{deeper}")
        ],
        "{moved}"
    );
    assert!(library.is_own("tools/ship", Some("shop")), "and by the same name where the factory keeps it");
    assert!(!on_disk(&rig, &format!(".hidden/{}", brand::blueprint_file("away"))).0.exists());
    assert_eq!(moved["blueprints"], usual());
    assert_eq!(rig.read(KEPT).await["blueprints"], usual());
    // Each copied as it was, the repository's left as it was.
    for (from, there) in
        [(&file, placed()), (&words.to_string(), "@blueprints/reach/prompts/ask.md".to_string())]
    {
        let (_, repo) = on_disk(&rig, from);
        let (place, _) = on_disk(&rig, there.trim_start_matches("@blueprints/"));
        assert_eq!(std::fs::read(&repo).unwrap(), std::fs::read(&place).unwrap(), "{from}");
    }
    // What the fork was made from says where it is now, and it is the project's own still, vouched for by who moved it.
    assert_eq!(app.engine.forked_from("shop", "reach/round").await.unwrap().path, placed());
    assert_eq!(
        app.engine.library().called("reach/round", Some("shop")).as_deref(),
        Some(brand::blueprint_file("reach/round").as_str())
    );
    assert_eq!(ran(&rig).await, ["also", "made"]);
    // Asked again it finds them there as they are, and moves nothing over them.
    let again = rig.send(KEPT, json!({ "move_in": true, "from": "blueprints" })).await;
    assert_eq!(again["moved"].as_array().map(Vec::len), Some(4), "{again}");
}

#[tokio::test]
async fn a_factory_from_before_keeps_what_each_project_said_and_reads_no_folder_of_a_repository_it_did_not_say()
 {
    let mut rig = shop().await;
    // A project made of the source of this program: every blueprint that ships under
    // `blueprints/std/`, which a program from before read as the project's forks and passed over with a note.
    let source = rig.dir.join("source");
    std::fs::create_dir_all(source.join("blueprints/reach")).unwrap();
    std::fs::write(source.join("blueprints/reach").join(brand::blueprint_file("round")), forked()).unwrap();
    rig.send("/api/projects", json!({ "name": "Mine", "path": source })).await;
    // And one that said a folder of its own, as a program from before kept it.
    let app = rig.node().app.clone();
    std::fs::create_dir_all(on_disk(&rig, "old/reach").1).unwrap();
    std::fs::write(on_disk(&rig, &format!("old/{}", brand::blueprint_file("reach/round"))).1, forked())
        .unwrap();
    app.journal
        .kv_set(conductor::library::folders::NAMESPACE, "shop", &json!({ "folder": "old" }))
        .await
        .unwrap();
    rig.restart().await;

    let library = rig.node().app.engine.library().clone();
    assert_eq!(library.passed_over("reach/round", Some("mine")), None, "no note of a file it does not read");
    assert!(!library.is_own("reach/round", Some("mine")));
    assert_eq!(rig.read("/api/projects/mine/blueprints").await["blueprints"], usual());
    assert_eq!(rig.read(KEPT).await["blueprints"]["folder"], "old", "what it said is read as it was");
    assert!(library.is_own("reach/round", Some("shop")));
    // Moved in from where it read them before, it reads them where the factory keeps them.
    let moved = rig.send("/api/projects/mine/blueprints", json!({ "move_in": true })).await;
    assert_eq!(
        moved["moved"][0]["from"],
        json!(format!("blueprints/{}", brand::blueprint_file("reach/round"))),
        "{moved}"
    );
    assert!(
        library.is_own("reach/round", Some("mine")),
        "what the factory keeps for it is never passed over"
    );
    assert_eq!(library.passed_over("reach/round", Some("mine")), None);
    assert!(
        source.join("blueprints/reach").join(brand::blueprint_file("round")).is_file(),
        "the repository's left"
    );
}

/// What ships as `reach/round`, but that what it hears does `word`: a
/// text taken where its instance is, which following takes.
fn heard_as(word: &str) -> String {
    format!(
        "blueprint: reach/round\nversion: 1\non:\n  poke:\n    event: reach.poke\n    steps:\n      {word}: {{ do: noop }}\n\
         steps:\n  made: {{ do: noop }}\n"
    )
}

#[tokio::test]
async fn where_texts_are_followed_another_folder_of_its_own_is_taken_as_a_save_of_its_files_would_be() {
    let rig = shop().await;
    let file = brand::blueprint_file("reach/round");
    write(&rig, &format!("@blueprints/{file}"), &heard_as("noticed")).await;
    let body = json!({ "blueprint": "reach/round", "project": "shop", "wait": true });
    let instance = rig.send("/api/apply", body).await["instance"].as_str().unwrap().to_string();
    let (status, now) = rig
        .call(Method::POST, "/api/system/texts", Some(&rig.token), Some(json!({ "come": "follow" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{now}");
    let source = || async {
        rig.read(&format!("/api/instances/{instance}")).await["source"].as_str().unwrap().to_string()
    };
    assert_eq!(source().await, heard_as("noticed"));
    // Written where it keeps nothing of its own yet: nothing is taken.
    let mark = rig.mark();
    write(&rig, &format!("tools/bp/{file}"), &heard_as("third")).await;
    rig.event("project.file.changed", move |event| event.seq > mark).await;
    assert_eq!(source().await, heard_as("noticed"));

    // Read from there too from now on: the factory's is taken away first, or the two are read from neither.
    rig.call(
        Method::DELETE,
        &format!("/api/projects/shop/file?path=@blueprints/{file}"),
        Some(&rig.token),
        None,
    )
    .await;
    let revision = blueprint::revision(&heard_as("third"));
    rig.send(KEPT, json!({ "folder": "tools/bp" })).await;
    rig.event("instance.updated", move |event| event.data["revision"] == revision.as_str()).await;
    assert_eq!(source().await, heard_as("third"));

    // Kept nowhere: as though the file was taken away, and what ships is taken.
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    let revision = blueprint::revision(&ships);
    let mark = rig.mark();
    rig.send(KEPT, json!({ "none": true })).await;
    rig.event("instance.updated", move |event| {
        event.seq > mark && event.data["revision"] == revision.as_str()
    })
    .await;
    assert_eq!(source().await, ships);
}

#[tokio::test]
async fn where_texts_are_not_followed_another_folder_of_its_own_is_taken_by_nothing_running() {
    let rig = shop().await;
    let file = brand::blueprint_file("reach/round");
    write(&rig, &format!("@blueprints/{file}"), &heard_as("noticed")).await;
    let body = json!({ "blueprint": "reach/round", "project": "shop", "wait": true });
    let instance = rig.send("/api/apply", body).await["instance"].as_str().unwrap().to_string();
    let before = rig.read(&format!("/api/instances/{instance}")).await;
    let mark = rig.mark();
    rig.send(KEPT, json!({ "none": true })).await;
    // Heard and gone through by what follows texts, which takes nothing here.
    rig.send(KEPT, json!({ "usual": true })).await;
    rig.event("project.blueprints", move |event| event.seq > mark && event.data["said"] == "usual").await;
    let taken = rig
        .said(journal::EventQuery { after: mark, topics: "instance.updated".into(), ..Default::default() })
        .await;
    assert!(taken.is_empty(), "{taken:?}");
    assert_eq!(rig.read(&format!("/api/instances/{instance}")).await["source"], before["source"]);
}

#[tokio::test]
async fn what_running_instances_were_vouched_for_of_a_folder_a_project_from_before_read_unsaid_is_let_go_once()
 {
    let mut rig = shop().await;
    // As a program from before left it: the project said nothing, read `blueprints/` all the same, and what
    // was applied there was vouched for with the file under it.
    let file = format!("blueprints/{}", brand::blueprint_file("reach/round"));
    rig.send(KEPT, json!({ "folder": "blueprints" })).await;
    write(&rig, &file, &forked()).await;
    // What reaches it, as a stage reaches a text it calls when it hears something.
    let caller = "blueprint: caller\non:\n  go:\n    event: shop.go\n    steps:\n      round: { call: reach/round }\n\
                  steps:\n  one: { do: noop }\n";
    let applied = rig.send("/api/apply", json!({ "source": caller, "project": "shop", "wait": true })).await;
    let instance = applied["instance"].as_str().unwrap().to_string();
    let given = |rig: &Rig| {
        let (engine, instance) = (rig.node().app.engine.clone(), instance.clone());
        async move {
            let whom = engine.instance(&instance).await.unwrap().principal;
            engine.text_within(&instance, &whom, "reach/round", Some("shop")).await
        }
    };
    assert_eq!(given(&rig).await, Some(forked()));
    let app = rig.node().app.clone();
    app.journal.kv_set(conductor::library::folders::NAMESPACE, "shop", &Value::Null).await.unwrap();
    rig.restart().await;
    // Called while the file is no longer the project's, it went on with it as it was vouched for and says so,
    // as instances in use said of every blueprint of the checkout.
    rig.send("/api/emit/shop.go", json!({ "project": "shop" })).await;
    let noted = |rig: &Rig| {
        let (engine, instance) = (rig.node().app.engine.clone(), instance.clone());
        async move { engine.instance(&instance).await.unwrap().error }
    };
    rig.event("instance.call_kept", |event| event.data["blueprint"] == "reach/round").await;
    let note = noted(&rig).await.unwrap_or_default();
    assert!(note.starts_with("`reach/round` cannot be applied here as it is now"), "{note}");
    // A factory from before had not settled anything.
    let app = rig.node().app.clone();
    let settled = conductor::library::folders::SETTLED;
    app.journal.kv_set(conductor::library::folders::NAMESPACE, settled, &Value::Null).await.unwrap();
    rig.restart().await;

    // Started by this program, the name is let go and the note taken back in that start, before anything calls
    // it again: what ships is what the instance is given.
    assert_eq!(noted(&rig).await, None, "the note is gone as the factory starts");
    assert_eq!(rig.read(KEPT).await["blueprints"], usual());
    let ships = rig.node().app.engine.library().shipped("reach/round", None).unwrap();
    assert_eq!(given(&rig).await, Some(ships));
    let app = rig.node().app.clone();
    assert!(app.journal.kv_get(conductor::library::folders::NAMESPACE, settled).await.unwrap().is_some());
    // Once: a project that comes to say nothing later is never taken back from what was vouched for.
    rig.send(KEPT, json!({ "folder": "blueprints" })).await;
    let again = rig
        .send("/api/apply", json!({ "source": caller, "project": "shop", "key": "two", "wait": true }))
        .await;
    let two = again["instance"].as_str().unwrap().to_string();
    rig.send(KEPT, json!({ "usual": true })).await;
    rig.restart().await;
    let engine = rig.node().app.engine.clone();
    let whom = engine.instance(&two).await.unwrap().principal;
    let kept = engine.text_within(&two, &whom, "reach/round", Some("shop")).await;
    assert_eq!(kept, Some(forked()), "what was vouched for is run as it was then");
}

#[tokio::test]
async fn what_a_project_uses_lists_its_own_with_the_files_each_links_and_says_where_a_new_one_is_written() {
    let rig = shop().await;
    let text = "blueprint: tell\nsteps:\n  said:\n    do: text.put\n    with:\n      parts:\n        - from: ask.md\n        - from: none.md\n";
    write(&rig, "@blueprints/tell.bp", text).await;
    write(&rig, "@blueprints/prompts/ask.md", "Ask it plainly.\n").await;
    let used = rig.read("/api/blueprints/used").await;
    let shop = &used["projects"]["shop"];
    assert_eq!(shop["written"], "@blueprints", "{used}");
    let tell = shop["own"].as_array().unwrap().iter().find(|one| one["name"] == "tell").cloned();
    let tell = tell.unwrap_or_else(|| panic!("{used}"));
    assert_eq!(tell["file"], "@blueprints/tell.bp");
    assert_eq!(
        tell["links"],
        json!(["@blueprints/prompts/ask.md"]),
        "a file it names that is not there is not listed"
    );
    // One that keeps none writes none.
    rig.send(KEPT, json!({ "none": true })).await;
    assert_eq!(rig.read("/api/blueprints/used").await["projects"]["shop"]["written"], Value::Null);
}
