//! What a credential may do is what its bearer can do, and being able to
//! reach the daemon is nothing: who a request is addressed to, where one
//! that changes something comes from, who makes the first account, what a
//! key can make, what folder a project may be made over, whose blueprints
//! run under a name that ships, and who is told what happened.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::Path;

use futures_util::StreamExt;
use reqwest::header::{HOST, ORIGIN};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::{Rig, soon};

/// A key of the rig's owner that holds `scopes` and nothing else.
async fn key(rig: &Rig, scopes: &[&str]) -> String {
    let made = rig.send("/api/credentials", json!({ "name": "narrow", "scopes": scopes })).await;
    made["token"].as_str().unwrap().to_string()
}

fn said(body: &Value) -> String {
    body["error"]["message"].as_str().unwrap_or_default().to_string()
}

#[tokio::test]
async fn a_file_of_the_client_is_only_ever_one_inside_its_folder() {
    let rig = Rig::with("guard-client", |config, dir| {
        let client = dir.join("client");
        std::fs::create_dir_all(client.join("assets")).unwrap();
        std::fs::write(client.join("index.html"), "<p>the client</p>").unwrap();
        std::fs::write(client.join("assets/app.js"), "// the client's own").unwrap();
        config.ui_dir = Some(client);
    })
    .await;
    let client = rig.dir.join("client");
    std::fs::write(rig.dir.join("secret.txt"), "not the client's").unwrap();

    // A path is taken apart by its parts and each must be an ordinary name:
    // nothing that goes up, starts again from the top, names a drive or
    // uses the separator of another system.
    assert_eq!(api::client_file(&client, "assets/app.js"), Some(client.join("assets/app.js")));
    for path in [
        "../secret.txt",
        "assets/../../secret.txt",
        "assets/./app.js",
        "/etc/passwd",
        "//etc/passwd",
        "C:/Windows/win.ini",
        "C:..\\secret.txt",
        "c:secret.txt",
        "assets\\app.js",
        "..\\secret.txt",
        "assets/",
        "",
    ] {
        assert_eq!(api::client_file(&client, path), None, "`{path}` is not a file of the client");
    }

    // And where it resolves is still inside: a link in the folder that
    // leads out of it is not the client's file, to anyone, with or without
    // a credential.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(rig.dir.join("secret.txt"), client.join("assets/out.txt")).unwrap();
        assert_eq!(api::client_file(&client, "assets/out.txt"), None);
        let through = reqwest::get(format!("{}/assets/out.txt", rig.url)).await.unwrap();
        assert_eq!(through.status(), StatusCode::NOT_FOUND);
    }
    let own = reqwest::get(format!("{}/assets/app.js", rig.url)).await.unwrap();
    assert_eq!((own.status(), own.text().await.unwrap().as_str()), (StatusCode::OK, "// the client's own"));
    for path in ["/..%2f..%2fsecret.txt", "/assets/..%5c..%5csecret.txt", "/C:/secret.txt"] {
        let asked = reqwest::get(format!("{}{path}", rig.url)).await.unwrap();
        assert!(!asked.text().await.unwrap().contains("not the client's"), "{path}");
    }
}

/// What the daemon answers when it is asked for `path` under `host`.
async fn asked(rig: &Rig, host: &str, path: &str) -> (StatusCode, Value) {
    let response =
        reqwest::Client::new().get(format!("{}{path}", rig.url)).header(HOST, host).send().await.unwrap();
    (response.status(), response.json::<Value>().await.unwrap_or_default())
}

