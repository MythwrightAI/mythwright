use kernel::ErrorKind;
use serde_json::json;

use crate::{Rig, ada, agent};

#[tokio::test]
async fn a_new_project_is_a_directory_with_somewhere_to_start() {
    let rig = Rig::new().await;
    let project = rig.studio.projects.create("Riverside Shop", None, &ada()).await.unwrap();
    assert_eq!(
        (project.id.as_str(), project.name.as_str(), project.brownfield),
        ("riverside-shop", "Riverside Shop", false)
    );
    assert!(std::path::Path::new(&project.path).ends_with("projects/riverside-shop"), "{:?}", project.path);

    let tree = rig.studio.projects.tree("riverside-shop").await.unwrap();
    let paths: Vec<&str> = tree.iter().map(|entry| entry.path.as_str()).collect();
    assert_eq!(
        paths,
        ["docs", "docs/architecture.md", "docs/overview.md", "README.md"],
        "directories first, then files; the repository's own files are not listed, and no folder of blueprints \
         is made in it: the factory keeps a project's own"
    );
    assert!(std::path::Path::new(&project.path).join(".git").is_dir(), "it is a repository from the start");
    let readme = rig.studio.projects.read("riverside-shop", "README.md").await.unwrap();
    assert!(readme.starts_with("# Riverside Shop\n"));

    let created = &rig.events("project.created").await[0];
    assert_eq!(created.data["project"], "riverside-shop");
    assert_eq!(created.source.actor.as_deref(), Some("user:ada"));
    assert_eq!(rig.studio.projects.dir_of("riverside-shop").unwrap().to_string_lossy(), project.path);

    let err = rig.studio.projects.create("riverside shop!", None, &ada()).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Conflict, "the same name, however it is punctuated");
}

#[tokio::test]
async fn an_existing_directory_is_adopted_exactly_as_it_is() {
    let rig = Rig::new().await;
    let existing = rig.dir.join("legacy-shop");
    std::fs::create_dir_all(existing.join("src")).unwrap();
    std::fs::write(existing.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(existing.join("README.md"), "the original readme\n").unwrap();

    let project = rig.studio.projects.create("Legacy Shop", Some(existing.clone()), &ada()).await.unwrap();
    assert!(project.brownfield);
    assert_eq!(rig.studio.projects.read("legacy-shop", "README.md").await.unwrap(), "the original readme\n");
    let paths: Vec<String> =
        rig.studio.projects.tree("legacy-shop").await.unwrap().into_iter().map(|e| e.path).collect();
    assert_eq!(paths, ["src", "src/main.rs", "README.md"], "nothing was added");
    assert!(!existing.join(".git").exists(), "and it was not turned into a repository");

    let err = rig.studio.projects.create("Other", Some("relative/path".into()), &ada()).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Invalid);
}

#[tokio::test]
async fn a_project_says_whose_its_folder_is_by_where_the_folder_is_and_nothing_is_kept_of_it() {
    let rig = Rig::new().await;
    let projects = &rig.studio.projects;
    // Nobody said a folder: the factory made one, where it keeps the projects it makes.
    let made = projects.create("Shop", None, &ada()).await.unwrap();
    assert_eq!(made.folder, "own");
    assert_eq!(serde_json::to_value(&made).unwrap()["folder"], "own", "it is handed out with the rest");

    // A folder somebody said is theirs: one with something in it, and one that was not there yet.
    let existing = rig.dir.join("legacy");
    std::fs::create_dir_all(&existing).unwrap();
    std::fs::write(existing.join("README.md"), "the original readme\n").unwrap();
    let taken = projects.create("Legacy", Some(existing), &ada()).await.unwrap();
    assert_eq!((taken.folder, taken.brownfield), ("given", true));
    let begun = projects.create("Begun", Some(rig.dir.join("begun")), &ada()).await.unwrap();
    assert_eq!((begun.folder, begun.brownfield), ("given", false));

    // One somebody named that is where the factory makes its own reads as the factory's own: it is
    // where the factory would have made it, and nothing is kept that says who named a folder.
    let among = std::path::Path::new(&made.path).parent().unwrap().join("named");
    let named = projects.create("Named", Some(among), &ada()).await.unwrap();
    assert_eq!(named.folder, "own");

    // A file is no folder for a project to be: refused in words, with nothing made of it.
    let file = rig.dir.join("a-file.txt");
    std::fs::write(&file, "not a folder\n").unwrap();
    let refused = projects.create("Paper", Some(file.clone()), &ada()).await.unwrap_err();
    assert_eq!(refused.kind, ErrorKind::Invalid, "{refused:?}");
    assert!(
        refused.message.starts_with(&format!("{} is a file, and a project is a folder.", file.display())),
        "{refused:?}"
    );
    assert!(file.is_file() && projects.get("paper").await.is_err());

    // Read back, one at a time and all together, each says what it said when it was made.
    let said = [("shop", "own"), ("legacy", "given"), ("begun", "given"), ("named", "own")];
    for (id, folder) in said {
        assert_eq!(projects.get(id).await.unwrap().folder, folder, "{id}");
    }
    let listed: Vec<(String, &str)> =
        projects.list().await.unwrap().into_iter().map(|project| (project.id, project.folder)).collect();
    assert_eq!(listed, said.map(|(id, folder)| (id.to_string(), folder)));

    // Nothing was written down for it: a project's row is what it was, so one a program from
    // before kept is answered for the same way.
    let columns: Vec<String> = rig
        .journal
        .db()
        .call(|connection| {
            let mut statement = connection.prepare("SELECT name FROM pragma_table_info('projects')")?;
            let names = statement.query_map([], |row| row.get::<_, String>(0))?;
            names.collect()
        })
        .await
        .unwrap();
    assert_eq!(columns, ["id", "name", "path", "brownfield", "created", "trusted"]);
}

