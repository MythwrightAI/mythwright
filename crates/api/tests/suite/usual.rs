//! What a new project starts with, as whoever adds it asks: a board to
//! write work up on, a fleet, where it is said who works it, and a
//! library of its documents. Each is a blueprint applied as whoever added
//! the project, so what they could not apply themselves they do not get
//! by adding a project; and each is asked for by name, or all of them by
//! saying yes.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;

/// The blueprints of what is running for a project, but for what a
/// stage calls as it comes up and is over in a moment: two projects set
/// up alike were held to each other, and one of them still had such a
/// thing alive where the other's had ended (on a machine held to four
/// processors).
async fn running(rig: &Rig, project: &str) -> Vec<String> {
    let live = rig.read(&format!("/api/instances?live=true&project={project}")).await;
    let mut names: Vec<String> = live["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|instance| instance["blueprint"].as_str().unwrap().to_string())
        .filter(|blueprint| !crate::PASSING.contains(&blueprint.as_str()))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The tables a project has.
async fn tables(rig: &Rig, project: &str) -> Vec<String> {
    let all = rig.read(&format!("/api/collections?project={project}")).await;
    let mut names: Vec<String> = all["collections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|collection| collection["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

/// The files of a project, by where they are.
async fn files(rig: &Rig, project: &str) -> Vec<String> {
    let tree = rig.read(&format!("/api/projects/{project}/tree")).await;
    let mut paths: Vec<String> = tree["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["dir"] == false)
        .map(|entry| entry["path"].as_str().unwrap().to_string())
        .collect();
    paths.sort();
    paths
}

/// The two documents a project that has a library begins with.
const DOCUMENTS: [&str; 2] = ["docs/architecture.md", "docs/overview.md"];

#[tokio::test]
async fn a_project_added_with_the_usual_has_a_board_a_fleet_and_a_library_and_one_added_without_has_none() {
    let rig = Rig::new().await;
    let made = rig.send("/api/projects", json!({ "name": "Shop", "set_up": true })).await;
    assert_eq!(made["id"], "shop");
    let given = &made["set_up"];
    assert!(given["board"].is_string() && given["fleet"].is_string() && given["docs"].is_string(), "{made}");
    assert!(given["left"].is_null(), "nothing was left out: {made}");
    // The library is its documents, and it begins with two.
    let documents = rig.read("/api/collections/shop/docs/records").await;
    assert_eq!(documents["records"].as_array().map(Vec::len), Some(2), "{documents}");
    let has = files(&rig, "shop").await;
    assert!(DOCUMENTS.iter().all(|document| has.iter().any(|path| path == document)), "{has:?}");

    // The board is there to be written on, and the fleet's own file is the project's.
    let board = rig.read("/api/board/shop").await;
    assert!(board["columns"].as_array().is_some_and(|columns| !columns.is_empty()), "{board}");
    // Its fleet's blueprint is kept by the factory, and is nothing of its folder.
    let tree = rig.read("/api/projects/shop/tree").await;
    let fleet = brand::blueprint_file("fleet");
    assert!(
        !tree["entries"].as_array().unwrap().iter().any(|entry| entry["path"] == fleet.as_str()),
        "{tree}"
    );
    assert!(rig.placed("shop").join(&fleet).is_file());
    // The fleet is as its tables say by the time the answer comes: both tables are there to be changed.
    let has = tables(&rig, "shop").await;
    for table in ["agents", "board", "docs", "stages"] {
        assert!(has.iter().any(|name| name == table), "`{table}` among {has:?}");
    }
    let live = running(&rig, "shop").await;
    for blueprint in ["fleet", "std/board", "std/docs", "std/fleet"] {
        assert!(live.iter().any(|name| name == blueprint), "`{blueprint}` among {live:?}");
    }

    // Said nothing of it, a project is added alone, as it always was.
    let bare = rig.send("/api/projects", json!({ "name": "Depot" })).await;
    assert!(bare["set_up"].is_null(), "{bare}");
    assert_eq!(running(&rig, "depot").await, Vec::<String>::new());
    assert_eq!(tables(&rig, "depot").await, Vec::<String>::new());
    let tree = rig.read("/api/projects/depot/tree").await;
    assert!(
        !tree["entries"].as_array().unwrap().iter().any(|entry| entry["path"] == fleet.as_str()),
        "{tree}"
    );
    // Its files are the ones a project was always made with, its two documents among them.
    let has = files(&rig, "depot").await;
    assert!(DOCUMENTS.iter().all(|document| has.iter().any(|path| path == document)), "{has:?}");
}

#[tokio::test]
async fn a_project_is_handed_out_saying_whose_its_folder_is() {
    let rig = Rig::new().await;
    // Added with nothing said of a folder, the factory makes it one of its own.
    let made = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    assert_eq!(made["folder"], "own", "{made}");
    // Added with a folder, the project is that folder: one that is there, and one that is not yet.
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("README.md"), "# Theirs\n").unwrap();
    let taken = rig.send("/api/projects", json!({ "name": "Theirs", "path": theirs })).await;
    assert_eq!((&taken["folder"], &taken["brownfield"]), (&json!("given"), &json!(true)), "{taken}");
    let begun = rig.dir.join("begun");
    let new = rig.send("/api/projects", json!({ "name": "Begun", "path": begun, "set_up": ["board"] })).await;
    assert_eq!((&new["folder"], &new["brownfield"]), (&json!("given"), &json!(false)), "{new}");
    assert!(begun.join("README.md").is_file(), "the project is the folder that was said");
    // Each says the same wherever it is handed out.
    for (id, folder) in [("shop", "own"), ("theirs", "given"), ("begun", "given")] {
        assert_eq!(rig.read(&format!("/api/projects/{id}")).await["folder"], folder, "{id}");
    }
    let listed = rig.read("/api/projects").await;
    let said: Vec<(&str, &str)> = listed["projects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|project| (project["id"].as_str().unwrap(), project["folder"].as_str().unwrap()))
        .collect();
    assert_eq!(said, [("shop", "own"), ("theirs", "given"), ("begun", "given")], "{listed}");
    // Saying a folder is the owner's to do: somebody who may add a project and holds no more is refused,
    // and a path that is not whole is refused whoever asks.
    let member = crate::held::key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let asked = Some(json!({ "name": "Yard", "path": rig.dir.join("yard") }));
    let (status, refused) = rig.call(Method::POST, "/api/projects", Some(&member), asked).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, refused) = rig.post("/api/projects", json!({ "name": "Yard", "path": "yard" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(refused["error"]["message"], "a project's path must be absolute");
    // A folder that is another project's is refused, naming it.
    let (status, refused) = rig.post("/api/projects", json!({ "name": "Yard", "path": theirs })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(refused["error"]["message"], format!("{} is already a project", theirs.display()));
    assert!(!rig.dir.join("yard").exists(), "nothing was made for what was refused");
    let (status, none) = rig.get("/api/projects/yard").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{none}");
    // And one that had been refused its folder is added with another.
    let later = rig.send("/api/projects", json!({ "name": "Yard" })).await;
    assert_eq!(later["folder"], "own", "{later}");
}

#[tokio::test]
async fn what_a_new_project_may_start_with_is_one_list_that_the_daemon_hands_out() {
    let rig = Rig::new().await;
    let meta = rig.read("/api/meta").await;
    let starts = meta["project_starts"].as_array().expect("the list").clone();
    let names: Vec<&str> = starts.iter().map(|start| start["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["board", "fleet", "docs", "features", "timed", "ab", "pipelines", "environments", "flags"],
        "{meta}"
    );
    for start in &starts {
        // Each says what it is called, what it is and which blueprint it is; and all but
        // features, card times, A/B testing, pipelines, environments and flags, which are given where they are asked for,
        // are on unless said.
        for word in ["title", "said", "says", "blueprint"] {
            assert!(start[word].as_str().is_some_and(|said| !said.is_empty()), "{word} of {start}");
        }
        let asked_for = ["features", "timed", "ab", "pipelines", "environments", "flags"]
            .contains(&start["name"].as_str().unwrap());
        assert_eq!(start["on"], !asked_for, "{start}");
        let on_the_board = ["features", "timed", "ab"].contains(&start["name"].as_str().unwrap());
        // What it needs is something of the list, and it says why it needs it.
        for need in start["needs"].as_array().unwrap() {
            assert!(names.contains(&need.as_str().unwrap()), "{start}");
            assert!(start["why"].as_str().is_some_and(|why| !why.is_empty()), "{start}");
        }
        // Where a person is offered it: as the project is added, or on the page of something it needs.
        // Features, card times and A/B testing are offered on the board's; the rest as the project is added.
        let offered = start["offered"].as_str().expect("where it is offered");
        assert_eq!(offered, if on_the_board { "board" } else { "adding" }, "{start}");
        assert!(
            offered == "adding" || start["needs"].as_array().unwrap().contains(&json!(offered)),
            "{start}"
        );
    }
    assert_eq!(starts[1]["needs"], json!(["board"]), "a fleet works a board: {meta}");
    // Asked for by name, every one of them is what saying yes asks for.
    let all = rig.send("/api/projects", json!({ "name": "Every", "set_up": names })).await;
    let yes = rig.send("/api/projects", json!({ "name": "Yes", "set_up": true })).await;
    for name in &names {
        assert!(all["set_up"][name].is_string(), "{all}");
        let asked_for = ["features", "timed", "ab", "pipelines", "environments", "flags"].contains(name);
        assert_eq!(yes["set_up"][name].is_string(), !asked_for, "{yes}");
    }
    assert!(all["set_up"]["left"].is_null(), "{all}");
    let without = |running: Vec<String>| -> Vec<String> {
        running
            .into_iter()
            // What card times run (its table declared again as the board is) is theirs too.
            .filter(|blueprint| {
                blueprint != "std/features"
                    && blueprint != "std/ab"
                    && !blueprint.starts_with("std/timed")
                    && !blueprint.starts_with("std/pipeline")
                    && blueprint != "std/environments"
                    && !blueprint.starts_with("std/flags")
            })
            .collect()
    };
    assert_eq!(without(running(&rig, "every").await), running(&rig, "yes").await);
    assert!(running(&rig, "every").await.contains(&"std/features".to_string()));
    assert_eq!(starts[3]["needs"], json!(["board", "fleet"]), "{meta}");
}

#[tokio::test]
async fn features_are_given_where_they_are_asked_for_and_whoever_asks_for_one_without_them_is_told_how() {
    let rig = Rig::new().await;
    // Asked for with what they need, a project has the table, and what
    // becomes of a feature is said where its fleet reports.
    let made =
        rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board", "fleet", "features"] })).await;
    assert!(made["set_up"]["features"].is_string() && made["set_up"]["left"].is_null(), "{made}");
    assert!(tables(&rig, "shop").await.contains(&"features".to_string()));
    let instance =
        rig.read(&format!("/api/instances/{}", made["set_up"]["features"].as_str().unwrap())).await;
    assert_eq!(instance["instance"]["params"]["channel"], "shop", "{instance}");
    // Without a fleet they are left out, saying why and how they are given.
    let made = rig.send("/api/projects", json!({ "name": "Ledger", "set_up": ["board", "features"] })).await;
    let left = made["set_up"]["left"][0].as_str().unwrap();
    assert!(
        left.starts_with("`ledger` was not given features: a feature is planned by one of the fleet")
            && left.ends_with(&format!(
                "`{} apply std/features --project ledger --param project=ledger --param channel=ledger`",
                brand::CLI
            )),
        "{left}"
    );
    // Somebody who asks for a feature there is told the one command that gives the project its features.
    let asked = json!({ "fields": { "title": "Open the shop", "status": "backlog" } });
    let (status, refused) = rig
        .call(
            axum::http::Method::POST,
            "/api/collections/ledger/features/records",
            Some(&rig.token),
            Some(asked),
        )
        .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{refused}");
    assert_eq!(
        refused["error"]["message"],
        format!(
            "the project `ledger` has no collection called `features`. It is given them by `{} apply std/features --project ledger --param project=ledger --param channel=ledger`",
            brand::CLI
        )
    );
    // So is somebody who asks what it has of them, and what a library holds
    // of a project that was given none: the same refusal, and the command.
    for (table, gives) in [
        ("features", "apply std/features --project ledger --param project=ledger --param channel=ledger"),
        ("docs", "apply std/docs --project ledger"),
    ] {
        let (status, refused) = rig.get(&format!("/api/collections/ledger/{table}/records")).await;
        assert_eq!(status, axum::http::StatusCode::NOT_FOUND, "{refused}");
        assert_eq!(
            refused["error"]["message"],
            format!(
                "the project `ledger` has no collection called `{table}`. It is given them by `{} {gives}`",
                brand::CLI
            )
        );
    }
    let (_, refused) = rig.get("/api/collections/ledger/sprints/records").await;
    assert_eq!(refused["error"]["message"], "the project `ledger` has no collection called `sprints`");
    // A table no project starts with is refused as it always was.
    let (_, refused) = rig
        .call(
            axum::http::Method::POST,
            "/api/collections/ledger/sprints/records",
            Some(&rig.token),
            Some(json!({ "fields": {} })),
        )
        .await;
    assert_eq!(refused["error"]["message"], "the project `ledger` has no collection called `sprints`");
}

#[tokio::test]
async fn a_project_is_given_what_was_asked_for_by_name_and_nothing_else() {
    let rig = Rig::new().await;
    let fleet = brand::blueprint_file("fleet");

    // A board and no fleet: nothing of a fleet is written or run, and no document, since no library was asked for.
    let made = rig.send("/api/projects", json!({ "name": "Ledger", "set_up": ["board"] })).await;
    assert!(made["set_up"]["board"].is_string(), "{made}");
    assert!(made["set_up"]["fleet"].is_null() && made["set_up"]["docs"].is_null(), "{made}");
    assert!(made["set_up"]["left"].is_null(), "nothing asked for was left out: {made}");
    assert_eq!(running(&rig, "ledger").await, ["std/board", "std/cards"]);
    let has = files(&rig, "ledger").await;
    assert!(!has.iter().any(|path| *path == fleet || path.starts_with("docs/")), "{has:?}");
    // What it says of itself does not send anyone to a folder it has not got.
    let readme = rig.read("/api/projects/ledger/file?path=README.md").await;
    assert!(!readme["content"].as_str().unwrap().contains("docs/"), "{readme}");

    // A library alone: its two documents, as records, and nothing else running.
    let made = rig.send("/api/projects", json!({ "name": "Shelf", "set_up": ["docs"] })).await;
    assert!(made["set_up"]["docs"].is_string() && made["set_up"]["board"].is_null(), "{made}");
    assert_eq!(running(&rig, "shelf").await, ["std/docs"]);
    assert_eq!(tables(&rig, "shelf").await, ["docs"]);
    let documents = rig.read("/api/collections/shelf/docs/records").await;
    assert_eq!(documents["records"].as_array().map(Vec::len), Some(2), "{documents}");

    // Every one turned down is not the same as nothing said: the project has no document either.
    let made = rig.send("/api/projects", json!({ "name": "Empty", "set_up": [] })).await;
    assert_eq!(made["set_up"], json!({}), "{made}");
    assert_eq!(running(&rig, "empty").await, Vec::<String>::new());
    assert_eq!(tables(&rig, "empty").await, Vec::<String>::new());
    assert_eq!(files(&rig, "empty").await, ["README.md"]);

    // Named in another order, or twice, they are given in the list's order, once.
    let made =
        rig.send("/api/projects", json!({ "name": "Twice", "set_up": ["fleet", "board", "board"] })).await;
    assert!(made["set_up"]["board"].is_string() && made["set_up"]["fleet"].is_string(), "{made}");
    assert!(made["set_up"]["left"].is_null(), "{made}");
}

#[tokio::test]
async fn a_fleet_asked_for_with_no_board_is_left_out_and_said_to_be_with_why() {
    let rig = Rig::new().await;
    let made = rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["fleet", "docs"] })).await;
    assert_eq!(made["id"], "shop", "the project was added all the same: {made}");
    assert!(made["set_up"]["fleet"].is_null() && made["set_up"]["docs"].is_string(), "{made}");
    let left = made["set_up"]["left"].as_array().expect("what was left out");
    assert_eq!(left.len(), 1, "{made}");
    let said = left[0].as_str().unwrap();
    assert!(said.contains("was not given a fleet") && said.contains("a fleet works a board"), "{said}");
    assert!(said.contains("fleet init shop"), "it says how, once there is a board: {said}");
    // Nothing of a fleet was written or begun.
    assert_eq!(running(&rig, "shop").await, ["std/docs"]);
    assert!(!files(&rig, "shop").await.contains(&brand::blueprint_file("fleet")));
}

#[tokio::test]
async fn a_fleet_applied_by_hand_where_there_is_no_board_says_so_with_the_command_and_works_the_board_once_it_is_there()
 {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.send("/api/projects/shop/fleet", json!({})).await;
    let apply = json!({ "blueprint": "fleet", "project": "shop", "wait": true });
    // It is not refused: it comes up, and whoever applied it is told what it has nothing to work without.
    let applied = rig.send("/api/apply", apply.clone()).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let said: Vec<&str> = applied["said"]
        .as_array()
        .expect("something said")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert_eq!(
        said,
        [format!(
            "`shop` has no board yet, and a fleet works a board: until it has, nothing reaches what was just \
             applied. It is given by `{} apply std/board --project shop`",
            brand::CLI
        )]
    );
    // Given its board afterwards, in that order, the fleet works it: a card made ready is taken.
    rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart", "status": "ready" })).await;
    let taken = rig
        .event("board.item.moved", |event| event.data["id"] == card["id"] && event.data["to"] == "doing")
        .await;
    assert_eq!(taken.data["from"], "ready", "{:?}", taken.data);
    // Applied where there is one, nothing is said of a board.
    let again = rig.send("/api/apply", apply).await;
    assert!(again["said"].is_null(), "{again}");
    // And what works no board says nothing of one: a library, applied for a project with none.
    rig.send("/api/projects", json!({ "name": "Ledger" })).await;
    let library =
        rig.send("/api/apply", json!({ "blueprint": "std/docs", "project": "ledger", "wait": true })).await;
    assert!(library["said"].is_null(), "{library}");
}

#[tokio::test]
async fn a_request_that_names_something_no_project_starts_with_adds_nothing() {
    let rig = Rig::new().await;
    for wrong in [json!(["board", "garden"]), json!("board"), json!([7])] {
        let asked = json!({ "name": "Shop", "set_up": wrong });
        let (status, answer) = rig.post("/api/projects", asked).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
        let said = answer["error"]["message"].as_str().unwrap_or_default();
        assert!(said.contains("board, fleet, docs") || said.contains("`board`, `fleet`, `docs`"), "{answer}");
    }
    let (status, gone) = rig.get("/api/projects/shop").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{gone}");
}

/// The instances there ever were of a blueprint for a project, alive or not.
async fn ever(rig: &Rig, project: &str, blueprint: &str) -> Vec<String> {
    let all = rig.read(&format!("/api/instances?project={project}&limit=500")).await;
    all["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|instance| instance["blueprint"] == blueprint)
        .map(|instance| instance["id"].as_str().unwrap().to_string())
        .collect()
}

/// How many times an instance's steps were run, which is once for each
/// time it was applied: told from what its handlers ran by what each run
/// is of, since a handler's run comes when what it hears does.
async fn steps_run(rig: &Rig, instance: &str) -> usize {
    let shown = rig.read(&format!("/api/instances/{instance}")).await;
    shown["runs"].as_array().unwrap().iter().filter(|run| run["flow"] == "main").count()
}

#[tokio::test]
async fn a_project_that_is_there_is_given_what_it_may_start_with_and_asked_again_is_told_it_is_there() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board", "fleet"] })).await;
    assert!(!tables(&rig, "shop").await.contains(&"features".to_string()));

    // Given what it was not added with: what it needs is what it has had since it was added.
    let given = rig.send("/api/projects/shop/starts", json!({ "names": ["features"] })).await;
    let instance = given["features"].as_str().expect("its instance").to_string();
    assert!(given["left"].is_null() && given["there"].is_null(), "{given}");
    assert!(tables(&rig, "shop").await.contains(&"features".to_string()));
    // It is told what it is told when a project is added with it: where its fleet reports.
    let told = rig.read(&format!("/api/instances/{instance}")).await;
    assert_eq!(told["instance"]["params"]["channel"], "shop", "{told}");
    assert_eq!(told["instance"]["params"]["project"], "shop", "{told}");

    // Asked again, nothing is applied: it is said to be there, by the instance that is it.
    assert_eq!(steps_run(&rig, &instance).await, 1);
    let again = rig.send("/api/projects/shop/starts", json!({ "names": ["features", "board"] })).await;
    assert_eq!(again["features"], instance.as_str(), "{again}");
    assert_eq!(again["there"], json!(["board", "features"]), "in the list's order: {again}");
    assert!(again["left"].is_null(), "{again}");
    assert_eq!(ever(&rig, "shop", "std/features").await, [instance.as_str()]);
    assert_eq!(ever(&rig, "shop", "std/board").await.len(), 1);
    assert_eq!(steps_run(&rig, &instance).await, 1, "its steps were not run again for being asked again");

    // What looks after the features is stopped where what runs is: the table and what is in it stay,
    // and asked for again the project is given it anew, over what it kept.
    let feature = json!({ "fields": { "title": "Open the shop", "status": "building" } });
    let kept = rig.send("/api/collections/shop/features/records", feature).await["id"].clone();
    rig.send(&format!("/api/instances/{instance}/stop"), json!({})).await;
    assert!(!running(&rig, "shop").await.contains(&"std/features".to_string()));
    let records = rig.read("/api/collections/shop/features/records").await;
    assert_eq!(records["records"][0]["id"], kept, "{records}");
    let anew = rig.send("/api/projects/shop/starts", json!({ "names": ["features"] })).await;
    assert!(anew["features"].is_string() && anew["features"] != instance.as_str(), "{anew}");
    assert!(anew["there"].is_null() && anew["left"].is_null(), "{anew}");
    assert!(running(&rig, "shop").await.contains(&"std/features".to_string()));
    let records = rig.read("/api/collections/shop/features/records").await;
    assert_eq!(records["records"].as_array().map(Vec::len), Some(1), "{records}");
    assert_eq!(records["records"][0]["id"], kept, "{records}");
}

#[tokio::test]
async fn what_a_project_is_given_later_is_left_out_for_the_lack_of_what_it_needs_unless_that_is_given_with_it()
 {
    let rig = Rig::new().await;
    // A board and no fleet: features are left out, saying why and how they are given, as when a project is added.
    rig.send("/api/projects", json!({ "name": "Ledger", "set_up": ["board"] })).await;
    let given = rig.send("/api/projects/ledger/starts", json!({ "names": ["features"] })).await;
    assert!(given["features"].is_null(), "{given}");
    let left = given["left"][0].as_str().unwrap_or_default();
    assert!(
        left.starts_with("`ledger` was not given features: a feature is planned by one of the fleet")
            && left.contains("it has not got a fleet")
            && left.ends_with(&format!(
                "`{} apply std/features --project ledger --param project=ledger --param channel=ledger`",
                brand::CLI
            )),
        "{given}"
    );
    assert_eq!(given["left"].as_array().map(Vec::len), Some(1), "{given}");
    assert!(!tables(&rig, "ledger").await.contains(&"features".to_string()));
    assert_eq!(ever(&rig, "ledger", "std/features").await, Vec::<String>::new());

    // Named with what it needs, in any order, each is given in the list's order: the board is
    // there, the fleet is given now, and the features after it.
    let given =
        rig.send("/api/projects/ledger/starts", json!({ "names": ["features", "fleet", "board"] })).await;
    assert!(given["fleet"].is_string() && given["features"].is_string(), "{given}");
    assert_eq!(given["there"], json!(["board"]), "{given}");
    assert!(given["left"].is_null(), "{given}");
    assert!(ledger_has(&rig, &["agents", "board", "features", "stages"]).await);

    // A project added alone is given all of it the same way.
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let given = rig.send("/api/projects/depot/starts", json!({ "names": ["board", "fleet", "docs"] })).await;
    for name in ["board", "fleet", "docs"] {
        assert!(given[name].is_string(), "{given}");
    }
    assert!(given["there"].is_null() && given["left"].is_null(), "{given}");
    // And a list with nothing in it asks for nothing.
    assert_eq!(rig.send("/api/projects/depot/starts", json!({ "names": [] })).await, json!({}));
}

/// Whether the ledger has each of `wanted` among its tables.
async fn ledger_has(rig: &Rig, wanted: &[&str]) -> bool {
    let has = tables(rig, "ledger").await;
    wanted.iter().all(|table| has.iter().any(|name| name == table))
}

#[tokio::test]
async fn giving_a_project_something_later_is_applying_and_is_refused_whoever_may_not_apply() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board", "fleet"] })).await;
    let before = (running(&rig, "shop").await, tables(&rig, "shop").await);
    // What a member holds, but for applying: they read blueprints and may not run one.
    let less: Vec<&str> = kernel::principal::MEMBER_SCOPES
        .iter()
        .map(|scope| if *scope == "blueprints" { "blueprints:read" } else { scope })
        .collect();
    let narrow = crate::held::key(&rig, &less).await;
    let asked = Some(json!({ "names": ["features"] }));
    let (status, refused) = rig.call(Method::POST, "/api/projects/shop/starts", Some(&narrow), asked).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(
        refused["error"]["message"], "narrow is not allowed to do this (needs `blueprints:run`)",
        "{refused}"
    );
    // Refused before anything is said of the project or of what was named: one that is not there,
    // and a name that is nothing, are answered the same.
    for (path, names) in [
        ("/api/projects/nowhere/starts", json!(["features"])),
        ("/api/projects/shop/starts", json!(["garden"])),
    ] {
        let (status, refused) =
            rig.call(Method::POST, path, Some(&narrow), Some(json!({ "names": names }))).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    }
    // Nothing was applied for them.
    assert_eq!((running(&rig, "shop").await, tables(&rig, "shop").await), before);
    assert_eq!(ever(&rig, "shop", "std/features").await, Vec::<String>::new());
    // What a member holds does give it.
    let member = crate::held::key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let asked = Some(json!({ "names": ["features"] }));
    let (status, given) = rig.call(Method::POST, "/api/projects/shop/starts", Some(&member), asked).await;
    assert_eq!(status, StatusCode::OK, "{given}");
    assert!(given["features"].is_string() && given["left"].is_null(), "{given}");
}

#[tokio::test]
async fn what_a_project_is_given_later_is_applied_as_whoever_asked_and_does_no_more_than_they_may() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["board", "fleet"] })).await;
    let before = (running(&rig, "shop").await, tables(&rig, "shop").await);
    // Somebody who may apply a blueprint and may not write a project: declaring a table for one takes that,
    // and the features' blueprint is applied as them, so it is refused where it declares its table.
    let narrow =
        crate::held::key(&rig, &["projects:read", "records", "board", "channels", "agents", "blueprints"])
            .await;
    let asked = json!({ "names": ["features"] });
    let (status, given) =
        rig.call(Method::POST, "/api/projects/shop/starts", Some(&narrow), Some(asked.clone())).await;
    // The asking was theirs to make, and what came of it is said, with why and how it is given by hand.
    assert_eq!(status, StatusCode::OK, "{given}");
    assert!(given["features"].is_null() && given["there"].is_null(), "{given}");
    let left = given["left"][0].as_str().unwrap_or_default();
    assert!(
        left.starts_with("`shop` was not given features: ")
            && left.contains("narrow may not use `projects:write`")
            && left.ends_with(&format!(
                "By hand: `{} apply std/features --project shop --param project=shop --param channel=shop`",
                brand::CLI
            )),
        "{given}"
    );
    assert_eq!(given["left"].as_array().map(Vec::len), Some(1), "{given}");
    // Nothing was declared for them, and nothing of it is left running.
    assert_eq!((running(&rig, "shop").await, tables(&rig, "shop").await), before);
    // What they began is in nobody's way: somebody who may is given it.
    let given = rig.send("/api/projects/shop/starts", asked).await;
    assert!(given["features"].is_string() && given["left"].is_null() && given["there"].is_null(), "{given}");
    assert!(tables(&rig, "shop").await.contains(&"features".to_string()));
}

#[tokio::test]
async fn a_project_that_is_not_there_is_given_nothing_and_neither_is_one_asked_for_what_no_project_starts_with()
 {
    let rig = Rig::new().await;
    let (status, none) = rig.post("/api/projects/nowhere/starts", json!({ "names": ["board"] })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{none}");
    assert_eq!(none["error"]["message"], "there is no project `nowhere`");

    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let (status, wrong) =
        rig.post("/api/projects/shop/starts", json!({ "names": ["board", "garden"] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}");
    assert_eq!(
        wrong["error"]["message"],
        "`names` names \"garden\", which is nothing a project starts with. It may name: board, fleet, docs, features, timed, ab, pipelines, environments, flags"
    );
    // Said some other way than as a list of names, it is told what is asked.
    for body in [json!({}), json!({ "names": "board" }), json!({ "names": true })] {
        let (status, wrong) = rig.post("/api/projects/shop/starts", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{wrong}");
        assert_eq!(
            wrong["error"]["message"],
            "`names` is needed: a list of what the project is to be given (`board`, `fleet`, `docs`, `features`, `timed`, `ab`, `pipelines`, `environments`, `flags`)"
        );
    }
    // Nothing was given it, the board named before what was wrong included.
    assert_eq!(running(&rig, "shop").await, Vec::<String>::new());
    assert_eq!(tables(&rig, "shop").await, Vec::<String>::new());
}

#[tokio::test]
async fn whoever_may_add_a_project_and_may_not_apply_a_blueprint_is_given_the_project_alone_and_told_why() {
    let rig = Rig::new().await;
    // What an agent holds: it may add a project, and may not apply.
    let asked = json!({ "name": "narrow", "scopes": kernel::principal::AGENT_SCOPES });
    let token = rig.send("/api/credentials", asked).await["token"].as_str().unwrap().to_string();
    let body = Some(json!({ "name": "Shop", "set_up": true }));
    let (status, made) = rig.call(Method::POST, "/api/projects", Some(&token), body).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made["id"], "shop");

    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(left.contains("`blueprints:run`") && left.contains("narrow"), "{made}");
    assert!(left.contains("apply std/board --project shop") && left.contains("fleet init shop"), "{made}");
    assert!(
        left.contains("a board, a fleet and docs") && left.contains("apply std/docs --project shop"),
        "{made}"
    );
    assert_eq!(made["set_up"]["left"].as_array().map(Vec::len), Some(1), "said once, for all of it: {made}");
    assert!(made["set_up"]["board"].is_null() && made["set_up"]["fleet"].is_null(), "{made}");
    assert!(made["set_up"]["docs"].is_null(), "{made}");
    // Nothing was applied for them, and nothing written into the project that they did not write.
    assert_eq!(running(&rig, "shop").await, Vec::<String>::new());
    assert_eq!(tables(&rig, "shop").await, Vec::<String>::new());
    let tree = rig.read("/api/projects/shop/tree").await;
    let fleet = brand::blueprint_file("fleet");
    assert!(
        !tree["entries"].as_array().unwrap().iter().any(|entry| entry["path"] == fleet.as_str()),
        "{tree}"
    );
}

#[tokio::test]
async fn a_folder_that_came_with_a_fleet_blueprint_of_its_own_is_given_a_board_and_its_blueprint_is_not_applied()
 {
    let rig = Rig::new().await;
    // Somebody else's folder: its fleet blueprint is theirs, and would start a service if it were applied.
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    let fleet = brand::blueprint_file("fleet");
    let text = "blueprint: fleet\nsteps:\n  up:\n    service: { name: theirs, cmd: [sleep, '600'] }\n";
    std::fs::write(theirs.join(&fleet), text).unwrap();

    let made = rig.send("/api/projects", json!({ "name": "Theirs", "path": theirs, "set_up": true })).await;
    assert_eq!(made["brownfield"], true, "{made}");
    assert!(made["set_up"]["board"].is_string() && made["set_up"]["fleet"].is_null(), "{made}");
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(
        left.contains("a fleet blueprint of its own") && left.contains("apply fleet --project theirs"),
        "{made}"
    );

    // The board, what looks after its cards, and the library: nothing of a fleet.
    assert_eq!(running(&rig, "theirs").await, ["std/board", "std/cards", "std/docs"]);
    // Their file is as they wrote it.
    assert_eq!(std::fs::read_to_string(theirs.join(&fleet)).unwrap(), text);
    let fleet = rig.read("/api/fleet").await;
    assert_eq!(fleet["services"].as_array().map(Vec::len), Some(0), "{fleet}");
}

#[tokio::test]
async fn a_folder_taken_up_with_no_fleet_blueprint_is_given_one_and_it_is_applied() {
    let rig = Rig::new().await;
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("README.md"), "# Theirs\n").unwrap();

    let made = rig.send("/api/projects", json!({ "name": "Theirs", "path": theirs, "set_up": true })).await;
    assert_eq!(made["brownfield"], true, "{made}");
    assert!(made["set_up"]["board"].is_string() && made["set_up"]["fleet"].is_string(), "{made}");
    assert!(made["set_up"]["left"].is_null(), "{made}");
    // The one file it was given is its fleet's, kept by the factory, and nothing was written in its folder.
    assert!(rig.placed("theirs").join(brand::blueprint_file("fleet")).is_file());
    assert!(!theirs.join(brand::blueprint_file("fleet")).exists());
    let live = running(&rig, "theirs").await;
    assert!(live.iter().any(|name| name == "std/fleet"), "{live:?}");
}

#[tokio::test]
async fn a_project_let_go_of_and_taken_up_again_has_its_fleet_back_since_its_blueprint_is_the_factorys_own() {
    let rig = Rig::new().await;
    // A folder with something in it, taken up and given the usual.
    let theirs = rig.dir.join("theirs");
    std::fs::create_dir_all(&theirs).unwrap();
    std::fs::write(theirs.join("README.md"), "# Theirs\n").unwrap();
    let asked = json!({ "name": "Theirs", "path": theirs, "set_up": true });
    let first = rig.send("/api/projects", asked.clone()).await;
    assert!(first["set_up"]["fleet"].is_string(), "{first}");
    let kept = rig.placed("theirs").join(brand::blueprint_file("fleet"));
    let written = std::fs::read_to_string(&kept).unwrap();

    // Let go of: what ran for it is stopped, and the project forgotten. Its folder is as it was.
    for instance in [first["set_up"]["fleet"].as_str().unwrap(), first["set_up"]["board"].as_str().unwrap()] {
        rig.send(&format!("/api/instances/{instance}/stop"), json!({})).await;
    }
    let (status, gone) = rig.call(Method::DELETE, "/api/projects/theirs", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{gone}");
    // Its folder is as it was, and its own blueprints went into the Trash with it.
    assert!(!kept.exists() && !theirs.join(brand::blueprint_file("fleet")).exists());

    // Taken up again, it is given its fleet again, where the factory keeps it, and it is applied.
    let again = rig.send("/api/projects", asked).await;
    assert_eq!(again["brownfield"], true, "{again}");
    assert!(again["set_up"]["board"].is_string() && again["set_up"]["fleet"].is_string(), "{again}");
    assert!(again["set_up"]["left"].is_null(), "{again}");
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), written);
    let live = running(&rig, "theirs").await;
    assert!(live.iter().any(|name| name == "std/fleet"), "{live:?}");
}

#[tokio::test]
async fn a_project_forgotten_with_its_fleet_up_says_what_still_runs_for_it_to_whoever_may_read_each_kind() {
    let rig = Rig::new().await;
    let key = |scopes: &'static [&'static str]| {
        let rig = &rig;
        async move {
            let made =
                rig.send("/api/credentials", json!({ "name": scopes.join("-"), "scopes": scopes })).await;
            made["token"].as_str().unwrap().to_string()
        }
    };
    let mut made = Vec::new();
    for name in ["Shop", "Ledger", "Depot"] {
        made.push(rig.send("/api/projects", json!({ "name": name, "set_up": true })).await);
    }
    let forget = |project: &'static str, token: String| {
        let rig = &rig;
        async move {
            let (status, gone) =
                rig.call(Method::DELETE, &format!("/api/projects/{project}"), Some(&token), None).await;
            assert_eq!(status, StatusCode::OK, "{gone}");
            gone["running"].clone()
        }
    };
    let sorted = |listed: &serde_json::Value| {
        let mut words: Vec<String> =
            listed.as_array().expect("a list").iter().map(|one| one.as_str().unwrap().to_string()).collect();
        words.sort();
        words
    };

    // Whoever may forget a project and read nothing else is told nothing of what runs.
    let running = forget("ledger", key(&["projects"]).await).await;
    assert_eq!(running, json!({}), "a key that reads no instance, agent or channel");
    // One that reads what is running, and no agent or channel, is told that and no more.
    let running = forget("depot", key(&["projects", "blueprints:read"]).await).await;
    assert!(running["agents"].is_null() && running["channels"].is_null(), "{running}");
    assert_eq!(running["instances"].as_array().map(Vec::len), Some(3), "{running}");

    // The owner is told all of it, and only this project's: what was
    // applied for it, by its id, its agents and its channel.
    let running = forget("shop", rig.token.clone()).await;
    let applied = running["instances"].as_array().expect("what was applied for it");
    let mut blueprints: Vec<&str> = applied.iter().map(|one| one["blueprint"].as_str().unwrap()).collect();
    blueprints.sort_unstable();
    assert_eq!(blueprints, ["fleet", "std/board", "std/docs"], "{running}");
    for start in ["board", "fleet", "docs"] {
        let id = &made[0]["set_up"][start];
        assert!(applied.iter().any(|one| &one["id"] == id), "{start} of {running}");
    }
    assert_eq!(sorted(&running["agents"]), ["shop-architect-1", "shop-worker-1"], "{running}");
    assert_eq!(sorted(&running["channels"]), ["shop"], "{running}");

    // Stopped as it says, nothing of the project is left running, and its agents are up no longer.
    for one in applied {
        rig.send(&format!("/api/instances/{}/stop", one["id"].as_str().unwrap()), json!({})).await;
    }
    assert!(running_at_all(&rig, "shop").await.is_empty());
    let agents = rig.read("/api/agents").await;
    let up = |name: &str| {
        agents["agents"].as_array().unwrap().iter().any(|one| one["name"] == name && one["active"] == true)
    };
    assert!(!up("shop-architect-1") && !up("shop-worker-1"), "{agents}");
}

