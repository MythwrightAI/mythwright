//! A project's environments and their variables, read and changed through
//! the daemon: what each change says, who may, and what an agent is shown.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::{key, said};

/// Changes a variable of `shop`'s `environment` with `token`, as `PUT` or `DELETE`.
async fn var(
    rig: &Rig,
    token: &str,
    method: Method,
    environment: &str,
    var: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let path = format!("/api/projects/shop/environments/{environment}/vars/{var}");
    rig.call(method, &path, Some(token), body).await
}

#[tokio::test]
async fn a_project_says_its_environments_and_their_variables_and_each_change_says_names_and_no_values() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    let base = "/api/projects/shop/environments";
    assert_eq!(rig.read(base).await["environments"], json!([]), "a project has none until it says one");

    // Made, by a name that reads; one that does not, and one there already, refused.
    let made = rig.send(base, json!({ "name": "staging" })).await;
    assert_eq!((&made["name"], &made["vars"]), (&json!("staging"), &json!({})));
    let (status, refused) = rig.post(base, json!({ "name": "Staging Two" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("lowercase letters, digits and `-`"), "{refused}");
    let (status, refused) = rig.post(base, json!({ "name": "staging" })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    let (status, refused) = rig.post("/api/projects/nowhere/environments", json!({ "name": "dev" })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");

    // Variables are set one by one, each plain, and the rest are left.
    let owner = rig.token.clone();
    let (status, set) =
        var(&rig, &owner, Method::PUT, "staging", "api", Some(json!({ "value": "https://staging.example" })))
            .await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!(set["changed"]["set"], json!(["api"]));
    var(&rig, &owner, Method::PUT, "staging", "replicas", Some(json!({ "value": 3, "agents": true }))).await;
    let (status, refused) =
        var(&rig, &owner, Method::PUT, "staging", "hosts", Some(json!({ "value": ["a", "b"] }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("is a list, and a variable is plain"), "{refused}");
    let (status, refused) =
        var(&rig, &owner, Method::PUT, "staging", "PATH", Some(json!({ "value": "/x", "agents": true })))
            .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("cannot be handed to an agent"), "{refused}");
    let staging = rig.read(&format!("{base}/staging")).await;
    assert_eq!(
        staging["vars"],
        json!({
            "api": { "value": "https://staging.example", "agents": false },
            "replicas": { "value": 3, "agents": true },
        })
    );

    // Each change is said by name, and no value is ever in what is said.
    let changed =
        rig.event("project.environment.changed", |event| event.data["set"] == json!(["replicas"])).await;
    assert_eq!(changed.data["project"], "shop");
    assert_eq!(changed.data["marked"], json!([]));
    let made = rig.event("project.environment.made", |_| true).await;
    assert_eq!((&made.data["environment"], &made.data["project"]), (&json!("staging"), &json!("shop")));
    let query = journal::EventQuery { topics: "project.environment.*".into(), ..Default::default() };
    for event in rig.said(query).await {
        let text = event.data.to_string();
        assert!(!text.contains("staging.example") && !text.contains(":3"), "{text}");
    }

    // A mark changed alone, and the same asked again changes nothing and says nothing.
    let (_, marked) = var(&rig, &owner, Method::PUT, "staging", "api", Some(json!({ "agents": true }))).await;
    assert_eq!(marked["changed"]["marked"], json!(["api"]));
    let said_before = rig.count_of("project.environment.changed").await;
    let (_, again) = var(&rig, &owner, Method::PUT, "staging", "api", Some(json!({ "agents": true }))).await;
    assert_eq!(again["changed"]["marked"], json!([]));
    assert_eq!(rig.count_of("project.environment.changed").await, said_before);
    var(&rig, &owner, Method::PUT, "staging", "api", Some(json!({ "agents": false }))).await;

    // Reading takes reading the project; changing takes writing it, with
    // `shell` and `secrets:use`, each said where it is lacking.
    let reader = key(&rig, &["projects:read"]).await;
    let (status, read) = rig.call(Method::GET, base, Some(&reader), None).await;
    assert_eq!(status, StatusCode::OK, "{read}");
    let writer = key(&rig, &["projects"]).await;
    let (status, refused) =
        var(&rig, &writer, Method::PUT, "staging", "api", Some(json!({ "value": "elsewhere" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("`shell`"), "{refused}");
    let (status, refused) = rig.call(Method::DELETE, &format!("{base}/staging"), Some(&reader), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let nobody = key(&rig, &["channels:read"]).await;
    let (status, refused) = rig.call(Method::GET, base, Some(&nobody), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // An agent is shown the variables marked for agents, and nothing of the rest.
    let invited = rig.send("/api/agent/invite", json!({ "name": "scout", "project": "shop" })).await;
    let (status, joined) = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invited["invitation"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    let scout = joined["token"].as_str().unwrap().to_string();
    let (status, shown) = rig.call(Method::GET, &format!("{base}/staging"), Some(&scout), None).await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    assert_eq!(shown["vars"], json!({ "replicas": { "value": 3, "agents": true } }));
    assert!(!shown.to_string().contains("staging.example"), "{shown}");

    // Unset, and taken away with what it had, said first and then done.
    let (status, unset) = var(&rig, &owner, Method::DELETE, "staging", "replicas", None).await;
    assert_eq!((status, &unset["changed"]["removed"]), (StatusCode::OK, &json!(["replicas"])));
    let (status, refused) = var(&rig, &owner, Method::DELETE, "staging", "replicas", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    assert!(said(&refused).contains("has no variable `replicas`; it has `api`"), "{refused}");
    let would = rig.read(&format!("{base}/staging/remove")).await;
    assert!(would["said"].as_str().unwrap().contains("its one variable goes with it"), "{would}");
    let (status, removed) = rig.call(Method::DELETE, &format!("{base}/staging"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    assert_eq!(rig.read(base).await["environments"], json!([]));
    let gone = rig.event("project.environment.removed", |_| true).await;
    assert_eq!((&gone.data["environment"], &gone.data["vars"]), (&json!("staging"), &json!(1)));
    let (status, _) = rig.get(&format!("{base}/staging")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_projects_environments_are_kept_in_its_own_journal_and_hold_to_the_lists_of_what_goes_where() {
    if crate::one_journal() {
        return;
    }
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    let journal = rig.node().app.journal.clone();
    let own = journal.of(Some("shop"));
    let kept = own.kv_get(runs::beside::PROJECT_ENVIRONMENTS, "shop\nprod").await.unwrap();
    assert_eq!(kept.unwrap()["name"], "prod", "kept where the project's own conductor keeps what is its");
    assert_eq!(journal.kv_get(runs::beside::PROJECT_ENVIRONMENTS, "shop\nprod").await.unwrap(), None);
    crate::apart::held_to_the_lists(&journal).await;
}

/// Changes a secret of `shop`'s `prod` with `token`.
async fn secret(
    rig: &Rig,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let path = format!("/api/projects/shop/environments/prod/secrets/{path}");
    rig.call(method, &path, Some(token), body).await
}

const VALUE: &str = "sk_live_kept_apart";

#[tokio::test]
async fn an_environments_secret_is_set_marked_and_taken_away_by_whoever_holds_the_scope_and_never_shown() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    let base = "/api/projects/shop/environments";
    rig.send(base, json!({ "name": "prod" })).await;
    let owner = rig.token.clone();

    // Whoever does not hold the scope is refused before anything sent is read,
    // whatever else they hold.
    let writer = key(&rig, &["projects", "shell", "secrets:use", "admin"]).await;
    let (status, refused) =
        secret(&rig, &writer, Method::PUT, "stripe", Some(json!({ "value": VALUE }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("`environments:secrets`"), "{refused}");
    let (status, refused) =
        secret(&rig, &owner, Method::PUT, "Stripe Key", Some(json!({ "value": VALUE }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, refused) = rig
        .call(
            Method::PUT,
            "/api/projects/shop/environments/dev/secrets/stripe",
            Some(&owner),
            Some(json!({ "value": VALUE })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");

    // The owner holds it without being given it; what is answered and said is the name.
    let body = json!({ "value": VALUE, "agents": "STRIPE_KEY" });
    let (status, set) = secret(&rig, &owner, Method::PUT, "stripe", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{set}");
    assert_eq!(
        (&set["changed"]["secrets"], &set["changed"]["secrets_marked"]),
        (&json!(["stripe"]), &json!(["stripe"]))
    );
    secret(&rig, &owner, Method::PUT, "hook", Some(json!({ "value": "whsec_other" }))).await;
    let shown = rig.read(&format!("{base}/prod")).await;
    assert_eq!(shown["secrets"]["stripe"]["kept"], true, "{shown}");
    assert_eq!(shown["secrets"]["stripe"]["agents"], "STRIPE_KEY", "{shown}");
    let said_of =
        rig.event("project.environment.changed", |event| event.data["secrets"] == json!(["hook"])).await;
    assert_eq!(said_of.data["project"], "shop");
    let query = journal::EventQuery { topics: "project.environment.*".into(), ..Default::default() };
    for event in rig.said(query).await {
        let text = event.data.to_string();
        assert!(!text.contains(VALUE) && !text.contains("whsec_other"), "{text}");
    }
    for listed in
        [rig.read(base).await, rig.read(&format!("{base}/prod")).await, rig.read("/api/secrets").await]
    {
        assert!(!listed.to_string().contains(VALUE), "{listed}");
    }
    assert!(!rig.read("/api/secrets").await.to_string().contains("stripe"), "not one of the factory's");
    let file =
        rig.dir.join("data").join("secrets").join("@projects").join("shop").join("prod").join("stripe");
    assert!(!String::from_utf8_lossy(&std::fs::read(&file).unwrap()).contains(VALUE), "sealed");

    // One name an agent's program is handed one thing by.
    let (status, refused) =
        var(&rig, &owner, Method::PUT, "prod", "STRIPE_KEY", Some(json!({ "value": "x", "agents": true })))
            .await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("the secret `stripe` as `STRIPE_KEY` already"), "{refused}");
    let (status, refused) =
        secret(&rig, &owner, Method::PUT, "hook/agents", Some(json!({ "agents": "STRIPE_KEY" }))).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");

    // Its names are for whoever may name secrets, and an agent's for what it is handed.
    let reader = key(&rig, &["projects:read"]).await;
    let (_, read) = rig.call(Method::GET, &format!("{base}/prod"), Some(&reader), None).await;
    assert!(read.get("secrets").is_none(), "{read}");
    let invited = rig.send("/api/agent/invite", json!({ "name": "scout", "project": "shop" })).await;
    let (_, joined) = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invited["invitation"] })))
        .await;
    let scout = joined["token"].as_str().unwrap().to_string();
    let (_, shown) = rig.call(Method::GET, &format!("{base}/prod"), Some(&scout), None).await;
    let names: Vec<&String> = shown["secrets"].as_object().unwrap().keys().collect();
    assert_eq!(names, ["stripe"], "{shown}");
    let (status, refused) = secret(&rig, &scout, Method::DELETE, "stripe", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");

    // Taken away off the list and then its value; an environment that lists
    // any takes the scope to take away, and its values go with it.
    let (status, gone) = secret(&rig, &owner, Method::DELETE, "stripe", None).await;
    assert_eq!((status, &gone["changed"]["secrets_removed"]), (StatusCode::OK, &json!(["stripe"])));
    assert!(!file.exists());
    let (status, refused) = secret(&rig, &owner, Method::DELETE, "stripe", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let changer = key(&rig, &["projects", "shell", "secrets:use"]).await;
    let (status, refused) = rig.call(Method::DELETE, &format!("{base}/prod"), Some(&changer), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let would = rig.read(&format!("{base}/prod/remove")).await;
    assert!(would["said"].as_str().unwrap().contains("its one secret goes with it"), "{would}");
    let (status, removed) = rig.call(Method::DELETE, &format!("{base}/prod"), Some(&owner), None).await;
    assert_eq!(status, StatusCode::OK, "{removed}");
    let gone = rig.event("project.environment.removed", |_| true).await;
    assert_eq!(gone.data["secrets"], 1);
    assert!(!file.parent().unwrap().exists(), "its folder went with it");
}

const REACH: &str = "extension: reach\ntitle: Reach\nblueprints:\n  round: |\n    blueprint: reach/round\n    \
                     version: 1\n    steps:\n      made: { do: noop }\n";

/// Whatever is kept by `shop`'s name beside the runner's rows, in every journal.
async fn kept_of_shop(rig: &Rig) -> Vec<String> {
    let mut kept = Vec::new();
    for journal in rig.node().app.journal.every() {
        for name in [runs::beside::PROJECT_ENVIRONMENTS, runs::beside::FORKED] {
            for (key, _) in journal.kv_list(name).await.unwrap() {
                if runs::beside::project_of(&key) == Some("shop") {
                    kept.push(format!("{name} {key}"));
                }
            }
        }
    }
    kept
}

#[tokio::test]
async fn a_project_deleted_for_good_takes_its_environments_their_secrets_and_its_forks_with_it() {
    let rig = Rig::new().await;
    rig.send("/api/extensions", json!({ "source": REACH })).await;
    let folder = rig.dir.join("data").join("secrets").join("@projects").join("shop");
    let files = rig.dir.join("data").join("projects").join("shop");
    let fork = rig.placed("shop").join("reach").join("round.bp");
    // From the Trash, and where the factory keeps nothing that is deleted.
    for days in [json!(30), json!(0)] {
        rig.send("/api/system/trash", json!({ "days": days })).await;
        rig.send("/api/projects", json!({ "name": "shop" })).await;
        rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
        var(&rig, &rig.token, Method::PUT, "prod", "api", Some(json!({ "value": "https://shop.example" })))
            .await;
        secret(&rig, &rig.token, Method::PUT, "stripe", Some(json!({ "value": VALUE }))).await;
        rig.send("/api/projects/shop/own", json!({ "blueprint": "reach/round" })).await;
        assert_eq!(kept_of_shop(&rig).await.len(), 2, "{days}");
        assert!(folder.is_dir());

        let (status, forgot) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{forgot}");
        if let Some(id) = forgot["trash"].as_str() {
            // Kept while it is in the Trash, so that it is given back whole.
            assert_eq!(kept_of_shop(&rig).await.len(), 2);
            assert!(folder.is_dir());
            let (status, gone) =
                rig.call(Method::DELETE, &format!("/api/trash/{id}"), Some(&rig.token), None).await;
            assert_eq!(status, StatusCode::OK, "{gone}");
        }
        assert_eq!(kept_of_shop(&rig).await, Vec::<String>::new(), "{days}");
        assert!(!folder.exists(), "{days}");
        // The fork's file was kept by the factory for the project, and goes
        // with it, as what it was forked from does; its own files stay where
        // they are.
        assert!(!fork.exists() && files.is_dir(), "{days}");
        // Taken away, as its owner would, so that it is a project made anew.
        std::fs::remove_dir_all(&files).unwrap();

        // A project added again under its name is given none of it.
        rig.send("/api/projects", json!({ "name": "shop" })).await;
        assert_eq!(rig.read("/api/projects/shop/environments").await["environments"], json!([]));
        assert!(rig.node().app.engine.forked_from("shop", "reach/round").await.is_none());
        let (status, forgot) = rig.call(Method::DELETE, "/api/projects/shop", Some(&rig.token), None).await;
        assert_eq!(status, StatusCode::OK, "{forgot}");
        if let Some(id) = forgot["trash"].as_str() {
            rig.call(Method::DELETE, &format!("/api/trash/{id}"), Some(&rig.token), None).await;
        }
        std::fs::remove_dir_all(&files).unwrap();
    }
}