#[tokio::test]
async fn writing_a_file_announces_it_and_reading_gives_it_back() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let projects = &rig.studio.projects;

    let entry =
        projects.write("shop", "docs/decisions/0001-queue.md", "# Use a queue\n", &ada()).await.unwrap();
    assert_eq!((entry.path.as_str(), entry.size), ("docs/decisions/0001-queue.md", 14));
    assert_eq!(projects.read("shop", "docs/decisions/0001-queue.md").await.unwrap(), "# Use a queue\n");
    projects.write("shop", "docs/decisions/0001-queue.md", "# Use two queues\n", &ada()).await.unwrap();

    let changes: Vec<(String, String)> = rig
        .events("project.file.changed")
        .await
        .iter()
        .map(|event| {
            (
                event.data["path"].as_str().unwrap().to_string(),
                event.data["change"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        changes,
        [
            ("docs/decisions/0001-queue.md".to_string(), "added".to_string()),
            ("docs/decisions/0001-queue.md".to_string(), "modified".to_string())
        ]
    );
    assert_eq!(projects.read("shop", "nope.md").await.unwrap_err().kind, ErrorKind::NotFound);
    assert_eq!(projects.read("shop", "docs").await.unwrap_err().kind, ErrorKind::Invalid);

    // Each says the folders the file is in, for whatever follows one of them.
    let said = rig.events("project.file.changed").await;
    assert_eq!(said[0].data["folders"], json!(["docs", "docs/decisions"]));
}

#[test]
fn the_folders_a_file_is_in_are_said_outermost_first_each_from_the_top() {
    let cases: [(&str, &[&str]); 9] = [
        ("docs/guides/install.md", &["docs", "docs/guides"]),
        ("docs/overview.md", &["docs"]),
        // A file at the top is in none, and neither is no file at all.
        ("README.md", &[]),
        ("", &[]),
        // A name that only begins like a folder's is another folder.
        ("docs-old/a.md", &["docs-old"]),
        ("a/b/c/d/e.txt", &["a", "a/b", "a/b/c", "a/b/c/d"]),
        // A folder said as one, with nothing in it named.
        ("docs/", &["docs"]),
        // What no project's file is called is taken as it is written.
        ("/docs/a.md", &["/docs"]),
        ("docs//a.md", &["docs", "docs/"]),
    ];
    for (path, in_these) in cases {
        assert_eq!(studio::projects::folders(path), *in_these, "`{path}`");
    }
}

#[tokio::test]
async fn a_file_written_where_nothing_is_listed_is_written_and_is_not_announced_or_later_said_to_be_gone() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let projects = &rig.studio.projects;

    // Where lanes work is no part of what a project lists.
    let entry = projects.write("shop", ".lanes/.gitignore", "*\n", &ada()).await.unwrap();
    assert_eq!(entry.path, ".lanes/.gitignore");
    assert_eq!(projects.read("shop", ".lanes/.gitignore").await.unwrap(), "*\n");

    assert!(projects.scan("shop").await.unwrap().is_empty(), "a scan finds nothing to say of it");
    assert!(rig.events("project.file.changed").await.is_empty());
    let listed: Vec<String> =
        projects.tree("shop").await.unwrap().into_iter().map(|entry| entry.path).collect();
    assert!(!listed.iter().any(|path| path.starts_with(".lanes")), "{listed:?}");
}

#[tokio::test]
async fn paths_cannot_leave_the_project() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let outside = rig.dir.join("secret.txt");
    std::fs::write(&outside, "do not read me").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&rig.dir, rig.studio.projects.dir_of("shop").unwrap().join("escape")).unwrap();

    let projects = &rig.studio.projects;
    let mut attempts = vec!["../secret.txt", "/etc/passwd", "docs/../../secret.txt", ".git/config", ""];
    if cfg!(unix) {
        attempts.push("escape/secret.txt");
    }
    for path in attempts {
        let read = projects.read("shop", path).await.unwrap_err();
        assert_eq!(read.kind, ErrorKind::Invalid, "read {path:?}: {}", read.message);
        let write = projects.write("shop", path, "overwritten", &ada()).await.unwrap_err();
        assert_eq!(write.kind, ErrorKind::Invalid, "write {path:?}: {}", write.message);
    }
    assert_eq!(std::fs::read_to_string(&outside).unwrap(), "do not read me");
}

