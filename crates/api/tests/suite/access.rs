use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

#[tokio::test]
async fn a_new_installation_says_it_needs_setting_up_and_the_first_person_owns_it() {
    let rig = Rig::fresh().await;
    let (status, meta) = rig.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!((meta["name"].as_str(), meta["cli"].as_str()), (Some(brand::DISPLAY), Some(brand::CLI)));
    assert_eq!((meta["needs_setup"].as_bool(), meta["device"].as_str()), (Some(true), Some("test-device")));

    let body = json!({ "email": "ada@example.com", "name": "Ada", "password": "correct horse" });
    let (status, signed) = rig.call(Method::POST, "/api/auth/register", None, Some(body)).await;
    assert_eq!(status, StatusCode::OK);
    let token = signed["token"].as_str().unwrap();
    assert_eq!(signed["account"]["role"], "owner");

    let (_, meta) = rig.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!((&meta["needs_setup"], &meta["signed_in"]), (&json!(false), &json!(false)));
    let (_, meta) = rig.call(Method::GET, "/api/meta", Some(token), None).await;
    assert_eq!(meta["signed_in"], true, "a client can tell whether to show the sign-in form");
    let (_, me) = rig.call(Method::GET, "/api/me", Some(token), None).await;
    assert!(me["actor"].as_str().unwrap().starts_with("user:acct_"), "{me}");
    assert_eq!((me["tier"].as_str(), me["can_invite"].as_bool()), (Some("free"), Some(false)));

    let wrong = json!({ "email": "ada@example.com", "password": "wrong horse" });
    let (status, error) = rig.call(Method::POST, "/api/auth/login", None, Some(wrong)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(error["error"]["kind"], "unauthenticated");
}

#[tokio::test]
async fn why_the_daemon_was_started_is_kept_with_its_start_in_the_journal() {
    let said = "The daemon was not running, so the window started it.";
    let rig = Rig::owned(Rig::with("api-note", |config, _| config.note = Some(said.into())).await).await;
    let started = |events: &Value| {
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["topic"] == "system.started")
            .cloned()
            .unwrap()
    };
    let noted = started(&rig.read("/api/events/recent?topics=system.*").await);
    assert_eq!(noted["data"]["note"], said);
    assert_eq!(noted["source"]["actor"], "system", "it is the factory's own to say");

    // Somebody who starts it with nothing to say leaves no note.
    let plain = Rig::new().await;
    assert_eq!(started(&plain.read("/api/events/recent?topics=system.*").await)["data"]["note"], Value::Null);
}