/// The id of everything alive for a project.
async fn running_at_all(rig: &Rig, project: &str) -> Vec<String> {
    let live = rig.read(&format!("/api/instances?live=true&project={project}")).await;
    live["instances"].as_array().unwrap().iter().map(|one| one["id"].as_str().unwrap().to_string()).collect()
}

#[tokio::test]
async fn what_can_be_held_is_one_list_by_area_that_the_daemon_hands_out_with_what_an_agent_usually_holds() {
    let rig = Rig::new().await;
    let meta = rig.read("/api/meta").await;
    let areas = meta["scopes"].as_array().expect("the list").clone();
    let board = areas.iter().find(|area| area["area"] == "board").expect("the board's");
    let verbs: Vec<&str> =
        board["verbs"].as_array().unwrap().iter().map(|verb| verb["verb"].as_str().unwrap()).collect();
    assert_eq!(verbs, ["read", "write"], "{board}");
    // Each thing that may be done says what it allows, and an area that is one thing says what holding it does.
    for area in &areas {
        let parts = area["verbs"].as_array().unwrap();
        assert!(
            parts.iter().all(|verb| verb["does"].as_str().is_some_and(|does| !does.is_empty())),
            "{area}"
        );
        assert!(!parts.is_empty() || area["does"].as_str().is_some_and(|does| !does.is_empty()), "{area}");
    }
    let run = areas.iter().find(|area| area["area"] == "blueprints").unwrap();
    assert_eq!(run["verbs"][1]["verb"], "run", "{run}");
    assert!(areas.iter().any(|area| area["area"] == "shell" && area["verbs"] == json!([])), "{meta}");
    // What an agent holds where its slot says nothing: the list a page shows as the usual one.
    let usual: Vec<&str> =
        meta["agent_scopes"].as_array().unwrap().iter().map(|scope| scope.as_str().unwrap()).collect();
    assert_eq!(usual, kernel::principal::AGENT_SCOPES, "{meta}");
}

