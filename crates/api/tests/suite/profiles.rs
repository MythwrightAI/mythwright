//! A factory has a profile: a name it is given once and keeps. The
//! factories on a device share nothing, and know of each other by name and
//! address only.

use std::path::{Path, PathBuf};

use api::{Config, Node};
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// A factory called `name`, on the list of factories kept at `registry`.
async fn factory(name: Option<&str>, registry: &Path) -> Rig {
    let (name, registry) = (name.map(str::to_string), registry.to_path_buf());
    let label = format!("api-env-{}", name.as_deref().unwrap_or("unnamed"));
    Rig::owned(
        Rig::with(&label, move |config, _| {
            config.profile = name;
            config.registry = Some(registry);
        })
        .await,
    )
    .await
}

/// A folder for what several factories in one test share: the list they
/// are all on, and directories they might each want.
struct Shared(PathBuf);

impl Shared {
    fn new() -> Shared {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("api-env-shared-{}", &kernel::id::secret()[..12]));
        std::fs::create_dir_all(&dir).unwrap();
        Shared(dir)
    }

    fn registry(&self) -> PathBuf {
        self.0.join("environments.json")
    }

    /// A directory with a file in it, as somebody's existing work would be.
    fn work(&self, name: &str) -> PathBuf {
        let dir = self.0.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# Existing work\n").unwrap();
        dir
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Why starting a node on `config` was refused.
async fn refused(config: Config) -> String {
    match Node::start(config).await {
        Ok(_) => panic!("the node started"),
        Err(err) => err.message,
    }
}

/// What a factory lists as the factories on its device, asked as a page
/// reached at `host` would ask, with whatever else a proxy would add.
async fn listed(rig: &Rig, host: Option<&str>, more: &[(&str, &str)]) -> Value {
    let mut request = reqwest::Client::new().get(format!("{}/api/profiles", rig.url)).bearer_auth(&rig.token);
    if let Some(host) = host {
        request = request.header("host", host);
    }
    for (name, value) in more {
        request = request.header(*name, *value);
    }
    let response = request.send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await.unwrap()
}

fn one<'a>(listed: &'a Value, name: &str) -> &'a Value {
    listed["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap_or_else(|| panic!("`{name}` is not listed: {listed}"))
}

fn port(rig: &Rig) -> u16 {
    rig.node().addr.port()
}

async fn adopt(rig: &Rig, name: &str, dir: &Path) -> (StatusCode, Value) {
    rig.post("/api/projects", json!({ "name": name, "path": dir })).await
}

#[tokio::test]
async fn a_factory_started_with_a_name_says_what_it_is_called_and_one_without_says_none() {
    let shared = Shared::new();
    let named = factory(Some("dev1"), &shared.registry()).await;
    let (_, meta) = named.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!(meta["profile"], "dev1", "before anyone has signed in: a page says where it is at once");
    assert_eq!(meta["environment"], "dev1", "and as a page open since before 0.1.0 reads it");
    assert_eq!(named.node().profile(), Some("dev1"));

    let unnamed = Rig::new().await;
    let (_, meta) = unnamed.call(Method::GET, "/api/meta", None, None).await;
    assert_eq!(meta["profile"], Value::Null);
    // And is as it was before factories had profiles: nothing is
    // written into its data to say what it is, and it is on no list.
    assert!(!unnamed.dir.join("data").join(api::profile::STAMP_FILE).exists());
    // Said twice over: as `environment` too, for a page left open since a
    // program from before 0.1.0, which called a profile that.
    assert_eq!(
        unnamed.read("/api/profiles").await,
        json!({ "profile": null, "profiles": [], "environment": null, "environments": [] })
    );
    assert_eq!(unnamed.read("/api/environments").await, unnamed.read("/api/profiles").await);
    assert_eq!((&meta["profile"], &meta["environment"]), (&Value::Null, &Value::Null));
}

#[tokio::test]
async fn a_data_folder_keeps_the_name_it_was_first_given_and_refuses_another() {
    let shared = Shared::new();
    let mut rig = factory(Some("prod"), &shared.registry()).await;
    rig.stop().await;

    let said = refused(Config { profile: Some("dev1".into()), ..rig.config.clone() }).await;
    let data = rig.dir.join("data");
    assert!(said.contains(&format!("{} is the data of a `prod` factory", data.display())), "{said}");
    assert!(said.contains("was asked to start as `dev1`"), "{said}");
    assert!(said.contains("A data folder belongs to one profile for good"), "{said}");

    // Started with no name at all, it is still what it was.
    rig.config.profile = None;
    rig.start().await;
    assert_eq!(rig.read("/api/meta").await["profile"], "prod");
    assert!(rig.read("/api/me").await["account"]["email"] == "ada@example.com", "and has what it had");
}