#[tokio::test]
async fn a_file_is_attached_to_a_message_fetched_back_and_an_agent_is_told_where_it_is() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    rig.send(&format!("/api/channels/{id}/agents"), json!({ "name": "builder" })).await;
    let attach = async |name: &str, kind: Option<&str>, bytes: Vec<u8>| {
        let url = format!("{}/api/channels/{id}/attachments?name={name}", rig.url);
        let mut request = rig.client.post(url).bearer_auth(&rig.token).body(bytes);
        if let Some(kind) = kind {
            request = request.header("content-type", kind);
        }
        let response = request.send().await.unwrap();
        (response.status(), response.json::<Value>().await.unwrap_or_default())
    };

    // The file is sent as it is, and is kept for the message that follows.
    let png: Vec<u8> = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
    let (status, picture) = attach("board.png", Some("image/png"), png.clone()).await;
    assert_eq!(status, StatusCode::OK, "{picture}");
    assert_eq!(
        (&picture["name"], &picture["media"], &picture["size"]),
        (&json!("board.png"), &json!("image/png"), &json!(7))
    );
    let (_, script) = attach("run.sh", None, b"echo hi\n".to_vec()).await;
    assert_eq!(
        script["media"], "application/octet-stream",
        "what it is, is what its name says when it does not say"
    );

    let body =
        json!({ "body": "@builder what is on the board?", "attachments": [picture["id"], script["id"]] });
    let said = rig.send(&format!("/api/channels/{id}/messages"), body).await;
    assert_eq!(said["attachments"], json!([picture, script]));
    let listed = rig.read(&format!("/api/channels/{id}/messages")).await;
    let first = &listed["messages"].as_array().unwrap()[0];
    assert_eq!(first["attachments"][1]["name"], "run.sh");
    // With something attached, nothing need be said.
    let (_, alone) = attach("plan.pdf", None, vec![1, 2]).await;
    let silent =
        rig.send(&format!("/api/channels/{id}/messages"), json!({ "attachments": [alone["id"]] })).await;
    assert_eq!(
        (&silent["body"], &silent["attachments"][0]["media"]),
        (&json!(""), &json!("application/pdf"))
    );

    // It is fetched back as it was. A picture is to be shown where it is;
    // anything else is only ever handed over to be saved, never opened as
    // a page of this site.
    let fetch = async |attachment: &Value, token: Option<&str>| {
        let mut request =
            rig.client.get(format!("{}/api/attachments/{}", rig.url, attachment["id"].as_str().unwrap()));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        let header = |name: &str| {
            response.headers().get(name).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default()
        };
        // Its address may say which page asked: nothing it leads to is told that address.
        if response.status() == StatusCode::OK {
            assert_eq!(header("referrer-policy"), "no-referrer");
        }
        let told = (
            response.status(),
            header("content-type"),
            header("content-disposition"),
            header("x-content-type-options"),
        );
        (told, response.bytes().await.unwrap().to_vec())
    };
    let ((status, kind, how, sniff), bytes) = fetch(&picture, Some(&rig.token)).await;
    assert_eq!(
        (status, kind.as_str(), how.as_str(), sniff.as_str()),
        (StatusCode::OK, "image/png", "inline; filename=\"board.png\"", "nosniff")
    );
    assert_eq!(bytes, png);
    let ((_, kind, how, _), bytes) = fetch(&script, Some(&rig.token)).await;
    assert_eq!(
        (kind.as_str(), how.as_str()),
        ("application/octet-stream", "attachment; filename=\"run.sh\"")
    );
    assert_eq!(bytes, b"echo hi\n");
    let ((status, ..), _) = fetch(&picture, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "what is attached is no more public than what is said");

    // The agent that was asked is told the command that reads each, as
    // whoever it is, and never where its file is on this device, which
    // means nothing behind a wall or on another.
    rig.event("agent.turn.finished", |event| event.data["slot"] == "builder").await;
    let turns = rig.read("/api/agents/builder/turns").await;
    let prompt = turns["turns"][0]["prompt"].as_str().unwrap();
    let cli = brand::CLI;
    let told = format!(
        "attached: board.png (image/png), read with `{cli} attachment {} --out <file>`",
        picture["id"].as_str().unwrap()
    );
    assert!(prompt.contains(&told), "{prompt}");
    let told = format!(
        "attached: run.sh (application/octet-stream), read with `{cli} attachment {} --out <file>`",
        script["id"].as_str().unwrap()
    );
    assert!(prompt.contains(&told), "{prompt}");
    assert!(!prompt.contains(" at /"), "{prompt}");

    // More than may be attached is refused before it is kept.
    let (status, _) = attach("huge.bin", None, vec![0u8; 33 * 1024 * 1024]).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn a_page_that_is_attached_is_shown_as_a_view_held_in_a_box_of_its_own() {
    let rig = Rig::new().await;
    let channel = rig.send("/api/channels", json!({ "name": "general" })).await;
    let id = channel["id"].as_str().unwrap().to_string();
    let attach = async |name: &str, bytes: &[u8]| {
        let url = format!("{}/api/channels/{id}/attachments?name={name}", rig.url);
        let response =
            rig.client.post(url).bearer_auth(&rig.token).body(bytes.to_vec()).send().await.unwrap();
        response.json::<Value>().await.unwrap()
    };
    let get = async |path: String, token: Option<&str>| {
        let mut request = rig.client.get(format!("{}{path}", rig.url));
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request.send().await.unwrap();
        let header = |name: &str| {
            response.headers().get(name).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default()
        };
        let told = (response.status(), header("content-type"), header("content-security-policy"));
        (told, response.text().await.unwrap())
    };

    // A page is one by its name; it goes with a message like anything else.
    let page = "<h1>Build times</h1><canvas id=\"chart\"></canvas><script>draw()</script>";
    let chart = attach("build-times.html", page.as_bytes()).await;
    assert_eq!(chart["media"], "text/html");
    let picture = attach("board.png", &[1, 2, 3]).await;
    let said = json!({ "body": "this week", "attachments": [chart["id"], picture["id"]] });
    rig.send(&format!("/api/channels/{id}/messages"), said).await;

    // Shown, it is its own HTML after a start that tells the channel how
    // tall it is, and it is held to a box: no origin of its own, scripts
    // allowed, nothing over the network.
    let view = format!("/api/attachments/{}/view", chart["id"].as_str().unwrap());
    let ((status, kind, policy), body) = get(view.clone(), Some(&rig.token)).await;
    assert_eq!((status, kind.as_str()), (StatusCode::OK, "text/html; charset=utf-8"));
    assert!(policy.starts_with("sandbox allow-scripts; default-src 'none';"), "{policy}");
    assert!(!policy.contains("allow-same-origin") && !policy.contains("connect-src"), "{policy}");
    assert!(body.starts_with("<!doctype html>") && body.ends_with(page), "{body}");
    assert!(
        body.contains(&format!("id=\"{}\"", chart["id"].as_str().unwrap())),
        "the start says which view is speaking"
    );
    let ((status, ..), _) = get(view, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Asked for as a file, it is only ever handed over to be saved: never a page of this site.
    let ((_, kind, _), _) =
        get(format!("/api/attachments/{}", chart["id"].as_str().unwrap()), Some(&rig.token)).await;
    assert_eq!(kind, "application/octet-stream");
    // And what is not a page has nothing to show.
    let ((status, ..), why) =
        get(format!("/api/attachments/{}/view", picture["id"].as_str().unwrap()), Some(&rig.token)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(why.contains("is not a page"), "{why}");

    // A blueprint shows one with what it says.
    let source = "blueprint: chart\npermissions: [channels]\nsteps:\n  tell:\n    do: channel.post\n    with: { channel: general, body: \"As drawn\", view: \"<p>drawn</p>\" }\n";
    let applied = rig.send("/api/apply", json!({ "source": source, "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let listed = rig.read(&format!("/api/channels/{id}/messages")).await;
    let last = listed["messages"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        (&last["body"], &last["attachments"][0]["name"], &last["attachments"][0]["media"]),
        (&json!("As drawn"), &json!("view.html"), &json!("text/html"))
    );
    let drawn = format!("/api/attachments/{}/view", last["attachments"][0]["id"].as_str().unwrap());
    assert!(get(drawn, Some(&rig.token)).await.1.ends_with("<p>drawn</p>"));
}

#[tokio::test]
async fn nothing_is_served_to_a_request_without_a_credential() {
    let rig = Rig::new().await;
    let routes = [
        "/api/me",
        "/api/events/recent",
        "/api/projects",
        "/api/blueprints",
        "/api/blueprints/used",
        "/api/actions",
        "/api/blueprints/language",
        "/api/blueprints/schema",
        "/api/instances",
        "/api/runs",
        "/api/channels",
        "/api/collections",
        "/api/agents",
        "/api/fleet",
        "/api/search?q=x",
        "/api/metrics?name=http",
        "/api/metrics/labels",
        "/api/accounts",
        "/api/credentials",
    ];
    for route in routes {
        let (status, body) = rig.call(Method::GET, route, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{route} answered without a credential: {body}");
        let (status, _) = rig.call(Method::GET, route, Some("ses_made-up-token"), None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{route} accepted a made-up token");
        let (status, _) = rig.get(route).await;
        assert_eq!(status, StatusCode::OK, "{route} refused its owner");
    }
    let (status, body) =
        rig.call(Method::POST, "/api/apply", None, Some(json!({ "source": "blueprint: x" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    let (status, _) = rig.call(Method::GET, "/api/events", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "the event stream is not public either");
}

#[tokio::test]
async fn a_browser_signs_in_with_a_cookie_that_scripts_cannot_read_and_that_counts_only_with_its_pages_key() {
    let rig = Rig::new().await;
    let client = reqwest::Client::new();
    let at = |path: &str| format!("{}{path}", rig.url);
    let login = json!({ "email": "ada@example.com", "password": "correct horse", "page": true });
    let response = client.post(at("/api/auth/login")).json(&login).send().await.unwrap();
    let cookie = response.headers()["set-cookie"].to_str().unwrap().to_string();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"), "{cookie}");
    let session = cookie.split(';').next().unwrap().to_string();
    let signed: Value = response.json().await.unwrap();
    let key = signed["page"].as_str().unwrap().to_string();
    assert!(
        signed["token"].is_null(),
        "a page is handed its key, and its token only in the cookie: {signed}"
    );

    // With the key its page was given, the cookie is who it says.
    let asked = |method: Method, path: &str| client.request(method, at(path)).header("cookie", &session);
    let me = asked(Method::GET, "/api/me").header("x-page-key", &key).send().await.unwrap();
    assert_eq!(me.status(), StatusCode::OK);

    // By itself it is nobody's word. A browser sends it to every port of
    // the machine it was set for and with any link followed there, so
    // whatever listens on another port has it; what it lacks is the key.
    for (method, path) in [
        (Method::GET, "/api/me"),
        (Method::GET, "/api/channels"),
        (Method::GET, "/api/credentials"),
        (Method::POST, "/api/credentials"),
        (Method::POST, "/api/apply"),
        (Method::POST, "/api/emit/shop.said"),
    ] {
        let alone = asked(method.clone(), path).json(&json!({ "name": "taken" })).send().await.unwrap();
        assert_eq!(alone.status(), StatusCode::UNAUTHORIZED, "{method} {path}");
    }
    let wrong = asked(Method::GET, "/api/me").header("x-page-key", "not-the-key").send().await.unwrap();
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    // Nor is it a token to be said: said, it is a cookie somebody read.
    let token = session.split_once('=').unwrap().1;
    let said = client.get(at("/api/me")).bearer_auth(token).header("x-page-key", &key).send().await.unwrap();
    assert_eq!(said.status(), StatusCode::UNAUTHORIZED);

    // What a browser fetches by itself and only reads cannot carry the
    // key: the stream of events, a file or a picture shown. It counts with
    // what the page was given to say in its address, which is not the key.
    // The cookie reads nothing alone, the address nothing without the
    // cookie, and neither is a way to ask as the page asks.
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let shows = signed["shows"].as_str().unwrap().to_string();
    assert_ne!(shows, key);
    for path in ["/api/events", "/api/projects/shop/raw?path=docs/overview.md"] {
        let with = |by: &str| format!("{path}{}by={by}", if path.contains('?') { '&' } else { '?' });
        let alone = asked(Method::GET, path).send().await.unwrap();
        assert_eq!(alone.status(), StatusCode::UNAUTHORIZED, "{path} was read with the cookie alone");
        let said = asked(Method::GET, &with(&shows)).send().await.unwrap();
        assert_eq!(said.status(), StatusCode::OK, "{path}");
        // A file shown says its own address to nothing it leads to.
        if path.contains("/raw") {
            assert_eq!(said.headers()["referrer-policy"], "no-referrer");
        }
        drop(said);
        for wrong in ["not-what-it-shows", key.as_str(), ""] {
            let refused = asked(Method::GET, &with(wrong)).send().await.unwrap();
            assert_eq!(refused.status(), StatusCode::UNAUTHORIZED, "{path} with `{wrong}`");
        }
        let no_cookie = client.get(at(&with(&shows))).send().await.unwrap();
        assert_eq!(no_cookie.status(), StatusCode::UNAUTHORIZED, "{path} with the address alone");
    }
    // Said in the address of anything else, it is not heard: not of what
    // only begins or ends as one of the three does, a project that is
    // called `raw` and something attached shown as a page among them.
    rig.send("/api/projects", json!({ "name": "Raw" })).await;
    let with_key = asked(Method::GET, "/api/projects/raw").header("x-page-key", &key).send().await.unwrap();
    assert_eq!(with_key.status(), StatusCode::OK, "there is a project called `raw`");
    for path in ["/api/projects/raw", "/api/attachments/att_any/view", "/api/events/recent"] {
        let refused = asked(Method::GET, &format!("{path}?by={shows}")).send().await.unwrap();
        assert_eq!(refused.status(), StatusCode::UNAUTHORIZED, "{path}");
    }
    for (method, path) in
        [(Method::GET, "/api/me"), (Method::GET, "/api/channels"), (Method::POST, "/api/channels")]
    {
        let asked = asked(method.clone(), &format!("{path}?by={shows}")).json(&json!({ "name": "taken" }));
        assert_eq!(asked.send().await.unwrap().status(), StatusCode::UNAUTHORIZED, "{method} {path}");
    }
    // A page that signed in before it was given this asks for it, as itself.
    let again = asked(Method::GET, "/api/auth/shows").header("x-page-key", &key).send().await.unwrap();
    assert_eq!(again.json::<Value>().await.unwrap()["shows"], shows.as_str());
    let alone = asked(Method::GET, "/api/auth/shows").send().await.unwrap();
    assert_eq!(alone.status(), StatusCode::UNAUTHORIZED);

    // What keeps its own token is given one, and no cookie: a token is
    // not good as a cookie either.
    let plain = json!({ "email": "ada@example.com", "password": "correct horse" });
    let response = client.post(at("/api/auth/login")).json(&plain).send().await.unwrap();
    assert!(response.headers().get("set-cookie").is_none());
    let kept = response.json::<Value>().await.unwrap()["token"].as_str().unwrap().to_string();
    let me = client.get(at("/api/me")).bearer_auth(&kept).send().await.unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    let as_cookie =
        client.get(at("/api/me")).header("cookie", format!("session={kept}")).send().await.unwrap();
    assert_eq!(as_cookie.status(), StatusCode::UNAUTHORIZED);

    let out = asked(Method::POST, "/api/auth/logout").send().await.unwrap();
    assert!(out.headers()["set-cookie"].to_str().unwrap().contains("Max-Age=0"));
    let me = asked(Method::GET, "/api/me").header("x-page-key", &key).send().await.unwrap();
    assert_eq!(
        me.status(),
        StatusCode::UNAUTHORIZED,
        "signing out ends the session on the server, not just in the browser"
    );
}

#[tokio::test]
async fn an_agents_credential_does_what_its_slot_may_and_nothing_else() {
    let rig = Rig::new().await;
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    let token = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let as_agent = |method: Method, path: &'static str, body: Option<Value>| {
        let (rig, token) = (&rig, token.clone());
        async move { rig.call(method, path, Some(&token), body).await }
    };

    let (status, me) = as_agent(Method::GET, "/api/me", None).await;
    assert_eq!((status, me["actor"].as_str()), (StatusCode::OK, Some("agent:helper")));
    let (status, _) =
        as_agent(Method::POST, "/api/channels/general/messages", Some(json!({ "body": "hello" }))).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) =
        as_agent(Method::POST, "/api/projects", Some(json!({ "name": "Made By An Agent" }))).await;
    assert_eq!(status, StatusCode::OK, "an agent may create projects");

    for (method, path, body) in [
        (Method::GET, "/api/accounts", None),
        (Method::GET, "/api/credentials", None),
        (Method::POST, "/api/tier", Some(json!({ "tier": "enterprise" }))),
        (Method::POST, "/api/agents", Some(json!({ "name": "minion" }))),
        (Method::POST, "/api/credentials", Some(json!({ "name": "escalate" }))),
    ] {
        let (status, body) = as_agent(method.clone(), path, body).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{method} {path}: {body}");
        assert_eq!(body["error"]["kind"], "denied");
    }

    // It reads what is running and starts none of it, unless its slot
    // says it may: whoever may apply may stop any instance there is.
    let source = "blueprint: sneaky\npermissions: [shell]\nsteps:\n  run: { do: shell, with: { run: id } }\n";
    let asked = json!({ "source": source, "wait": true });
    let (status, refused) = as_agent(Method::POST, "/api/apply", Some(asked.clone())).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(refused["error"]["message"].as_str().unwrap().contains("needs `blueprints:run`"), "{refused}");

    // One that may: a blueprint it applies runs with its authority, and no shell.
    rig.send("/api/channels/general/agents", json!({ "name": "runner", "scopes": ["blueprints"] })).await;
    let runner = std::fs::read_to_string(rig.dir.join("data/slots/runner/token")).unwrap();
    let (status, applied) = rig.call(Method::POST, "/api/apply", Some(runner.trim()), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["steps"][0]["error"], "runner may not use `shell`");
}

#[tokio::test]
async fn working_with_others_is_gated_by_tier() {
    let rig = Rig::new().await;
    let (status, error) = rig.post("/api/invites", json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(error["error"]["message"].as_str().unwrap().contains("premium or enterprise"));

    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    let code = rig.send("/api/invites", json!({ "email": "grace@example.com" })).await["code"]
        .as_str()
        .unwrap()
        .to_string();
    let body =
        json!({ "email": "grace@example.com", "name": "Grace", "password": "correct horse", "invite": code });
    let (status, grace) = rig.call(Method::POST, "/api/auth/register", None, Some(body)).await;
    assert_eq!((status, grace["account"]["role"].as_str()), (StatusCode::OK, Some("member")));
    assert_eq!(rig.read("/api/accounts").await["accounts"].as_array().unwrap().len(), 2);
    assert_eq!(rig.read("/api/me").await["can_invite"], true);
}

#[tokio::test]
async fn files_are_read_and_written_through_the_api_and_cannot_escape_their_project() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let write = |path: &'static str| {
        rig.call(
            Method::PUT,
            "/api/projects/shop/file",
            Some(&rig.token),
            Some(json!({ "path": path, "content": "hello\n" })),
        )
    };

    let (status, entry) = write("notes/today.md").await;
    assert_eq!((status, entry["size"].as_u64()), (StatusCode::OK, Some(6)));
    assert_eq!(rig.read("/api/projects/shop/file?path=notes/today.md").await["content"], "hello\n");
    // Whether a path is a file, asked with nothing of it read: its entry, and nothing for a path that is none.
    let there = rig.read("/api/projects/shop/file?path=notes/today.md&head").await;
    assert_eq!(
        (&there["path"], &there["dir"], &there["size"], there.get("content")),
        (&json!("notes/today.md"), &json!(false), &json!(6), None)
    );
    assert_eq!(rig.read("/api/projects/shop/file?path=notes&head=true").await["dir"], true);
    let (status, _) = rig.get("/api/projects/shop/file?path=notes/missing.md&head").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let tree = rig.read("/api/projects/shop/tree").await;
    assert!(tree["entries"].as_array().unwrap().iter().any(|entry| entry["path"] == "notes/today.md"));

    for path in ["../outside.md", "/etc/passwd", ".git/config"] {
        let (status, error) = write(path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {error}");
    }
    let (status, _) = rig.get("/api/projects/shop/file?path=../../data/journal.db").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = rig.get("/api/projects/shop/file?path=missing.md").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) =
        rig.call(Method::DELETE, "/api/projects/shop/file?path=notes", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_file_that_is_not_text_is_handed_over_as_it_is_and_cannot_run() {
    let rig = Rig::new().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    // The start of a PNG, which is not text, and a drawing with a script in it.
    let picture: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0xff, 0xfe];
    std::fs::write(dir.join("logo.png"), picture).unwrap();
    std::fs::write(
        dir.join("drawing.svg"),
        "<svg xmlns='http://www.w3.org/2000/svg'><script>alert(1)</script></svg>",
    )
    .unwrap();

    let (status, error) = rig.get("/api/projects/shop/file?path=logo.png").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "it is not opened as text: {error}");

    let raw = async |path: &str| {
        let request = reqwest::Client::new().get(format!("{}/api/projects/shop/raw?path={path}", rig.url));
        request.bearer_auth(&rig.token).send().await.unwrap()
    };
    let answer = raw("logo.png").await;
    assert_eq!(answer.status(), StatusCode::OK);
    assert_eq!(answer.headers()["content-type"], "image/png");
    assert_eq!(answer.bytes().await.unwrap().as_ref(), picture);

    // Whatever it is, the browser is told not to guess and not to run it.
    let answer = raw("drawing.svg").await;
    assert_eq!(answer.headers()["content-type"], "image/svg+xml");
    assert_eq!(answer.headers()["x-content-type-options"], "nosniff");
    assert!(answer.headers()["content-security-policy"].to_str().unwrap().starts_with("sandbox"));

    assert_eq!(
        raw("../../data/journal.db").await.status(),
        StatusCode::BAD_REQUEST,
        "it stays in its project"
    );
    assert_eq!(raw("missing.png").await.status(), StatusCode::NOT_FOUND);
    let unsigned = reqwest::get(format!("{}/api/projects/shop/raw?path=logo.png", rig.url)).await.unwrap();
    assert_eq!(unsigned.status(), StatusCode::UNAUTHORIZED, "and is nobody's who has not signed in");
}

#[tokio::test]
async fn unknown_api_routes_are_json_and_everything_else_is_the_client() {
    let rig = Rig::new().await;
    let (status, error) = rig.get("/api/no/such/thing").await;
    // Its own kind, so a client newer than the daemon can tell "the daemon
    // has not learned this yet" from "that thing does not exist".
    assert_eq!((status, error["error"]["kind"].as_str()), (StatusCode::NOT_FOUND, Some("no_such_route")));
    let message = error["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("GET /api/no/such/thing") && message.contains("restart the daemon"),
        "{message}"
    );

    let page = reqwest::get(format!("{}/projects/anything", rig.url)).await.unwrap();
    assert_eq!(page.status(), StatusCode::OK);
    assert!(page.headers()["content-type"].to_str().unwrap().starts_with("text/html"));
}

#[tokio::test]
async fn a_restart_keeps_sessions_data_and_the_event_sequence() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    rig.board("shop").await;
    rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let before = rig.read("/api/events/recent?limit=1").await["last_seq"].as_u64().unwrap();

    rig.restart().await;
    let me = rig.read("/api/me").await;
    assert_eq!(me["account"]["email"], "ada@example.com", "the session is still good");
    assert_eq!(rig.read("/api/board/shop").await["items"][0]["title"], "Add a cart");
    // Enough of them to reach back past what the card's own blueprint does on either side of the restart.
    let after = rig.read("/api/events/recent?limit=60").await;
    assert!(
        after["last_seq"].as_u64().unwrap() > before,
        "the sequence carries on rather than starting again"
    );
    let topics: Vec<&str> =
        after["events"].as_array().unwrap().iter().filter_map(|event| event["topic"].as_str()).collect();
    assert!(topics.contains(&"system.started") && topics.contains(&"system.stopped"), "{topics:?}");
}

#[tokio::test]
async fn only_the_owner_can_ask_the_daemon_to_stop() {
    let rig = Rig::new().await;
    let (status, _) = rig.call(Method::POST, "/api/system/stop", None, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // An agent's credential is good for its work, not for this.
    rig.send("/api/channels", json!({ "name": "general" })).await;
    rig.send("/api/channels/general/agents", json!({ "name": "helper" })).await;
    let token = std::fs::read_to_string(rig.dir.join("data/slots/helper/token")).unwrap();
    let (status, error) =
        rig.call(Method::POST, "/api/system/stop", Some(token.trim()), Some(json!({}))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{error}");

    assert_eq!(rig.send("/api/system/stop", json!({})).await["stopping"], true);
    crate::soon("the node to be told to stop", rig.node().asked_to_stop()).await;
}

#[tokio::test]
async fn a_document_is_previewed_as_the_page_it_would_be_published_as() {
    let rig = Rig::new().await;
    let (status, _) =
        rig.call(Method::POST, "/api/pages/preview", None, Some(json!({ "source": "# Hello" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // The text is sent, so what is previewed is what is in the editor.
    let answer = rig
        .send("/api/pages/preview", json!({ "source": "# Hello\n\nSome *text*.", "name": "notes.md" }))
        .await;
    assert_eq!(answer["title"], "Hello");
    let html = answer["html"].as_str().unwrap();
    assert!(html.contains("<h1>Hello</h1>") && html.contains("<em>text</em>"), "{html}");
    assert!(html.contains("<title>Hello</title>") && html.contains("<style>"));
}

#[tokio::test]
async fn a_request_that_comes_through_a_proxy_is_answered_under_a_name_the_daemon_was_told_and_still_has_to_sign_in()
 {
    // A proxy on this device, put in front of the daemon so that it can be
    // opened from elsewhere, sends requests on from this device's own
    // address, named for the address that was opened. Coming from here
    // proves nothing, and neither does the name: one the daemon was not
    // told is its own is not answered, since a page anywhere can point a
    // name of its making at this machine.
    let rig = Rig::new().await;
    let unknown = reqwest::Client::new()
        .get(format!("{}/api/meta", rig.url))
        .header(reqwest::header::HOST, "box.example.net:8443")
        .send()
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::FORBIDDEN);
    let said = unknown.json::<Value>().await.unwrap()["error"]["message"].as_str().unwrap().to_string();
    assert!(said.contains("was asked as `box.example.net:8443`"), "{said}");
    assert!(said.contains("names add box.example.net"), "{said}");
    // Its owner says the name, and from then on it is one of its own.
    rig.send("/api/system/names", json!({ "name": "box.example.net" })).await;
    let through = |method: Method, path: &str| {
        reqwest::Client::new()
            .request(method, format!("{}{path}", rig.url))
            .header(reqwest::header::HOST, "box.example.net:8443")
            .header("x-forwarded-for", "100.64.0.9")
            .header("x-forwarded-proto", "https")
    };
    let status = async |request: reqwest::RequestBuilder| request.send().await.unwrap().status();

    // What anyone may have: the page, and what it needs before signing in.
    assert_eq!(status(through(Method::GET, "/")).await, status(reqwest::Client::new().get(&rig.url)).await);
    assert_eq!(status(through(Method::GET, "/api/meta")).await, StatusCode::OK);
    // Everything else is for someone who has signed in.
    for (method, path) in [
        (Method::GET, "/api/me"),
        (Method::GET, "/api/events"),
        (Method::GET, "/api/blueprints"),
        (Method::GET, "/api/extensions"),
        (Method::POST, "/api/apply"),
        (Method::POST, "/api/system/stop"),
    ] {
        let asked = through(method.clone(), path).json(&json!({ "blueprint": "std/board" }));
        assert_eq!(status(asked).await, StatusCode::UNAUTHORIZED, "{method} {path}");
    }
    let wrong = json!({ "email": "ada@example.com", "password": "wrong horse" });
    assert_eq!(status(through(Method::POST, "/api/auth/login").json(&wrong)).await, StatusCode::UNAUTHORIZED);

    // Signing in works as it does here, and a browser's session with it.
    let right = json!({ "email": "ada@example.com", "password": "correct horse", "page": true });
    let signed = through(Method::POST, "/api/auth/login").json(&right).send().await.unwrap();
    assert_eq!(signed.status(), StatusCode::OK);
    let cookie = signed.headers()[reqwest::header::SET_COOKIE].to_str().unwrap().to_string();
    let session = cookie.split(';').next().unwrap().to_string();
    let signed = signed.json::<Value>().await.unwrap();
    let (key, shows) = (signed["page"].as_str().unwrap().to_string(), signed["shows"].as_str().unwrap());
    let me = through(Method::GET, "/api/me")
        .header(reqwest::header::COOKIE, &session)
        .header("x-page-key", &key)
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK);
    assert_eq!(me.json::<Value>().await.unwrap()["account"]["email"], "ada@example.com");
    // And what happens is followed as it happens, by a page that says in
    // the stream's address which page it is.
    let events = through(Method::GET, &format!("/api/events?by={shows}"))
        .header(reqwest::header::COOKIE, &session)
        .send()
        .await
        .unwrap();
    assert_eq!(events.status(), StatusCode::OK);
    assert_eq!(events.headers()[reqwest::header::CONTENT_TYPE], "text/event-stream");
}