#[tokio::test]
async fn a_request_asked_under_a_name_that_is_not_the_daemons_is_refused_before_any_route() {
    let mut rig = Rig::new().await;
    let port = rig.node().addr.port();

    // A name that merely leads here: the page, what a page needs before
    // anyone signs in, and everything else alike.
    for path in ["/", "/api/meta", "/api/me", "/library/shop/docs/"] {
        let (status, body) = asked(&rig, "factory.example.net", path).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
        assert!(said(&body).contains("was asked as `factory.example.net`"), "{body}");
        assert!(said(&body).contains("names add factory.example.net"), "{body}");
    }
    let register = reqwest::Client::new()
        .post(format!("{}/api/auth/register", rig.url))
        .header(HOST, format!("evil.example:{port}"))
        .json(&json!({ "email": "mallory@example.com", "name": "Mallory", "password": "correct horse" }))
        .send()
        .await
        .unwrap();
    assert_eq!(register.status(), StatusCode::FORBIDDEN);

    // Its own addresses, and this machine by its usual name.
    for host in [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("LOCALHOST:{port}"),
        format!("[::1]:{port}"),
        format!("192.168.1.5:{port}"),
        "factory.localhost".to_string(),
    ] {
        assert_eq!(asked(&rig, &host, "/api/meta").await.0, StatusCode::OK, "{host}");
    }

    // A name is the owner's to say, and is then one of its own, whatever
    // the port and however it is written.
    let narrow = key(&rig, &["fleet", "blueprints"]).await;
    let telling = json!({ "name": "factory.example.net" });
    let (status, body) =
        rig.call(Method::POST, "/api/system/names", Some(&narrow), Some(telling.clone())).await;
    assert_eq!((status, body["error"]["kind"].as_str()), (StatusCode::FORBIDDEN, Some("denied")), "{body}");
    for wrong in ["", "10.0.0.1", "a name", "factory.example.net/path", "http://factory.example.net"] {
        let (status, body) = rig.post("/api/system/names", json!({ "name": wrong })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "`{wrong}`: {body}");
    }
    let told = rig.send("/api/system/names", json!({ "name": "Factory.Example.Net:8443" })).await;
    assert_eq!(told["name"], "factory.example.net");
    rig.event("system.named", |event| event.data["name"] == "factory.example.net").await;
    for host in ["factory.example.net", "factory.example.net:8443", "FACTORY.example.net."] {
        assert_eq!(asked(&rig, host, "/api/meta").await.0, StatusCode::OK, "{host}");
    }
    assert_eq!(asked(&rig, "other.example.net", "/api/meta").await.0, StatusCode::FORBIDDEN);
    assert_eq!(rig.read("/api/system/names").await["names"], json!(["factory.example.net"]));

    // It is kept, and taken back.
    rig.restart().await;
    assert_eq!(asked(&rig, "factory.example.net", "/api/meta").await.0, StatusCode::OK);
    let (status, _) =
        rig.call(Method::DELETE, "/api/system/names/factory.example.net", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
    rig.event("system.unnamed", |event| event.data["name"] == "factory.example.net").await;
    assert_eq!(asked(&rig, "factory.example.net", "/api/meta").await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_blueprint_tells_the_daemon_a_name_only_for_its_owner() {
    let rig = Rig::new().await;
    let source = "blueprint: fronted\npermissions: [admin]\nsteps:\n  known: { do: system.answer_as, with: { name: front.example.net } }\n";
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let body = json!({ "source": source, "wait": true });
    let (_, applied) = rig.call(Method::POST, "/api/apply", Some(&member), Some(body.clone())).await;
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    assert!(applied["finished"]["error"].as_str().unwrap().contains("may not use `admin`"), "{applied}");
    assert_eq!(rig.read("/api/system/names").await["names"], json!([]));

    let applied = rig.send("/api/apply", json!({ "source": source, "wait": true, "key": "owners" })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(rig.read("/api/system/names").await["names"], json!(["front.example.net"]));
}

/// An address of this machine that is not the loopback one, if it has one:
/// the one it would send from to reach anywhere else. Nothing is sent.
fn own_address() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

#[tokio::test]
async fn the_first_account_is_made_on_the_machine_the_daemon_runs_on() {
    let rig = Rig::with("guard-first", |config, _| config.listen = SocketAddr::from(([0, 0, 0, 0], 0))).await;
    let port = rig.node().addr.port();
    let ada = json!({ "email": "ada@example.com", "name": "Ada", "password": "correct horse" });
    let register = async |address: String| {
        let response = reqwest::Client::new()
            .post(format!("http://{address}:{port}/api/auth/register"))
            .json(&ada)
            .send()
            .await
            .unwrap();
        (response.status(), response.json::<Value>().await.unwrap())
    };
    // Reached from an address that is not this machine's own way to itself,
    // there is no owner to be had, however the request is made.
    match own_address() {
        Some(elsewhere) => {
            let (status, body) = register(elsewhere.to_string()).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
            assert!(said(&body).contains("its first account is made on the machine it runs on"), "{body}");
            let (_, meta) = rig.call(Method::GET, "/api/meta", None, None).await;
            assert_eq!(meta["needs_setup"], true, "nobody became the owner");
        }
        None => eprintln!(
            "this machine has no address but its own loopback; the refusal is held in studio's suite"
        ),
    }
    let (status, body) = register("127.0.0.1".to_string()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["account"]["role"], "owner");
}

#[tokio::test]
async fn of_those_who_register_first_at_once_one_is_the_owner_and_the_others_are_told_there_is_one() {
    let rig = Rig::fresh().await;
    let attempts = (0..6).map(|n| {
        let url = format!("{}/api/auth/register", rig.url);
        async move {
            let body =
                json!({ "email": format!("user{n}@example.com"), "name": "Someone", "password": "correct horse" });
            let response = reqwest::Client::new().post(url).json(&body).send().await.unwrap();
            (response.status(), response.json::<Value>().await.unwrap())
        }
    });
    let outcomes = futures_util::future::join_all(attempts).await;
    let owners: Vec<&Value> =
        outcomes.iter().filter(|(status, _)| *status == StatusCode::OK).map(|(_, body)| body).collect();
    assert_eq!(owners.len(), 1, "{outcomes:?}");
    assert_eq!(owners[0]["account"]["role"], "owner");
    for (status, body) in outcomes.iter().filter(|(status, _)| *status != StatusCode::OK) {
        assert_eq!(*status, StatusCode::FORBIDDEN, "{body}");
        assert!(said(body).contains("already has an owner"), "{body}");
    }
    let accounts = rig.call(Method::GET, "/api/accounts", owners[0]["token"].as_str(), None).await.1;
    assert_eq!(accounts["accounts"].as_array().unwrap().len(), 1, "{accounts}");
}

#[tokio::test]
async fn a_key_makes_a_key_that_holds_no_more_than_it_does() {
    let rig = Rig::new().await;
    let narrow = key(&rig, &["search", "board:read"]).await;
    let make = async |token: &str, body: Value| {
        rig.call(Method::POST, "/api/credentials", Some(token), Some(body)).await
    };
    let scopes_of = async |token: &str| {
        let (_, me) = rig.call(Method::GET, "/api/me", Some(token), None).await;
        me["principal"]["scopes"].clone()
    };

    // Told nothing, the new key holds what its maker does: not what a
    // person holds, which is how a key that could only search became one
    // that could run commands.
    let (status, made) = make(&narrow, json!({ "name": "its-own" })).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!(made["scopes"], json!(["search", "board:read"]));
    assert_eq!(scopes_of(made["token"].as_str().unwrap()).await, json!(["search", "board:read"]));

    // Asked for more, it is refused, by the name of what it lacks.
    for beyond in ["shell", "board:write", "admin", "board"] {
        let (status, body) = make(&narrow, json!({ "name": "more", "scopes": ["search", beyond] })).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{beyond}: {body}");
        assert!(said(&body).contains(&format!("`{beyond}`")), "{body}");
    }
    // Less is always its to give.
    let (status, less) = make(&narrow, json!({ "name": "less", "scopes": ["search"] })).await;
    assert_eq!((status, &less["scopes"]), (StatusCode::OK, &json!(["search"])), "{less}");

    // The owner's own, told nothing, is the owner's.
    let whole = rig.send("/api/credentials", json!({ "name": "whole" })).await;
    assert!(whole["scopes"].as_array().unwrap().contains(&json!("admin")), "{whole}");
}

#[tokio::test]
async fn adopting_a_folder_is_the_owners_and_never_one_that_holds_the_factorys_own_data() {
    let rig = Rig::new().await;
    let data = rig.dir.join("data");
    let elsewhere = rig.dir.join("somebodys-repository");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::write(elsewhere.join("README.md"), "# Theirs\n").unwrap();
    let adopt = async |token: &str, name: &str, path: &Path| {
        let body = json!({ "name": name, "path": path });
        rig.call(Method::POST, "/api/projects", Some(token), Some(body)).await
    };

    // Whoever may make projects may make them where the factory keeps
    // them; a folder that is already somewhere takes the owner.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let (status, body) = adopt(&member, "Theirs", &elsewhere).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(said(&body).contains("needs `admin`"), "{body}");
    let (status, made) =
        rig.call(Method::POST, "/api/projects", Some(&member), Some(json!({ "name": "Made Here" }))).await;
    assert_eq!(status, StatusCode::OK, "{made}");
    assert_eq!((&made["brownfield"], &made["trusted"]), (&json!(false), &json!(true)));

    // The owner, and still not over the factory's data, around it, or in it.
    for (name, path) in [
        ("Data", data.clone()),
        ("Around", rig.dir.clone()),
        // The top of everything, as this system writes it.
        ("Root", rig.dir.ancestors().last().unwrap().to_path_buf()),
        ("Slots", data.join("slots")),
        ("Inside", data.join("not-there-yet")),
    ] {
        let (status, body) = adopt(&rig.token, name, &path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {body}");
        assert!(said(&body).contains("this factory's own data"), "{body}");
    }
    #[cfg(unix)]
    {
        // By another way round to the same place, too.
        let link = rig.dir.join("a-link-to-the-data");
        std::os::unix::fs::symlink(&data, &link).unwrap();
        let (status, body) = adopt(&rig.token, "Linked", &link).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    assert!(!data.join("not-there-yet").exists());
    let (status, adopted) = adopt(&rig.token, "Theirs", &elsewhere).await;
    assert_eq!(status, StatusCode::OK, "{adopted}");
    assert_eq!((&adopted["brownfield"], &adopted["trusted"]), (&json!(true), &json!(false)));
    // Where the factory makes them itself is still a place for one.
    let (status, own) = adopt(&rig.token, "Beside", &data.join("projects/beside")).await;
    assert_eq!(status, StatusCode::OK, "{own}");
}

/// A board blueprint of a repository's own making: it says it is
/// `std/board`, and what it does is not what the one that ships does.
const THEIR_BOARD: &str = "\
blueprint: std/board
description: Somebody else's idea of a board.
steps:
  theirs: { do: emit, with: { topic: theirs.ran } }
";

#[tokio::test]
async fn a_project_made_of_a_folder_that_was_there_does_not_replace_the_blueprints_that_ship_until_it_is_trusted()
 {
    let rig = Rig::new().await;
    let repository = rig.dir.join("their-repository");
    std::fs::create_dir_all(repository.join("blueprints/std")).unwrap();
    std::fs::write(repository.join("blueprints/std/board.bp"), THEIR_BOARD).unwrap();
    std::fs::write(repository.join("theirs.bp"), "blueprint: theirs\nsteps:\n  one: { do: noop }\n").unwrap();
    let adopted = rig.send("/api/projects", json!({ "name": "Shop", "path": repository })).await;
    assert_eq!(adopted["trusted"], false, "{adopted}");
    // Its repository's are read as its own only where it says so: a folder of them, and its top.
    rig.send("/api/projects/shop/blueprints", json!({ "folder": "blueprints" })).await;
    let source =
        async || rig.read("/api/blueprints/source?name=std/board&project=shop").await["source"].clone();
    let described = async || {
        let all = rig.read("/api/blueprints?project=shop").await;
        all["blueprints"].as_array().unwrap().iter().find(|one| one["name"] == "std/board").cloned().unwrap()
    };

    // What `std/board` is there is what ships, and its own file is not
    // listed as if it were.
    let shipped = rig.read("/api/blueprints/source?name=std/board").await["source"].clone();
    assert_eq!(source().await, shipped);
    assert_eq!(described().await["file"], Value::Null);
    let used = rig.read("/api/blueprints/used").await;
    let own: Vec<&str> = used["projects"]["shop"]["own"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|one| one["name"].as_str())
        .collect();
    assert_eq!(own, ["theirs"], "its blueprints under names of its own are its own: {used}");

    // Whoever checks is told, with the file and how to have it used.
    let checked = rig.send("/api/blueprints/check", json!({ "source": shipped, "project": "shop" })).await;
    let passed_over: Vec<&str> = checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["code"] == "own-not-used")
        .filter_map(|one| one["message"].as_str())
        .collect();
    assert_eq!(passed_over.len(), 1, "{checked}");
    assert!(passed_over[0].contains("`blueprints/std/board.bp`"), "{passed_over:?}");
    assert!(passed_over[0].contains("project trust shop"), "{passed_over:?}");
    assert_eq!(checked["ok"], true, "it is something to know, not a mistake: {checked}");

    // Whoever applies gets the board that ships, and is told the same.
    let applied =
        rig.send("/api/apply", json!({ "blueprint": "std/board", "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert!(applied["said"][0].as_str().unwrap().contains("`blueprints/std/board.bp`"), "{applied}");
    assert_eq!(rig.count_of("theirs.ran").await, 0, "their steps did not run");
    assert_eq!(rig.read("/api/board/shop").await["columns"].as_array().map(Vec::len), Some(5));
    // One of its own, under a name of its own, is applied as it always was.
    let theirs =
        rig.send("/api/apply", json!({ "blueprint": "theirs", "project": "shop", "wait": true })).await;
    assert_eq!((&theirs["finished"]["status"], &theirs["said"]), (&json!("ok"), &Value::Null), "{theirs}");

    // Trusting it is the owner's, and is said.
    let member = key(&rig, kernel::principal::MEMBER_SCOPES).await;
    let trust = json!({ "trusted": true });
    let (status, body) =
        rig.call(Method::POST, "/api/projects/shop/trust", Some(&member), Some(trust.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(rig.send("/api/projects/shop/trust", trust).await["trusted"], true);
    rig.event("project.trusted", |event| event.data["project"] == "shop").await;
    assert_eq!(source().await, THEIR_BOARD);
    assert_eq!(described().await["file"], "blueprints/std/board.bp");
    let checked =
        rig.send("/api/blueprints/check", json!({ "source": THEIR_BOARD, "project": "shop" })).await;
    assert!(!checked.to_string().contains("own-not-used"), "{checked}");

    // And taken back.
    assert_eq!(rig.send("/api/projects/shop/trust", json!({ "trusted": false })).await["trusted"], false);
    rig.event("project.untrusted", |event| event.data["project"] == "shop").await;
    assert_eq!(source().await, shipped);

    // What the factory keeps for a project is its owner's own, whatever is put there.
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    let depot = rig.placed("depot").join("std");
    std::fs::create_dir_all(&depot).unwrap();
    std::fs::write(depot.join("board.bp"), THEIR_BOARD).unwrap();
    let theirs = rig.read("/api/blueprints/source?name=std/board&project=depot").await;
    assert_eq!(theirs["source"], THEIR_BOARD);
}

/// The topics of what the caller with `token` is given as having happened.
async fn recent(rig: &Rig, token: &str) -> BTreeSet<String> {
    let (status, body) = rig.call(Method::GET, "/api/events/recent?limit=1000", Some(token), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|event| event["topic"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn a_caller_is_told_only_of_what_its_scopes_let_it_read() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    let talk = key(&rig, &["channels:read"]).await;
    let cards = key(&rig, &["board"]).await;
    let nothing = key(&rig, &["secrets:use"]).await;

    // Listening, with a key that reads channels and nothing else.
    let stream = reqwest::Client::new()
        .get(format!("{}/api/events", rig.url))
        .bearer_auth(&talk)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    let mut stream = stream.bytes_stream();

    rig.send("/api/board/shop/items", json!({ "title": "Sell mugs" })).await;
    rig.send(
        "/api/apply",
        json!({ "source": "blueprint: quiet\nsteps:\n  one: { do: noop }\n", "wait": true }),
    )
    .await;
    rig.send("/api/channels/general/messages", json!({ "body": "the mugs are in" })).await;

    // Everything that happened is in order, so by the time what was said
    // arrives, the card and the run before it would have, had they been
    // sent.
    let heard = soon("what was said in the channel", async {
        let mut heard = String::new();
        while !heard.contains("the mugs are in") {
            heard.push_str(&String::from_utf8_lossy(&stream.next().await.unwrap().unwrap()));
        }
        heard
    })
    .await;
    assert!(heard.contains("\"topic\":\"channel.message\""), "{heard}");
    for kept_back in ["board.item.created", "run.started", "step.finished", "instance.started", "Sell mugs"] {
        assert!(
            !heard.contains(kept_back),
            "`{kept_back}` was told to a key that reads channels only: {heard}"
        );
    }

    // And asked what happened lately, each is given its own.
    let begins = |topics: &BTreeSet<String>, with: &str| topics.iter().any(|topic| topic.starts_with(with));
    let (to_talk, to_cards, to_nothing) =
        (recent(&rig, &talk).await, recent(&rig, &cards).await, recent(&rig, &nothing).await);
    assert!(to_talk.contains("channel.message") && to_talk.contains("channel.created"), "{to_talk:?}");
    assert!(
        !begins(&to_talk, "board.") && !begins(&to_talk, "run.") && !begins(&to_talk, "identity."),
        "{to_talk:?}"
    );
    assert!(to_cards.contains("board.item.created"), "{to_cards:?}");
    assert!(!begins(&to_cards, "channel.") && !begins(&to_cards, "step."), "{to_cards:?}");
    // That the daemon started is anyone's who is signed in, and is all a
    // key that reads nothing is told.
    assert_eq!(to_nothing, BTreeSet::from(["system.started".to_string()]), "{to_nothing:?}");

    // The owner hears all of it; a member all but what is said of accounts.
    // (What the runner says of its runs is passed on and not kept, and is
    // not among what happened lately: that the run's instance started is.)
    let owner = recent(&rig, &rig.token).await;
    for topic in
        ["channel.message", "board.item.created", "instance.started", "identity.registered", "system.started"]
    {
        assert!(owner.contains(topic), "{topic}: {owner:?}");
    }
    let member = recent(&rig, &key(&rig, kernel::principal::MEMBER_SCOPES).await).await;
    let kept_from_a_member: Vec<&String> = owner.difference(&member).collect();
    // Among it, that the key they are asking with was made.
    assert_eq!(kept_from_a_member, ["identity.key.made", "identity.registered"], "{member:?}");
}

#[tokio::test]
async fn every_event_this_program_describes_says_who_may_be_told_of_it() {
    let rig = Rig::new().await;
    let scopes: Vec<&str> = kernel::principal::SCOPES.iter().map(|(scope, _)| *scope).collect();
    let registry = rig.node().app.engine.registry();
    assert!(registry.own_events().len() > 80);
    for spec in registry.own_events() {
        let Some(scope) = spec.hears.as_deref() else {
            panic!(
                "`{}` does not say who may be told of it; say so where its crate lists its events (`events.rs`)",
                spec.topic
            );
        };
        assert!(
            scope == blueprint::events::ANYONE || scopes.contains(&scope),
            "`{}` is told to whoever holds `{scope}`, which is not a scope there is",
            spec.topic
        );
    }
}

#[tokio::test]
async fn a_request_that_changes_something_and_says_it_comes_from_elsewhere_is_refused() {
    let rig = Rig::new().await;
    let own = rig.url.clone();
    let asked = async |method: Method, path: &str, headers: &[(&str, &str)]| {
        let mut request = reqwest::Client::new()
            .request(method, format!("{}{path}", rig.url))
            .bearer_auth(&rig.token)
            .json(&json!({ "name": "made" }));
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send().await.unwrap();
        (response.status(), response.json::<Value>().await.unwrap_or_default())
    };
    let port = rig.node().addr.port();
    // Another page on this machine: another port is another program. With
    // the owner's own credential, and all the same.
    let another_port = format!("http://127.0.0.1:{}", port.wrapping_add(1).max(1024));
    for headers in [
        &[(ORIGIN.as_str(), another_port.as_str())][..],
        &[(ORIGIN.as_str(), "https://evil.example")],
        &[(ORIGIN.as_str(), "null")],
        &[("sec-fetch-site", "cross-site")],
        &[("sec-fetch-site", "same-site")],
        &[(ORIGIN.as_str(), own.as_str()), ("sec-fetch-site", "cross-site")],
    ] {
        for (method, path) in [
            (Method::POST, "/api/channels"),
            (Method::POST, "/api/system/stop"),
            (Method::DELETE, "/api/projects/x"),
        ] {
            let (status, body) = asked(method.clone(), path, headers).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path} {headers:?}: {body}");
            assert!(said(&body).contains("says it comes from elsewhere"), "{body}");
        }
    }
    assert_eq!(rig.read("/api/channels").await["channels"], json!([]), "nothing was changed");

    // From its own pages, and from what says nothing of where it is (the
    // terminal, an agent), as before.
    for headers in [
        &[(ORIGIN.as_str(), own.as_str())][..],
        &[(ORIGIN.as_str(), own.as_str()), ("sec-fetch-site", "same-origin")],
        &[("sec-fetch-site", "none")],
        &[],
    ] {
        let (status, body) = asked(Method::POST, "/api/projects", headers).await;
        assert!(matches!(status, StatusCode::OK | StatusCode::CONFLICT), "{headers:?}: {status} {body}");
    }
    // Reading changes nothing, and is whoever's the credential lets it be.
    let (status, _) = asked(Method::GET, "/api/me", &[(ORIGIN.as_str(), "https://evil.example")]).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn starting_a_service_takes_being_allowed_to_run_commands() {
    let rig = Rig::new().await;
    let source = &"\
blueprint: hosted
permissions: [fleet]
steps:
  up: { do: service.start, with: { name: quiet, cmd: IDLES, restart: never } }
"
    .replace("IDLES", &crate::standin::cmd(&["idle"]));
    let hosts = key(&rig, &["blueprints", "fleet"]).await;
    let body = json!({ "source": source, "wait": true });
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&hosts), Some(body.clone())).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["status"], "failed", "{applied}");
    let why = applied["finished"]["error"].as_str().unwrap();
    assert!(
        why.contains("may not use `shell`, which starting a service needs as well as `fleet:write`"),
        "{why}"
    );
    assert_eq!(rig.read("/api/fleet").await["services"], json!([]), "nothing was started");

    // With both, it starts; whoever applied it stops it.
    let applied = rig.send("/api/apply", json!({ "source": source, "wait": true, "key": "owners" })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    rig.send(&format!("/api/instances/{}/stop", applied["instance"].as_str().unwrap()), json!({})).await;
}

#[tokio::test]
async fn a_projects_files_cannot_be_written_into_its_repository_or_through_a_link_to_nothing() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = rig.dir.join("data/projects/shop");
    let write = async |path: &str| {
        let body = json!({ "path": path, "content": "[core]\n\tfsmonitor = ./run-me\n" });
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await
    };
    // However it is written: where names are not told apart by their case
    // these are the repository's own folder, and where a dot or a space at
    // the end of a name is dropped, so are those.
    let refused = [
        ".git/config",
        ".GIT/config",
        ".Git/hooks/pre-commit",
        "docs/.gIt/config",
        ".git./config",
        ".git /config",
    ];
    // Whether this system tells names apart by their case: where it does
    // not, a folder asked for in capitals is there whenever the
    // repository's own is, and its being there says nothing.
    let told_apart = !dir.join(".GIT").exists();
    for path in refused {
        let (status, body) = write(path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
    }
    let (status, body) = write("notes.md.").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(said(&body).contains("ends with a dot or a space"), "{body}");
    assert!(!dir.join("notes.md.").exists());
    assert!(!told_apart || !dir.join(".GIT").exists());
    // And whatever the system makes of each name, none of them holds
    // what was sent.
    for path in refused {
        let there = std::fs::read_to_string(dir.join(path)).unwrap_or_default();
        assert!(!there.contains("run-me"), "{path} was written");
    }

    #[cfg(unix)]
    {
        // A link whose target is not there yet: writing to it would make
        // the target, wherever the link says that is.
        let outside = rig.dir.join("outside-the-project.txt");
        std::os::unix::fs::symlink(&outside, dir.join("looks-harmless.txt")).unwrap();
        let (status, body) = write("looks-harmless.txt").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(said(&body).contains("a link to something that is not there"), "{body}");
        assert!(!outside.exists());
    }
    let (status, body) = write("notes.md").await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

#[tokio::test]
async fn a_slots_credential_is_in_a_file_only_its_owner_can_read() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    let token = rig.dir.join("data/slots/helper/token");
    assert!(std::fs::read_to_string(&token).unwrap().starts_with("agt_"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&token).unwrap().permissions().mode() & 0o777, 0o600);
    }
}