#[tokio::test]
async fn a_folder_from_before_profiles_is_stamped_by_the_first_name_it_is_started_with() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Riverside Shop" })).await;
    rig.stop().await;

    rig.config.profile = Some("prod".into());
    rig.start().await;
    assert_eq!(rig.read("/api/meta").await["profile"], "prod");
    assert_eq!(
        rig.read("/api/projects").await["projects"][0]["id"],
        "riverside-shop",
        "what it held, it holds"
    );

    // And from then on it is that, and nothing else.
    rig.stop().await;
    let said = refused(Config { profile: Some("staging".into()), ..rig.config.clone() }).await;
    assert!(said.contains("is the data of a `prod` factory"), "{said}");
}

#[tokio::test]
async fn a_name_is_a_lowercase_word() {
    for name in ["Prod", "dev 1", "", "-dev", "dev_1", "../prod", &"x".repeat(40)] {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("api-env-name-{}", &kernel::id::secret()[..12]));
        let config = Config {
            listen: "127.0.0.1:0".parse().unwrap(),
            profile: Some(name.to_string()),
            walls: Some(api::Walls::Open),
            ..Config::new(dir.join("data"))
        };
        let said = refused(config).await;
        assert!(said.contains("cannot be a profile's name"), "`{name}`: {said}");
        assert!(said.contains("such as `prod` or `dev1`"), "{said}");
        assert!(!dir.exists(), "`{name}`: nothing is made for a factory that is refused");
    }
}

#[tokio::test]
async fn a_stamp_that_cannot_be_read_is_not_taken_for_no_stamp() {
    let shared = Shared::new();
    let mut rig = factory(Some("prod"), &shared.registry()).await;
    rig.stop().await;
    let stamp = rig.dir.join("data").join(api::profile::STAMP_FILE);
    std::fs::write(&stamp, "{ not what it was").unwrap();

    // Or production's data could be started as anything.
    let said = refused(Config { profile: Some("dev1".into()), ..rig.config.clone() }).await;
    assert!(said.contains("says which profile this data folder belongs to, and cannot be read"), "{said}");
    assert_eq!(
        std::fs::read_to_string(&stamp).unwrap(),
        "{ not what it was",
        "and it is left to be put right"
    );
}

#[tokio::test]
async fn a_named_factory_given_no_address_takes_a_free_one_and_has_it_again_every_time_after() {
    let shared = Shared::new();
    let registry = shared.registry();
    let rig = Rig::with("api-env-address", |config, _| {
        config.profile = Some("dev1".into());
        config.registry = Some(registry);
        // What a factory with no profile listens on, were nothing said.
        config.listen = "127.0.0.1:7700".parse().unwrap();
        config.keep_address = true;
    })
    .await;
    let mut rig = Rig::owned(rig).await;
    let first = rig.node().addr;
    assert!(first.ip().is_loopback(), "on this device only");
    assert_ne!(first.port(), 7700, "never where a factory with no profile would be");

    rig.restart().await;
    assert_eq!(rig.node().addr, first, "the same address, so what was pointed at it still is");

    // Told where to listen, it listens there, and that is then what it
    // remembers. It is stopped and comes back there, so the port is one
    // that is this test's own meanwhile (`crate::ports`): any port the
    // system gave it would be the system's to give again while it is
    // stopped.
    let own = crate::ports::free();
    rig.config.keep_address = false;
    rig.config.listen = std::net::SocketAddr::from(([127, 0, 0, 1], own.number()));
    rig.restart().await;
    let told = rig.node().addr;
    assert_eq!(told, rig.config.listen);
    rig.config.keep_address = true;
    rig.config.listen = "127.0.0.1:7700".parse().unwrap();
    rig.restart().await;
    assert_eq!(rig.node().addr, told);

    // Something else has the address it remembers: it says so, and does not
    // go elsewhere unasked. (The address is put into what it remembers
    // here, so that it is one this test holds.)
    rig.stop().await;
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let free = taken.local_addr().unwrap();
    let stamp = rig.dir.join("data").join(api::profile::STAMP_FILE);
    let mut kept: Value = serde_json::from_str(&std::fs::read_to_string(&stamp).unwrap()).unwrap();
    assert_eq!(kept["address"], told.to_string());
    kept["address"] = json!(free.to_string());
    std::fs::write(&stamp, kept.to_string()).unwrap();
    let said = refused(rig.config.clone()).await;
    assert!(said.contains(&format!("cannot listen on {free}")), "{said}");
    assert!(said.contains("so that its address stays the same"), "{said}");
}

