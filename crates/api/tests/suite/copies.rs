//! What a copy of a factory must never do: reach back into the factory it
//! was made from, through a repository, a store, a directory or an agent's
//! work; and what becomes of an arrival that was cut short, or of a
//! snapshot that was not made as a factory makes one.

use std::path::Path;

use api::{Config, Node};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::snapshots::{
    Shared, bodies, counted, files_under, holds, refused, sign_in, stop_instance, taken, titles,
};

/// Unix's alone: what is copied here has links in it, made as Unix makes
/// them, and a pipe with a name, which only Unix has.
#[cfg(unix)]
#[tokio::test]
async fn a_copied_repository_cannot_reach_the_working_directories_of_the_one_it_was_copied_from() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Web" })).await;
    let web = prod.dir.join("data").join("projects").join("web");
    // A repository with a lane open: a working directory of its own under
    // `.lanes`, which the repository knows by its whole path. (Made by
    // hand, as `git worktree add` leaves it, so that it is there whether
    // or not this device has git.)
    let git = web.join(".git");
    for dir in ["objects", "refs/heads/worktrees", "worktrees/web-1"] {
        std::fs::create_dir_all(git.join(dir)).unwrap();
    }
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git.join("refs/heads/worktrees/tidy"), "0000000000000000000000000000000000000001\n")
        .unwrap();
    std::fs::write(git.join("refs/heads/lane-web-1"), "0000000000000000000000000000000000000002\n").unwrap();
    let lane = web.join(".lanes/web-1");
    std::fs::create_dir_all(&lane).unwrap();
    std::fs::write(git.join("worktrees/web-1/gitdir"), format!("{}\n", lane.join(".git").display())).unwrap();
    std::fs::write(lane.join(".git"), format!("gitdir: {}\n", git.join("worktrees/web-1").display()))
        .unwrap();
    std::fs::write(lane.join("work.txt"), "half done, in prod").unwrap();
    // A directory whose repository is somewhere else altogether.
    std::fs::create_dir_all(web.join("vendored")).unwrap();
    std::fs::write(web.join("vendored/.git"), format!("gitdir: {}\n", git.join("worktrees/web-1").display()))
        .unwrap();
    std::fs::write(web.join("vendored/lib.txt"), "vendored").unwrap();
    // Links: one within the project, one out of it, one into the factory
    // by its whole path; and something that is no file at all.
    let outside = shared.0.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "not the project's").unwrap();
    std::os::unix::fs::symlink("README.md", web.join("readme-link")).unwrap();
    std::os::unix::fs::symlink(&outside, web.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(web.join("docs"), web.join("back-to-prod")).unwrap();
    let pipe = std::process::Command::new("mkfifo").arg(web.join("pipe")).status();

    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    let left: Vec<(&str, &str)> = manifest["left_out"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| (file["path"].as_str().unwrap(), file["why"].as_str().unwrap()))
        .collect();
    let why =
        |path: &str| left.iter().find(|(left, _)| *left == path).map(|(_, why)| *why).unwrap_or_default();
    assert!(why("web/.lanes").contains("working directories of lanes"), "{left:?}");
    assert!(why("web/.git/worktrees").contains("names directories of the factory copied"), "{left:?}");
    assert!(why("web/vendored/.git").contains("repository is somewhere else"), "{left:?}");
    assert!(why("web/back-to-prod").contains("a link into the factory copied"), "{left:?}");
    if pipe.is_ok_and(|made| made.success()) {
        assert!(why("web/pipe").contains("not a file, a folder or a link"), "{left:?}");
    }

    for copy in [
        to.join("projects/web"),
        shared.copy("dev2", &to).await.dir.join("data").join("projects").join("web"),
    ] {
        // The repository and its branches came, one called `worktrees` among them;
        assert!(copy.join(".git/HEAD").is_file() && copy.join(".git/refs/heads/lane-web-1").is_file());
        assert!(copy.join(".git/refs/heads/worktrees/tidy").is_file());
        // its list of working directories, and the directories, did not.
        assert!(!copy.join(".git/worktrees").exists() && !copy.join(".lanes").exists());
        assert!(copy.join("vendored/lib.txt").is_file() && !copy.join("vendored/.git").exists());
        // A link is a link still, and was not followed out of the project.
        assert_eq!(std::fs::read_link(copy.join("readme-link")).unwrap(), Path::new("README.md"));
        assert_eq!(std::fs::read_link(copy.join("elsewhere")).unwrap(), outside);
        assert!(std::fs::symlink_metadata(copy.join("back-to-prod")).is_err() && !copy.join("pipe").exists());
        // Nothing in the copy says where the other factory's files are.
        let prod_files = web.to_str().unwrap();
        for (path, bytes) in files_under(&copy) {
            assert!(!holds(&bytes, prod_files), "{} names {prod_files}", path.display());
        }
    }
    assert!(files_under(&to).iter().all(|(_, bytes)| !holds(bytes, "not the project's")));
    assert_eq!(std::fs::read_to_string(lane.join("work.txt")).unwrap(), "half done, in prod");
}