#[tokio::test]
async fn writing_needs_permission_and_reading_does_not_need_as_much() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let reader = agent("reviewer", &["projects:read"]);
    let err = rig.studio.projects.write("shop", "notes.md", "hi", &reader).await.unwrap_err();
    assert_eq!(err.kind, ErrorKind::Denied);
    assert_eq!(
        rig.studio.projects.create("Another", None, &reader).await.unwrap_err().kind,
        ErrorKind::Denied
    );
}

#[tokio::test]
async fn a_scan_finds_what_changed_behind_our_back_and_only_that() {
    let rig = Rig::new().await;
    let project = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let dir = std::path::Path::new(&project.path);
    let projects = &rig.studio.projects;
    assert_eq!(projects.scan("shop").await.unwrap(), [], "nothing has changed since it was created");

    // As an agent or a build would: straight to disk.
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn add() {}\n").unwrap();
    std::fs::write(dir.join("README.md"), "# Shop\n\nNow with more words.\n").unwrap();
    std::fs::remove_file(dir.join("docs/architecture.md")).unwrap();
    std::fs::create_dir_all(dir.join("target/debug")).unwrap();
    std::fs::write(dir.join("target/debug/shop"), "binary").unwrap();

    let mut changes: Vec<(String, &str)> =
        projects.scan("shop").await.unwrap().into_iter().map(|change| (change.path, change.change)).collect();
    changes.sort();
    assert_eq!(
        changes,
        [
            ("README.md".to_string(), "modified"),
            ("docs/architecture.md".to_string(), "removed"),
            ("src/lib.rs".to_string(), "added")
        ],
        "build output is ignored"
    );
    assert_eq!(rig.events("project.file.changed").await.len(), 3);
    assert_eq!(projects.scan("shop").await.unwrap(), [], "a second look finds nothing new");
}