#[tokio::test]
async fn two_factories_on_one_device_share_nothing_and_each_lists_the_other() {
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    let dev = factory(Some("dev1"), &shared.registry()).await;
    assert_ne!(prod.dir, dev.dir);
    assert_ne!(prod.node().addr, dev.node().addr);

    // What is made in one is not in the other.
    prod.send("/api/projects", json!({ "name": "Riverside Shop" })).await;
    prod.board("riverside-shop").await;
    prod.send("/api/board/riverside-shop/items", json!({ "title": "Ship it" })).await;
    prod.send("/api/channels", json!({ "name": "general" })).await;
    assert_eq!(dev.read("/api/projects").await["projects"], json!([]));
    assert_eq!(dev.read("/api/channels").await["channels"], json!([]));
    let (status, _) = dev.get("/api/board/riverside-shop").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!dev.dir.join("data/projects/riverside-shop").exists());

    // Signed in to one is not signed in to the other, though both are Ada.
    let (status, _) = dev.call(Method::GET, "/api/me", Some(&prod.token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(prod.read("/api/me").await["account"]["id"], dev.read("/api/me").await["account"]["id"]);

    // Each lists both, itself among them, and the other at where it answers
    // for a page that is on the device with them.
    let here = format!("127.0.0.1:{}", port(&prod));
    let from_prod = listed(&prod, Some(&here), &[]).await;
    assert_eq!(from_prod["profile"], "prod");
    assert_eq!(one(&from_prod, "prod"), &json!({ "name": "prod", "this": true, "url": null, "why": null }));
    assert_eq!(
        one(&from_prod, "dev1"),
        &json!({ "name": "dev1", "this": false, "url": dev.url, "why": null })
    );
    let from_dev = listed(&dev, Some(&format!("localhost:{}", port(&dev))), &[]).await;
    assert_eq!(
        (one(&from_dev, "dev1")["this"].clone(), one(&from_dev, "prod")["url"].clone()),
        (json!(true), json!(prod.url))
    );

    // The list is names, folders and addresses. Nobody's credential is in it.
    let written = std::fs::read_to_string(shared.registry()).unwrap();
    assert!(written.contains(&dev.node().addr.to_string()) && written.contains("\"prod\""), "{written}");
    assert!(!written.contains(&prod.token) && !written.contains(&dev.token), "{written}");

    // And is not for anyone who has not signed in.
    let (status, _) = prod.call(Method::GET, "/api/profiles", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_browser_on_one_host_is_signed_in_to_each_factory_separately() {
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    let dev = factory(Some("dev1"), &shared.registry()).await;
    let unnamed = Rig::new().await;
    let sign_in = |rig: &Rig| {
        let url = format!("{}/api/auth/login", rig.url);
        async move {
            let body = json!({ "email": "ada@example.com", "password": "correct horse", "page": true });
            let response = reqwest::Client::new().post(url).json(&body).send().await.unwrap();
            let cookie = response.headers()["set-cookie"].to_str().unwrap().to_string();
            // And the key its page keeps, which goes with what the page asks.
            let key = response.json::<Value>().await.unwrap()["page"].as_str().unwrap().to_string();
            (cookie.split(';').next().unwrap().to_string(), key)
        }
    };
    let ((prods, prod_key), (devs, dev_key), (plain, plain_key)) =
        (sign_in(&prod).await, sign_in(&dev).await, sign_in(&unnamed).await);
    let name = |cookie: &str| cookie.split('=').next().unwrap().to_string();
    // A browser keeps cookies by host and not by port: called the same,
    // signing in to one factory would sign the page out of the other.
    assert_ne!(name(&prods), name(&devs));
    assert_eq!(name(&plain), "session", "a factory with no profile calls it what it always did");
    assert!(name(&prods).starts_with("session-"), "{prods}");

    // So a browser sends all three to each of them, and each takes its own.
    let all = format!("{prods}; {devs}; {plain}");
    let signed_in = |rig: &Rig, cookies: &str, key: &str| {
        let request = reqwest::Client::new()
            .get(format!("{}/api/meta", rig.url))
            .header("cookie", cookies)
            .header("x-page-key", key);
        async move { request.send().await.unwrap().json::<Value>().await.unwrap()["signed_in"] == true }
    };
    for (rig, key) in [(&prod, &prod_key), (&dev, &dev_key), (&unnamed, &plain_key)] {
        assert!(signed_in(rig, &all, key).await);
    }
    assert!(
        !signed_in(&dev, &format!("{prods}; {plain}"), &dev_key).await,
        "another factory's session is nobody's here"
    );
    assert!(!signed_in(&unnamed, &format!("{prods}; {devs}"), &plain_key).await);
    // Even one that is handed over under this factory's own name.
    let forged = format!("{}={}", name(&devs), prods.split_once('=').unwrap().1);
    assert!(!signed_in(&dev, &forged, &dev_key).await);
    // And a cookie without the key its page keeps is what every other
    // port of the machine is sent: nobody is signed in by it.
    assert!(!signed_in(&dev, &all, "").await);
}

#[tokio::test]
async fn another_factory_is_listed_with_an_address_only_where_one_can_be_given_honestly() {
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    let dev = factory(Some("dev1"), &shared.registry()).await;
    let own = port(&prod);
    let way = |listed: &Value| (one(listed, "dev1")["url"].clone(), one(listed, "dev1")["why"].clone());
    // The name of what was put in front of it, which its owner told it:
    // asked under one it was not told, it does not answer at all.
    prod.send("/api/system/names", json!({ "name": "factory.example" })).await;

    // Through a proxy on another name: it leads to this factory and no other.
    for (host, more) in [
        ("factory.example:8443", &[][..]),
        ("factory.example", &[]),
        // Even on this factory's own port, when something says it passed the request on.
        (&format!("127.0.0.1:{own}"), &[("x-forwarded-for", "100.64.0.9")]),
        (&format!("127.0.0.1:{own}"), &[("forwarded", "for=100.64.0.9")]),
    ] {
        let (url, why) = way(&listed(&prod, Some(host), more).await);
        assert_eq!(url, Value::Null, "{host}");
        let why = why.as_str().unwrap();
        assert!(why.contains(&format!("This page came through {host}")), "{why}");
        assert!(
            why.contains(&format!("`dev1` answers at {} on the device itself", dev.node().addr)),
            "{why}"
        );
    }

    // Straight to this factory, from another device: the other listens on this device only.
    let (url, why) = way(&listed(&prod, Some(&format!("192.168.1.5:{own}")), &[]).await);
    assert_eq!(url, Value::Null);
    assert!(why.as_str().unwrap().contains("`dev1` listens on the device itself only"), "{why}");
    let (url, _) = way(&listed(&prod, Some(&format!("factory.example:{own}")), &[]).await);
    assert_eq!(url, Value::Null, "a name could be anywhere");

    // Factories that listen elsewhere, written to the list as they would write themselves.
    let mut written: Value =
        serde_json::from_str(&std::fs::read_to_string(shared.registry()).unwrap()).unwrap();
    let data = |name: &str| {
        let dir = shared.0.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    };
    written["factories"].as_array_mut().unwrap().extend([
        json!({ "name": "everywhere", "data": data("everywhere"), "address": "0.0.0.0:7801" }),
        json!({ "name": "one-address", "data": data("one-address"), "address": "192.168.1.5:7802" }),
    ]);
    std::fs::write(shared.registry(), written.to_string()).unwrap();
    let url = |listed: &Value, name: &str| one(listed, name)["url"].clone();

    // One that listens on every address is at whichever the page came to.
    let from_lan = listed(&prod, Some(&format!("192.168.1.5:{own}")), &[]).await;
    assert_eq!(url(&from_lan, "everywhere"), "http://192.168.1.5:7801");
    assert_eq!(url(&from_lan, "one-address"), "http://192.168.1.5:7802", "the page came to that address too");
    let by_name = listed(&prod, Some(&format!("factory.example:{own}")), &[]).await;
    assert_eq!(url(&by_name, "everywhere"), "http://factory.example:7801");
    assert_eq!(url(&by_name, "one-address"), Value::Null, "whether the name is that address is not known");
    let on_device = listed(&prod, Some(&format!("localhost:{own}")), &[]).await;
    assert_eq!(url(&on_device, "everywhere"), "http://localhost:7801");
    assert_eq!(
        url(&on_device, "one-address"),
        "http://192.168.1.5:7802",
        "a device reaches its own addresses"
    );
    let proxied = listed(&prod, Some("factory.example:8443"), &[]).await;
    assert_eq!((url(&proxied, "everywhere"), url(&proxied, "one-address")), (Value::Null, Value::Null));
}

#[tokio::test]
async fn two_factories_may_be_called_the_same_and_are_still_two() {
    let shared = Shared::new();
    let one_of = factory(Some("dev1"), &shared.registry()).await;
    let other = factory(Some("dev1"), &shared.registry()).await;

    let from_one = listed(&one_of, Some(&format!("127.0.0.1:{}", port(&one_of))), &[]).await;
    let entries = from_one["profiles"].as_array().unwrap();
    assert_eq!(entries.len(), 2, "{from_one}");
    assert_eq!(
        entries.iter().filter(|entry| entry["this"] == true).count(),
        1,
        "which is this one is by its data, not its name"
    );
    assert!(entries.iter().any(|entry| entry["url"] == json!(other.url)), "{from_one}");
    // A session of one is not a session of the other, name or no name.
    let (status, _) = other.call(Method::GET, "/api/me", Some(&one_of.token), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_factory_whose_data_is_gone_is_no_longer_listed() {
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    let mut dev = factory(Some("dev1"), &shared.registry()).await;
    assert_eq!(prod.read("/api/profiles").await["profiles"].as_array().unwrap().len(), 2);

    dev.stop().await;
    std::fs::remove_dir_all(dev.dir.join("data")).unwrap();
    let left = prod.read("/api/profiles").await;
    assert_eq!(left["profiles"].as_array().unwrap().len(), 1, "{left}");
}

#[tokio::test]
async fn a_directory_one_profile_holds_is_refused_to_another() {
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    let dev = factory(Some("dev1"), &shared.registry()).await;
    let shop = shared.work("shop");
    let (status, adopted) = adopt(&prod, "Shop", &shop).await;
    assert_eq!(status, StatusCode::OK, "{adopted}");

    let (status, error) = adopt(&dev, "Shop", &shop).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    let said = error["error"]["message"].as_str().unwrap();
    assert!(said.contains(&format!("{} is a project of a `prod` factory", shop.display())), "{said}");
    assert!(said.contains("and this is a `dev1` factory"), "{said}");
    assert!(said.contains("Two profiles never work in the same directory"), "{said}");
    assert!(said.contains("a clone or a copy"), "{said}");
    assert_eq!(dev.read("/api/projects").await["projects"], json!([]));

    // Nor a folder inside it, nor one it is inside: the files are the same files.
    std::fs::create_dir_all(shop.join("site")).unwrap();
    let (status, error) = adopt(&dev, "Site", &shop.join("site")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("shares files with a project of a `prod` factory")
    );
    let (status, _) = adopt(&dev, "Everything", &shared.0).await;
    assert_eq!(status, StatusCode::CONFLICT);
    // Nor a directory reached by another way of writing it.
    #[cfg(unix)]
    {
        let link = shared.0.join("also-shop");
        std::os::unix::fs::symlink(&shop, &link).unwrap();
        let (status, _) = adopt(&dev, "Also Shop", &link).await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    // Nor what the other factory keeps for itself.
    let (status, error) = adopt(&dev, "Theirs", &prod.dir.join("data/projects/theirs")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert!(error["error"]["message"].as_str().unwrap().contains("is among the data of a `prod` factory"));
    assert!(!prod.dir.join("data/projects/theirs").exists(), "and nothing was made there");

    // Nothing was written into the directory to say whose it is, by either.
    let inside: Vec<String> = std::fs::read_dir(&shop)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(inside.len(), 2, "{inside:?}");
    assert!(inside.contains(&"README.md".to_string()) && inside.contains(&"site".to_string()), "{inside:?}");

    // A directory of its own is its own, and what prod lets go of can be taken.
    let (status, _) = adopt(&dev, "Other", &shared.work("other")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = prod.call(Method::DELETE, "/api/projects/shop", Some(&prod.token), None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, adopted) = adopt(&dev, "Shop", &shop).await;
    assert_eq!(status, StatusCode::OK, "{adopted}");
}

#[tokio::test]
async fn what_a_factory_held_before_it_stopped_is_still_held_and_one_with_no_profile_holds_too() {
    let shared = Shared::new();
    // The factory there was before any had a name, with a project it adopted.
    let mut unnamed = factory(None, &shared.registry()).await;
    let shop = shared.work("shop");
    assert_eq!(adopt(&unnamed, "Shop", &shop).await.0, StatusCode::OK);
    unnamed.stop().await;
    // The list is lost, as it would be were it never written: the factory
    // writes what it holds again when it starts.
    std::fs::remove_file(shared.registry()).unwrap();
    unnamed.start().await;

    let dev = factory(Some("dev1"), &shared.registry()).await;
    let (status, error) = adopt(&dev, "Shop", &shop).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    let said = error["error"]["message"].as_str().unwrap();
    assert!(said.contains("is a project of a factory with no profile"), "{said}");

    // Two factories of one profile are that profile's own business.
    let other = factory(Some("dev1"), &shared.registry()).await;
    let theirs = shared.work("theirs");
    assert_eq!(adopt(&dev, "Theirs", &theirs).await.0, StatusCode::OK);
    assert_eq!(adopt(&other, "Theirs", &theirs).await.0, StatusCode::OK);
}

#[tokio::test]
async fn a_factory_with_no_profile_that_adopted_nothing_writes_nothing_and_its_data_is_still_its_own() {
    let shared = Shared::new();
    let unnamed = factory(None, &shared.registry()).await;
    unnamed.send("/api/projects", json!({ "name": "Riverside Shop" })).await;
    // As it always was: nothing of it is written anywhere but in its own data.
    assert!(!shared.registry().exists());

    // Its data is known for a factory's by what is in it, written down or not.
    let dev = factory(Some("dev1"), &shared.registry()).await;
    let (status, error) = adopt(&dev, "Goods", &unnamed.dir.join("data/projects/riverside-shop")).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    let said = error["error"]["message"].as_str().unwrap();
    assert!(said.contains("is among the data of a factory with no profile"), "{said}");
    let written = std::fs::read_to_string(shared.registry()).unwrap();
    assert!(!written.contains(unnamed.dir.to_str().unwrap()), "{written}");

    // A directory that holds a whole factory holds its projects too.
    let (status, error) = adopt(&unnamed, "All of dev", &dev.dir).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert!(
        error["error"]["message"].as_str().unwrap().contains("holds the data of a `dev1` factory"),
        "{error}"
    );
}

#[tokio::test]
async fn a_blueprint_reads_the_profile_and_what_it_runs_is_told_it() {
    const SOURCE: &str = r#"
blueprint: whereabouts
permissions: [shell]
steps:
  loose: { do: noop, when: ${ profile.name != "prod" } }
  told: { do: shell, with: { cmd: SAYS } }
outputs:
  slot: "${ profile.name == 'prod' ? 'careful' : 'builder' }"
  told: "${ trim(steps.told.out.stdout) }"
  loose: ${ !steps.loose.skipped }
"#;
    // What its program is told: the variable, as the program reads it.
    let says = crate::standin::cmd(&["say", "[%FACTORY_PROFILE%]"]);
    let source = SOURCE.replace("SAYS", &says);
    let outputs = async |rig: &Rig| {
        let applied = rig.send("/api/apply", json!({ "source": source, "wait": true })).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        applied["finished"]["outputs"].clone()
    };
    let shared = Shared::new();
    let prod = factory(Some("prod"), &shared.registry()).await;
    assert_eq!(outputs(&prod).await, json!({ "slot": "careful", "told": "[prod]", "loose": false }));
    let dev = factory(Some("dev1"), &shared.registry()).await;
    assert_eq!(outputs(&dev).await, json!({ "slot": "builder", "told": "[dev1]", "loose": true }));
    // With no profile, the name is the empty text, and so is the variable.
    let unnamed = Rig::new().await;
    assert_eq!(outputs(&unnamed).await, json!({ "slot": "builder", "told": "[]", "loose": true }));

    // What is not part of a profile is refused before anything runs.
    // (Its variables are: `crate::variables`.)
    let unknown = source.replace("profile.name != \"prod\"", "profile.region != \"eu\"");
    let (status, error) = prod.post("/api/apply", json!({ "source": unknown, "wait": true })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    assert!(error["error"]["message"].as_str().unwrap().contains("it has `name`, `vars`"), "{error}");
}