#[tokio::test]
async fn a_project_given_its_environments_has_each_empty_and_what_else_it_starts_with_runs_under_none() {
    let rig = Rig::new().await;
    let asked = json!({ "name": "Shop", "set_up": ["board", "fleet", "environments"] });
    let made = rig.send("/api/projects", asked).await;
    let given = &made["set_up"];
    assert!(given["environments"].is_string() && given["left"].is_null(), "{made}");
    // Its environments are made, each with no variables yet, by whoever added it.
    let read = rig.read("/api/projects/shop/environments").await;
    let environments = read["environments"].as_array().unwrap();
    let names: Vec<&str> = environments.iter().map(|one| one["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["dev", "prod"], "{read}");
    for one in environments {
        assert!(one["vars"].as_object().is_none_or(|vars| vars.is_empty()), "{one}");
    }
    // What else it starts with was applied under none, as it always is: its board and its fleet run.
    let instances = rig.read("/api/instances?project=shop&live=true").await;
    for one in instances["instances"].as_array().unwrap() {
        assert_eq!(one["environment"].as_str().unwrap_or_default(), "", "{one}");
    }
    let live = running(&rig, "shop").await;
    for blueprint in ["fleet", "std/board", "std/fleet"] {
        assert!(live.iter().any(|name| name == blueprint), "`{blueprint}` among {live:?}");
    }
    // What made them holds nothing and is over: the environments are what the project has of it.
    assert!(!live.iter().any(|name| name == "std/environments"), "{live:?}");
    // Anything applied for it from now on says which it runs under.
    let body = json!({ "blueprint": "std/docs", "project": "shop" });
    let (status, refused) = rig.call(Method::POST, "/api/apply", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("say one of dev, prod"), "{refused}");
    // Given again, or applied again under one of them, it makes nothing more.
    let later = rig.send("/api/projects/shop/starts", json!({ "names": ["environments"] })).await;
    assert!(later["environments"].is_string() && later["left"].is_null(), "{later}");
    let again = json!({
        "blueprint": "std/environments", "project": "shop", "environment": "dev", "key": "again",
        "params": { "names": ["dev", "prod", "dev"] }, "wait": true,
    });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&rig.token), Some(again)).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    let read = rig.read("/api/projects/shop/environments").await;
    assert_eq!(read["environments"].as_array().map(Vec::len), Some(2), "{read}");

    // Whoever may apply and may not make an environment by hand is left without them, saying why.
    let scopes = ["projects", "blueprints", "board", "channels", "records", "agents", "fleet"];
    let token = rig.send("/api/credentials", json!({ "name": "no-shell", "scopes": scopes })).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    let body = Some(json!({ "name": "Depot", "set_up": ["environments"] }));
    let (status, made) = rig.call(Method::POST, "/api/projects", Some(&token), body).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(left.starts_with("`depot` was not given environments: ") && left.contains("shell"), "{made}");
    let read = rig.read("/api/projects/depot/environments").await;
    assert_eq!(read["environments"], json!([]), "{read}");
}