#[tokio::test]
async fn a_board_kept_by_a_store_service_has_no_store_in_a_copy_and_never_the_other_factorys() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let file = prod.dir.join("store/cards.db");
    let store = api::store::StoreServer::start(&file, "127.0.0.1:0".parse().unwrap()).await.unwrap();
    prod.send("/api/projects", json!({ "name": "Shop" })).await;
    let params =
        json!({ "store": "service", "database": store.url(), "columns": ["backlog", "doing", "done"] });
    let board = json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true });
    assert_eq!(prod.send("/api/apply", board.clone()).await["finished"]["status"], "ok");
    prod.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;

    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    assert_eq!(manifest["left_out"]["stores"], json!([{ "project": "shop", "collection": "board" }]));
    // In the copy the board is declared, says where it is not, and reads
    // and writes nothing: least of all the other factory's cards.
    let dev = shared.copy("dev2", &to).await;
    let told = dev.node().arrived.clone().unwrap().said().join("\n");
    assert!(
        told.contains("The table `board` of `shop` was kept by a store service of that factory"),
        "{told}"
    );
    assert!(told.contains("--param store=service --param database=<its url>"), "{told}");
    let declared = dev.read("/api/collections?project=shop").await;
    assert_eq!(
        (&declared["collections"][0]["store"], &declared["collections"][0]["at"]),
        (&json!("service"), &Value::Null)
    );
    let (status, said) = dev.get("/api/board/shop").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{said}");
    assert!(said["error"]["message"].as_str().unwrap().contains("has no store here"), "{said}");
    assert!(said["error"]["message"].as_str().unwrap().contains("started from a copy of another"), "{said}");
    let (status, said) = dev.post("/api/board/shop/items", json!({ "title": "Only in dev" })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{said}");
    // Declared again without saying where, it still has none.
    let again = json!({ "store": "service", "columns": ["backlog", "doing", "done"] });
    let applied = dev
        .send(
            "/api/apply",
            json!({ "blueprint": "std/board", "project": "shop", "params": again, "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert_eq!(
        titles(&prod.read("/api/board/shop").await),
        ["Sell mugs"],
        "the other's cards are as they were"
    );

    // Given a store of its own, it is a board again, and its own.
    let own = api::store::StoreServer::start(&dev.dir.join("store/cards.db"), "127.0.0.1:0".parse().unwrap())
        .await
        .unwrap();
    let params =
        json!({ "store": "service", "database": own.url(), "columns": ["backlog", "doing", "done"] });
    let applied = dev
        .send(
            "/api/apply",
            json!({ "blueprint": "std/board", "project": "shop", "params": params, "wait": true }),
        )
        .await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    dev.send("/api/board/shop/items", json!({ "title": "Only in dev" })).await;
    assert_eq!(titles(&dev.read("/api/board/shop").await), ["Only in dev"]);
    assert_eq!(titles(&prod.read("/api/board/shop").await), ["Sell mugs"]);
    for rig in [&dev, &prod] {
        for instance in rig.read("/api/instances?live=true&roots=true").await["instances"].as_array().unwrap()
        {
            stop_instance(rig, instance["id"].as_str().unwrap()).await;
        }
    }
}

#[tokio::test]
async fn a_project_in_a_directory_outside_the_factory_arrives_empty_and_is_named() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let site = shared.0.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(site.join("README.md"), "# The site, where it always was\n").unwrap();
    prod.send("/api/projects", json!({ "name": "Site", "path": site })).await;
    // Its board is kept as files, in that directory.
    let params = json!({ "store": "files", "columns": ["backlog", "done"] });
    let board = json!({ "blueprint": "std/board", "project": "site", "params": params, "wait": true });
    assert_eq!(prod.send("/api/apply", board).await["finished"]["status"], "ok");
    prod.send("/api/board/site/items", json!({ "title": "Redo the front page" })).await;
    assert!(site.join("board/1.md").is_file());

    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    let left = &manifest["left_out"]["projects"][0];
    assert_eq!((&left["id"], &left["directory"]), (&json!("site"), &json!(site.to_str().unwrap())));
    assert!(left["why"].as_str().unwrap().contains("two factories never share a directory"), "{left}");
    assert_eq!(left["kept_as_files"], json!(["board"]));
    assert_eq!(manifest["holds"]["projects"], json!([]));
    assert!(files_under(&to).iter().all(|(_, bytes)| !holds(bytes, "where it always was")));

    // It is a project in the new factory, in an empty directory of that
    // factory's own, and whoever started it is told which and why.
    let dev = shared.copy("dev2", &to).await;
    let here = dev.dir.join("data").join("projects").join("site");
    let said = dev.read("/api/projects/site").await["path"].as_str().unwrap().to_string();
    assert_eq!(std::path::Path::new(&said), here);
    assert!(here.is_dir() && std::fs::read_dir(&here).unwrap().next().is_none());
    assert_eq!(dev.read("/api/projects/site/tree").await["entries"], json!([]));
    let told = dev.node().arrived.clone().unwrap().said().join("\n");
    assert!(told.contains("The project `site` came without its files"), "{told}");
    assert!(
        told.contains(&format!("they are in {}, which is the other factory's", site.display())),
        "{told}"
    );
    assert!(told.contains(&format!("It has an empty directory here, {}", here.display())), "{told}");
    assert!(told.contains("Its table `board` is kept as files there"), "{told}");
    let copied = dev.read("/api/events/recent?topics=system.copied").await;
    assert_eq!(copied["events"][0]["data"]["without_files"][0]["was"], site.to_str().unwrap());

    // What is kept as files there is declared, and has nothing in it;
    // what is put there from now on is this factory's.
    assert_eq!(dev.read("/api/board/site").await["items"], json!([]));
    assert_eq!(dev.send("/api/projects/site/scan", json!({})).await["changes"], json!([]));
    dev.send("/api/board/site/items", json!({ "title": "Tried in dev" })).await;
    assert!(here.join("board/1.md").is_file());
    let before: Vec<String> = std::fs::read_dir(site.join("board"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(before, ["1.md"], "and nothing was written where the other keeps its");
    assert!(std::fs::read_to_string(site.join("board/1.md")).unwrap().contains("Redo the front page"));
    for instance in prod.read("/api/instances?live=true&roots=true").await["instances"].as_array().unwrap() {
        stop_instance(&prod, instance["id"].as_str().unwrap()).await;
    }
}

#[tokio::test]
async fn an_arrival_that_was_cut_short_is_finished_by_starting_from_the_same_snapshot_again() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Web" })).await;
    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    let other = taken(&prod, &shared.snapshot("another")).await;

    // What a start that was killed part-way leaves: the folder's lock and
    // name, a note of which snapshot was arriving, some of its files, and
    // a database that is not whole and not yet in its place.
    let (mut config, dir) = shared.config(Some("dev2"), &to);
    config.default_adapter = Some("builtin".into());
    let data = dir.join("data");
    std::fs::create_dir_all(data.join("projects/web")).unwrap();
    std::fs::write(data.join("lock"), "").unwrap();
    std::fs::write(data.join("projects/web/README.md"), "half of it").unwrap();
    std::fs::write(data.join("journal.db.arriving"), "not a database").unwrap();
    let marker = data.join(api::arrival::MARKER);

    // Part of another snapshot is not finished with this one,
    std::fs::write(&marker, json!({ "snapshot": other["id"] }).to_string()).unwrap();
    let said = refused(config.clone()).await;
    assert!(said.contains("holds part of another snapshot"), "{said}");
    assert!(said.contains(other["id"].as_str().unwrap()), "{said}");
    assert!(!data.join("journal.db").exists());

    // and part of this one is: the same start, given again.
    std::fs::write(&marker, json!({ "snapshot": manifest["id"] }).to_string()).unwrap();
    let node = Node::start(config.clone()).await.unwrap();
    assert!(
        data.join("journal.db").is_file() && !marker.exists() && !data.join("journal.db.arriving").exists()
    );
    assert_ne!(std::fs::read_to_string(data.join("projects/web/README.md")).unwrap(), "half of it");
    assert!(node.arrived.is_some());
    node.stop().await;

    // Once it has arrived it is a factory, and nothing is copied into it
    // again; started as any is, it says what it was started from no more.
    let said = refused(config.clone()).await;
    assert!(said.contains("already holds a factory"), "{said}");
    let node = Node::start(Config { from: None, ..config }).await.unwrap();
    assert!(node.arrived.is_none());
    let seen = node.app.journal.read(journal_query("system.copied")).await.unwrap();
    assert_eq!(seen.len(), 1, "said once, in its own journal");
    node.stop().await;
    let _ = std::fs::remove_dir_all(dir);
}

fn journal_query(topic: &str) -> journal::EventQuery {
    journal::EventQuery { topics: topic.to_string(), ..journal::EventQuery::default() }
}

#[tokio::test]
async fn a_message_an_agent_answered_is_not_answered_again_in_a_copy_and_a_new_one_is() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let channel = prod.send("/api/channels", json!({ "name": "general" })).await;
    let channel = channel["id"].as_str().unwrap().to_string();
    prod.send(&format!("/api/channels/{channel}/agents"), json!({ "name": "helper", "adapter": "worker" }))
        .await;
    prod.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@helper tidy the docs" })).await;
    prod.event("channel.message", |event| event.data["author"] == "agent:helper").await;
    // An answer is said before its turn is said to have ended, so the
    // turn is waited for too before anything is read of it or copied.
    prod.event("agent.turn.finished", |event| event.data["slot"] == "helper").await;
    let token = prod.dir.join("data/slots/helper/token");
    let credential = std::fs::read_to_string(&token).unwrap();
    // It has written something of its own where it works with no project.
    std::fs::write(prod.dir.join("data/slots/helper/notes.txt"), "the agent's own").unwrap();

    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    assert_eq!(manifest["left_out"]["slots"], json!(["helper"]), "what it wrote there is said to be left");
    assert!(
        files_under(&to)
            .iter()
            .all(|(_, bytes)| !holds(bytes, credential.trim()) && !holds(bytes, "the agent's own"))
    );

    // The agent is in the copy, in the channel, with what it did; and
    // nothing of the folder its credential was in.
    let dev = shared.copy("dev2", &to).await;
    let slots = dev.dir.join("data/slots");
    assert!(!slots.join("helper/token").exists() && !slots.join("helper/notes.txt").exists());
    let told = dev.node().arrived.clone().unwrap().said().join("\n");
    assert!(
        told.contains("What `helper` wrote in its own folder, working with no project, was left where it is"),
        "{told}"
    );
    assert_eq!(dev.read("/api/agents/helper/turns").await["turns"].as_array().unwrap().len(), 1);

    // Asked something new here, it answers that, once, signed in to this
    // factory with a credential made for it now. What was asked before the
    // copy was made is not asked of it again.
    let before = dev.mark();
    let asked = dev
        .send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@helper and the tests" }))
        .await;
    // (The journal came with the copy: the answer waited for is the one to this.)
    dev.event("channel.message", |event| {
        event.data["author"] == "agent:helper" && event.data["reply_to"] == asked["id"]
    })
    .await;
    // Its answer is said before its turn is said to have ended.
    dev.event("agent.turn.finished", move |event| event.seq > before && event.data["slot"] == "helper").await;
    let turns = dev.read("/api/agents/helper/turns").await;
    let turns = turns["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2, "{turns:?}");
    assert!(turns.iter().all(|turn| turn["status"] == "done"), "{turns:?}");
    let said = bodies(&dev.read(&format!("/api/channels/{channel}/messages")).await);
    assert_eq!(
        said,
        ["@helper tidy the docs", "I did the work.", "@helper and the tests", "I did the work."]
    );
    let fresh = std::fs::read_to_string(slots.join("helper/token")).unwrap();
    assert_ne!(fresh.trim(), credential.trim());
    let (status, me) = dev.call(Method::GET, "/api/me", Some(fresh.trim()), None).await;
    assert_eq!(status, StatusCode::OK, "{me}");
    let (status, _) = dev.call(Method::GET, "/api/me", Some(credential.trim()), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "the other factory's is nothing here");
    let (status, _) = prod.call(Method::GET, "/api/me", Some(credential.trim()), None).await;
    assert_eq!(status, StatusCode::OK, "and is still good where it was issued");
    assert_eq!(prod.read("/api/agents/helper/turns").await["turns"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn work_an_agent_had_in_hand_stays_with_the_factory_it_was_given_in() {
    let shared = Shared::new();
    // An agent whose program does not finish until it is let: it waits
    // at a hold that nothing lets it go from.
    let gate = shared.0.join("gate");
    crate::hold(&gate);
    let waits = crate::standin::argv(&[
        "swallow",
        "held",
        &gate.to_string_lossy(),
        "slow",
        "say",
        "finished in prod",
    ]);
    let registry = shared.registry();
    let prod = Rig::owned(
        Rig::with("api-snap-prod", move |config, _| {
            config.profile = Some("prod".into());
            config.registry = Some(registry);
            config.adapters.push(("slow".into(), waits));
        })
        .await,
    )
    .await;
    let channel = prod.send("/api/channels", json!({ "name": "general" })).await;
    let channel = channel["id"].as_str().unwrap().to_string();
    prod.send(&format!("/api/channels/{channel}/agents"), json!({ "name": "sloth", "adapter": "slow" }))
        .await;
    // One piece of work under way, and one waiting behind it.
    prod.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@sloth the first job" })).await;
    prod.event("agent.turn.started", |_| true).await;
    prod.send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@sloth the second job" })).await;
    prod.count("agent.turn.queued", 2).await;

    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    assert_eq!(manifest["left_out"]["turns"], 2);

    // In the copy the same agent runs on something that answers at once.
    // Neither piece of work is taken up there: each is ended, in so many
    // words, and what is asked of the agent there is the first thing it does.
    let (registry, from) = (shared.registry(), to.clone());
    let mut dev = Rig::with("api-snap-dev2", move |config, _| {
        config.profile = Some("dev2".into());
        config.registry = Some(registry);
        config.from = Some(from);
        config.adapters.push(("slow".into(), crate::standin::argv(&["swallow", "say", "answered in dev"])));
    })
    .await;
    dev.token = sign_in(&dev).await;
    let before = dev.mark();
    let asked = dev
        .send(&format!("/api/channels/{channel}/messages"), json!({ "body": "@sloth something new" }))
        .await;
    dev.event("channel.message", |event| event.data["reply_to"] == asked["id"]).await;
    // Its answer is said before its turn is said to have ended.
    dev.event("agent.turn.finished", move |event| {
        event.seq > before && event.data["slot"] == "sloth" && event.data["ok"] == true
    })
    .await;
    let said = bodies(&dev.read(&format!("/api/channels/{channel}/messages")).await);
    assert_eq!(
        said,
        ["@sloth the first job", "@sloth the second job", "@sloth something new", "answered in dev"]
    );
    let turns = dev.read("/api/agents/sloth/turns").await;
    let turns: Vec<(&str, &str)> = turns["turns"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .map(|turn| (turn["status"].as_str().unwrap(), turn["error"].as_str().unwrap_or_default()))
        .collect();
    assert_eq!(turns.len(), 3, "{turns:?}");
    assert_eq!(turns[2], ("done", ""));
    for (status, error) in &turns[..2] {
        assert_eq!(*status, "failed");
        assert!(error.contains("a copy takes up none of that factory's work"), "{error}");
    }

    // The factory they were given in does them, when it is let: one at
    // a time, each let go once it is waiting.
    for done in 1..=2 {
        crate::let_go("the slow agent's program to be waiting", &gate).await;
        prod.count("agent.turn.finished", done).await;
    }
    let said = bodies(&prod.read(&format!("/api/channels/{channel}/messages")).await);
    assert_eq!(said.iter().filter(|body| body.contains("finished in prod")).count(), 2, "{said:?}");
    assert!(!said.iter().any(|body| body.contains("in dev")), "{said:?}");
}

#[tokio::test]
async fn a_snapshot_that_was_not_made_as_a_factory_makes_one_still_arrives_signed_out_and_with_nothing_running()
 {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    prod.send("/api/projects", json!({ "name": "Web" })).await;
    prod.send("/api/agents", json!({ "name": "builder", "project": "web" })).await;
    let kept = prod.send("/api/apply", json!({ "source": crate::snapshots::keeper() })).await;
    let keeper = kept["instance"].as_str().unwrap().to_string();
    prod.event("step.waiting", |event| event.source.instance.as_deref() == Some(&keeper)).await;

    // A folder that says it is a snapshot, around the factory's database
    // as it stands: everyone signed in, everything running, and every
    // project where the factory has it.
    let to = shared.snapshot("by-hand");
    taken(&prod, &to).await;
    std::fs::remove_file(to.join("database.db")).unwrap();
    let whole = rusqlite::Connection::open(prod.dir.join("data/journal.db")).unwrap();
    whole.execute("VACUUM INTO ?1", [to.join("database.db").to_str().unwrap()]).unwrap();
    drop(whole);
    assert!(files_under(&to).iter().any(|(_, bytes)| holds(
        bytes,
        prod.dir.join("data").join("projects").join("web").to_str().unwrap()
    )));

    // What is taken out when a snapshot is made is taken out again on
    // arrival, and whoever started the factory is told that it had to be.
    let dev = shared.copy("dev2", &to).await;
    let told = dev.node().arrived.clone().unwrap().said().join("\n");
    assert!(told.contains("The snapshot was not as a factory makes one"), "{told}");
    let (status, _) = dev.call(Method::GET, "/api/me", Some(&prod.token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(dev.read("/api/credentials").await["credentials"], json!([]));
    let fleet = dev.read("/api/fleet").await;
    assert_eq!((&fleet["instances"], &fleet["services"]), (&json!([]), &json!([])));
    assert_eq!(dev.read(&format!("/api/instances/{keeper}")).await["instance"]["status"], "stopped");
    // No project is left where the snapshot says it was: that is the
    // other factory's directory. It has one of its own, with nothing in it.
    let here = dev.dir.join("data").join("projects").join("web");
    assert_eq!(dev.read("/api/projects/web").await["path"], here.to_str().unwrap());
    assert!(std::fs::read_dir(&here).unwrap().next().is_none());
    assert!(told.contains("The project `web` came without its files"), "{told}");

    // The other factory still runs what it ran, in the process it ran it in.
    let service = &prod.read("/api/fleet").await["services"][0];
    assert_eq!((&service["name"], &service["status"]), (&json!("keeper"), &json!("healthy")));
    assert!(prod.dir.join("data/projects/web/README.md").is_file());
    stop_instance(&prod, &keeper).await;
}

#[tokio::test]
async fn a_copy_has_none_of_the_variables_of_the_factory_it_was_made_from_until_they_are_set_in_it() {
    use crate::variables::{read, set};
    const KEPT: &str = "SELECT count(*) FROM kv WHERE namespace = 'environment'";
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let vars = json!({ "api": "https://api.prod.example", "replicas": 3 });
    assert_eq!(set(&prod, vars.clone()).await["status"], "ok");
    assert_eq!(counted(&prod, KEPT).await, 1);

    // They are said of the factory copied (where its services answer),
    // and a copy that read them would take that factory's addresses for
    // its own. The snapshot says that it has none, and has none.
    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    let never = manifest["left_out"]["never"].as_array().unwrap();
    assert!(never.iter().any(|never| never.as_str().unwrap().contains("the variables of the profile")));
    let kept = |snapshot: &Path| {
        let database = rusqlite::Connection::open(snapshot.join("database.db")).unwrap();
        database.query_row(KEPT, [], |row| row.get::<_, i64>(0)).unwrap()
    };
    assert_eq!(kept(&to), 0);
    let dev = shared.copy("dev2", &to).await;
    assert_eq!(read(&dev).await, json!({ "api": "http://localhost", "replicas": null, "all": {} }));
    assert_eq!(counted(&dev, KEPT).await, 0);
    assert_eq!(
        dev.read("/api/instances?live=true").await["instances"],
        json!([]),
        "and nothing is there to set them"
    );

    // Nor from a snapshot made some other way, with the factory's
    // database as it stands: they are taken out again on arrival.
    let by_hand = shared.snapshot("by-hand");
    taken(&prod, &by_hand).await;
    std::fs::remove_file(by_hand.join("database.db")).unwrap();
    let whole = rusqlite::Connection::open(prod.dir.join("data/journal.db")).unwrap();
    whole.execute("VACUUM INTO ?1", [by_hand.join("database.db").to_str().unwrap()]).unwrap();
    drop(whole);
    assert_eq!(kept(&by_hand), 1);
    let other = shared.copy("dev3", &by_hand).await;
    assert_eq!(read(&other).await["all"], json!({}));
    assert_eq!(counted(&other, KEPT).await, 0);

    // The new factory has its own once its owner says them there, and
    // the factory copied has what it had.
    assert_eq!(set(&dev, json!({ "api": "http://127.0.0.1:8080" })).await["status"], "ok");
    assert_eq!(read(&dev).await["all"], json!({ "api": "http://127.0.0.1:8080" }));
    assert_eq!(read(&prod).await["all"], vars);
}

/// Every page of a factory started from a copy says so across its top, with
/// its profile, what it is a copy of and when it was taken; the factory
/// copied says nothing there. It is said until whoever manages
/// the copy says it may stop, which every page hears, and said again where
/// it is told something new to say.
#[tokio::test]
async fn a_copy_says_on_every_page_what_it_is_a_copy_of_until_its_owner_says_it_may_stop() {
    let shared = Shared::new();
    let prod = shared.factory("prod").await;
    let to = shared.snapshot("prod-copy");
    let manifest = taken(&prod, &to).await;
    let dev = shared.copy("dev2", &to).await;
    assert_eq!(prod.read("/api/meta").await["across"], Value::Null, "the factory copied says nothing there");
    let said = dev.read("/api/meta").await["across"].clone();
    assert_eq!(
        (said["kind"].as_str(), said["of"].as_str(), said["profile"].as_str()),
        (Some("copy"), Some("prod"), Some("dev2"))
    );
    assert_eq!(said["made"], manifest["made"], "{said}");
    let words = said["words"].as_str().unwrap();
    assert!(
        words.starts_with("Profile: dev2. A copy of prod taken ")
            && words.ends_with(": what is done here stays here."),
        "{words}"
    );

    // Somebody who does not manage it may not; whoever does may, and every page is told.
    let narrow = crate::held::key(&dev, &["board", "channels"]).await;
    let (status, _) =
        dev.call(Method::POST, "/api/system/across", Some(&narrow), Some(json!({ "quiet": true }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let from = dev.mark();
    let quiet = dev.send("/api/system/across", json!({ "quiet": true })).await;
    assert_eq!((quiet["across"].clone(), quiet["quiet"].clone()), (Value::Null, json!(true)));
    let heard = dev.event("system.across", |event| event.seq > from).await;
    assert_eq!(heard.data["across"], Value::Null);
    assert_eq!(dev.read("/api/meta").await["across"], Value::Null);

    // Told what to say, it says that, copy or not, though it was told it may stop.
    let told = dev.send("/api/system/across", json!({ "says": "Trying the release here." })).await;
    assert_eq!(
        (told["across"]["kind"].as_str(), told["across"]["words"].as_str()),
        (Some("said"), Some("Trying the release here."))
    );
    let (status, _) = dev
        .call(Method::POST, "/api/system/across", Some(&dev.token), Some(json!({ "says": "one\ntwo" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "what is said is one line");
    let usual = dev.send("/api/system/across", json!({ "says": null })).await;
    assert!(
        usual["across"]["words"].as_str().unwrap().starts_with("Profile: dev2. A copy of prod"),
        "{usual}"
    );
}