#[tokio::test]
async fn removing_a_file_or_directory_is_announced() {
    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.remove("shop", "docs", &ada()).await.unwrap();
    let paths: Vec<String> =
        rig.studio.projects.tree("shop").await.unwrap().into_iter().map(|e| e.path).collect();
    assert_eq!(paths, ["README.md"]);
    let removed: Vec<String> = rig
        .events("project.file.changed")
        .await
        .iter()
        .filter(|event| event.data["change"] == "removed")
        .map(|event| event.data["path"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(removed, ["docs/architecture.md", "docs/overview.md"]);
    assert_eq!(
        rig.studio.projects.remove("shop", "docs", &ada()).await.unwrap_err().kind,
        ErrorKind::NotFound
    );
}

#[tokio::test]
async fn setting_up_the_fleet_writes_a_blueprint_once_that_says_where_the_fleet_is_and_calls_the_one_that_ships()
 {
    let rig = Rig::new().await;
    rig.studio.projects.create("Riverside Shop", None, &ada()).await.unwrap();
    let projects = &rig.studio.projects;

    let entry = projects.init_fleet("riverside-shop", &ada()).await.unwrap();
    // Kept by the factory, and never in the project's folder.
    assert_eq!(entry.path, format!("@blueprints/{}", brand::blueprint_file("fleet")));
    assert!(
        !std::path::Path::new(&projects.get("riverside-shop").await.unwrap().path).join("fleet.bp").exists()
    );
    let source = projects.read("riverside-shop", &entry.path).await.unwrap();

    // It is a well-formed blueprint about this project.
    let checked = blueprint::check(&source, &blueprint::NoCatalog);
    assert_eq!(checked.errors().map(|d| d.render("fleet")).collect::<Vec<_>>(), Vec::<String>::new());
    let blueprint = checked.blueprint.unwrap();
    assert_eq!(blueprint.name, "fleet");
    let default = |name: &str| blueprint.params.iter().find(|p| p.name == name).unwrap().default.clone();
    assert_eq!(default("project"), Some(json!("riverside-shop")));
    assert!(blueprint.description.unwrap().contains("Riverside Shop"));
    // Who is in the fleet and what they do with a card are tables of the
    // project, changed there: the file says nothing of either.
    for table in ["architects", "workers", "reviewers", "seats", "agents", "stages"] {
        assert!(blueprint.params.iter().all(|param| param.name != table), "`{table}` is said in the file");
    }
    // What it does say is a param of its own, and is handed on as it is to
    // the fleet that ships, which is the one thing this does.
    assert_eq!(blueprint.flow.steps.iter().map(|step| step.id.as_str()).collect::<Vec<_>>(), ["fleet"]);
    assert!(blueprint.triggers.is_empty());
    for param in blueprint.params.iter().map(|param| param.name.as_str()) {
        assert!(
            source.contains(&format!("      {param}: ${{params.{param}}}"))
                || source.contains(&format!("      {param}: \"${{params.{param}}}\"")),
            "`{param}` is asked for and not handed on"
        );
    }
    for choice in ["channel", "planned", "fetch", "push"] {
        assert!(default(choice).is_some(), "`{choice}` is not a choice of the project's");
    }

    // Someone edits it; setting the fleet up again must not undo that.
    projects
        .write("riverside-shop", &entry.path, "blueprint: fleet\nsteps:\n  mine: { do: noop }\n", &ada())
        .await
        .unwrap();
    projects.init_fleet("riverside-shop", &ada()).await.unwrap();
    assert!(projects.read("riverside-shop", &entry.path).await.unwrap().contains("mine"));
}

#[tokio::test]
async fn forgetting_a_project_leaves_its_files_alone() {
    let rig = Rig::new().await;
    let project = rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    rig.studio.projects.forget("shop", &ada()).await.unwrap();
    assert!(rig.studio.projects.list().await.unwrap().is_empty());
    assert!(std::path::Path::new(&project.path).join("README.md").is_file());
    assert_eq!(rig.studio.projects.tree("shop").await.unwrap_err().kind, ErrorKind::NotFound);

    // And it can be added back as the directory it now is.
    let again = rig.studio.projects.create("Shop", Some(project.path.into()), &ada()).await.unwrap();
    assert!(again.brownfield);
}

#[cfg(unix)]
#[tokio::test]
async fn a_file_that_is_saved_is_put_in_place_whole_and_never_seen_cut_short() {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;

    let rig = Rig::new().await;
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    let projects = &rig.studio.projects;
    let first = "#!/bin/sh\necho the first, and a good deal longer than what follows\n";
    projects.write("shop", "tools/run.sh", first, &ada()).await.unwrap();
    let file = projects.dir_of("shop").unwrap().join("tools/run.sh");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Something has the file open as it is saved again: an export reading
    // it, an agent, an editor.
    let mut open = std::fs::File::open(&file).unwrap();
    projects.write("shop", "tools/run.sh", "#!/bin/sh\necho second\n", &ada()).await.unwrap();

    // It reads the whole of what the file was. Written over where it was,
    // it would read a file emptied and half filled.
    let mut read = String::new();
    open.read_to_string(&mut read).unwrap();
    assert_eq!(read, first);
    // Whoever opens it now reads the whole of what it is, and it may still
    // be run.
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "#!/bin/sh\necho second\n");
    assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o777, 0o755);
    // And nothing it was written under on the way is left beside it.
    let beside: Vec<String> = std::fs::read_dir(file.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(beside, ["run.sh"]);

    // A link is written through, as it was: it stays a link.
    std::os::unix::fs::symlink("run.sh", file.with_file_name("go.sh")).unwrap();
    projects.write("shop", "tools/go.sh", "#!/bin/sh\necho third\n", &ada()).await.unwrap();
    assert!(file.with_file_name("go.sh").symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "#!/bin/sh\necho third\n");
}

/// Runs a test where what blocks is done by one thread, in the order it
/// was asked for: a look's walk of the files, and then whatever the test
/// asks for to know that the walk is over.
fn one_at_a_time(test: impl Future<Output = ()>) {
    let runtime =
        tokio::runtime::Builder::new_current_thread().max_blocking_threads(1).enable_all().build().unwrap();
    runtime.block_on(test);
}

/// Begins a look over the project's files and holds it where its walk is
/// over and it has decided nothing: it goes on when it is next waited for.
async fn walked<T>(look: &mut std::pin::Pin<&mut impl Future<Output = T>>) {
    // The one thread is kept busy until the look has asked for its walk,
    // so that the look gets no further than asking: begun with the thread
    // free, it may have walked and decided before it was looked at again.
    let (go, wait) = std::sync::mpsc::channel::<()>();
    let busy = tokio::task::spawn_blocking(move || {
        let _ = wait.recv();
    });
    let mut nobody = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(look.as_mut().poll(&mut nobody).is_pending(), "its walk waits for the thread");
    go.send(()).unwrap();
    busy.await.unwrap();
    // The walk was next, and is over when what was asked for after it is.
    tokio::task::spawn_blocking(|| ()).await.unwrap();
}

async fn said_removed(rig: &Rig) -> Vec<String> {
    let events = rig.events("project.file.changed").await;
    let removed = events.iter().filter(|event| event.data["change"] == "removed");
    removed.map(|event| event.data["path"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_file_written_while_a_look_has_walked_and_not_yet_decided_is_not_said_to_be_removed() {
    one_at_a_time(async {
        let rig = Rig::new().await;
        let projects = &rig.studio.projects;
        projects.create("Shop", None, &ada()).await.unwrap();
        let look = projects.scan("shop");
        tokio::pin!(look);
        walked(&mut look).await;

        // Written through the daemon, which writes it down and says so.
        projects.write("shop", "docs/new.md", "# New\n", &ada()).await.unwrap();

        // The walk did not see it and it is written down: it is there.
        assert_eq!(look.await.unwrap(), [], "the look that walked before it was written");
        assert_eq!(said_removed(&rig).await, [] as [&str; 0]);
        assert_eq!(projects.scan("shop").await.unwrap(), [], "and a later look has nothing to add");
    });
}

#[test]
fn of_two_looks_the_one_that_walked_first_and_decides_last_goes_by_the_files_as_they_are_then() {
    one_at_a_time(async {
        let rig = Rig::new().await;
        let projects = &rig.studio.projects;
        let dir = std::path::PathBuf::from(projects.create("Shop", None, &ada()).await.unwrap().path);
        std::fs::write(dir.join("old.md"), "old").unwrap();
        projects.scan("shop").await.unwrap();
        let first = projects.scan("shop");
        tokio::pin!(first);
        walked(&mut first).await;

        // Behind the daemon's back: one made, one changed, one taken away.
        std::fs::write(dir.join("made.md"), "made").unwrap();
        std::fs::write(dir.join("README.md"), "changed, and longer than it was by a good deal").unwrap();
        std::fs::remove_file(dir.join("old.md")).unwrap();
        let later: Vec<(String, &str)> = projects
            .scan("shop")
            .await
            .unwrap()
            .into_iter()
            .map(|change| (change.path, change.change))
            .collect();
        let found = [
            ("README.md".to_string(), "modified"),
            ("made.md".to_string(), "added"),
            ("old.md".to_string(), "removed"),
        ];
        assert_eq!(later, found);

        // The first walked before any of it, and by now all of it is written
        // down: it has nothing to say, and takes nothing back.
        assert_eq!(first.await.unwrap(), [], "the look that walked first");
        assert_eq!(said_removed(&rig).await, ["old.md"]);
        assert_eq!(projects.scan("shop").await.unwrap(), []);
    });
}

#[tokio::test]
async fn a_project_made_anew_begins_with_nothing_said_of_where_it_keeps_its_own_blueprints() {
    use conductor::library::folders::{Folder, NAMESPACE};
    let rig = Rig::new().await;
    // What a project of its name said before it was deleted, as nothing this program writes leaves behind.
    rig.journal.kv_set(NAMESPACE, "shop", &json!({ "folder": null })).await.unwrap();
    rig.studio.projects.folders().put("shop", Folder::Nowhere);
    rig.studio.projects.create("Shop", None, &ada()).await.unwrap();
    assert_eq!(rig.journal.kv_get(NAMESPACE, "shop").await.unwrap(), None);
    assert_eq!(rig.studio.projects.folders().of("shop"), Folder::Usual);
}