#[tokio::test]
async fn what_a_project_starts_with_is_set_up_as_it_is_added_by_what_each_blueprint_takes() {
    let rig = Rig::new().await;
    // What each may be set up with is its blueprint's params, less what it is told of the project.
    let settings = rig.read("/api/starts").await;
    let of = |name: &str| -> Vec<String> {
        let start =
            settings["starts"].as_array().unwrap().iter().find(|start| start["name"] == name).unwrap();
        start["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|param| param["name"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(of("board"), ["columns", "store", "database"], "{settings}");
    assert_eq!(of("docs"), ["folder"], "{settings}");
    assert_eq!(of("environments"), ["names"], "{settings}");
    // The fleet's is the text a project's own is written from, and says its project and its channel itself.
    let fleet = of("fleet");
    assert!(fleet.contains(&"planned".to_string()) && fleet.contains(&"push".to_string()), "{fleet:?}");
    assert!(!fleet.contains(&"project".to_string()) && !fleet.contains(&"channel".to_string()), "{fleet:?}");
    assert!(!of("features").contains(&"project".to_string()), "{settings}");
    let board = &settings["starts"][0]["params"][0];
    assert_eq!(board["default"], json!(["backlog", "ready", "doing", "review", "done"]), "{board}");

    // Set up as it is added: each is handed what was said of it, and what was not goes by its text.
    let asked = json!({ "name": "Shop", "set_up": [
        { "name": "board", "with": { "columns": ["todo", "ready", "doing", "review", "done"] } },
        { "name": "fleet", "with": { "planned": "ready" } },
        "docs",
        { "name": "environments", "with": { "names": ["staging"] } },
    ] });
    let made = rig.send("/api/projects", asked).await;
    assert!(made["set_up"]["left"].is_null(), "{made}");
    let board = rig.read("/api/board/shop").await;
    let columns: Vec<&str> =
        board["columns"].as_array().unwrap().iter().map(|one| one.as_str().unwrap()).collect();
    assert_eq!(columns, ["todo", "ready", "doing", "review", "done"], "{board}");
    let fleet = rig.read(&format!("/api/instances/{}", made["set_up"]["fleet"].as_str().unwrap())).await;
    assert_eq!(fleet["instance"]["params"]["planned"], "ready", "{fleet}");
    assert_eq!(fleet["instance"]["params"]["project"], "shop", "{fleet}");
    let read = rig.read("/api/projects/shop/environments").await;
    assert_eq!(read["environments"].as_array().unwrap().len(), 1, "{read}");
    assert_eq!(read["environments"][0]["name"], "staging", "{read}");

    // What it is told of the project is not said here, and nothing is added for asking.
    let body = json!({ "name": "Depot", "set_up": [{ "name": "features", "with": { "project": "shop" } }] });
    let (status, refused) = rig.post("/api/projects", body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("`project` of `features` is the project's"), "{refused}");
    let (_, listed) = rig.call(Method::GET, "/api/projects/depot", Some(&rig.token), None).await;
    assert!(listed["error"].is_object(), "{listed}");
    // A param its blueprint does not take is its apply's to refuse: it is left out, saying so, and the rest is given.
    let body = json!({ "name": "Ledger", "set_up": ["board", { "name": "docs", "with": { "shelf": "x" } }] });
    let made = rig.send("/api/projects", body).await;
    let left = made["set_up"]["left"][0].as_str().unwrap_or_default();
    assert!(
        made["set_up"]["board"].is_string() && left.starts_with("`ledger` was not given docs: "),
        "{made}"
    );
    assert!(left.contains("shelf"), "{left}");
    // And a project that is there is given one set up the same way.
    let later = rig
        .send(
            "/api/projects/ledger/starts",
            json!({ "names": [{ "name": "docs", "with": { "folder": "notes" } }] }),
        )
        .await;
    let docs = rig.read(&format!("/api/instances/{}", later["docs"].as_str().unwrap())).await;
    assert_eq!(docs["instance"]["params"]["folder"], "notes", "{docs}");
}
