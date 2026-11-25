//! Running under a project's environment: chosen as something is
//! applied, the instance's and its children's for their life, read by
//! its expressions, told to its programs, and the environment's secrets
//! handed in place of the factory's of the same name.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::held::said;

const FACTORY: &str = "sk_factory_value_1";
const PROD: &str = "sk_prod_value_22";

/// `shop`, with the factory's secret `stripe`, and an environment `prod`
/// with a variable, one marked for agents, and its own `stripe`.
async fn shop(rig: &Rig) {
    rig.send("/api/projects", json!({ "name": "shop" })).await;
    let token = Some(rig.token.as_str());
    rig.call(Method::PUT, "/api/secrets/stripe", token, Some(json!({ "value": FACTORY }))).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    let vars = "/api/projects/shop/environments/prod/vars";
    rig.call(Method::PUT, &format!("{vars}/api"), token, Some(json!({ "value": "https://shop.example" })))
        .await;
    rig.call(Method::PUT, &format!("{vars}/region"), token, Some(json!({ "value": "eu", "agents": true })))
        .await;
    let secret = "/api/projects/shop/environments/prod/secrets/stripe";
    let (status, set) = rig.call(Method::PUT, secret, token, Some(json!({ "value": PROD }))).await;
    assert_eq!(status, StatusCode::OK, "{set}");
}

/// A text that reads its environment, is handed `stripe` and is told
/// where it runs, and starts a child that is told the same.
fn deploy() -> String {
    let key = crate::standin::cmd(&["env", "KEY"]);
    let told = crate::standin::cmd(&["env", "FACTORY_ENVIRONMENT"]);
    format!(
        "
blueprint: deploy
permissions: [shell, secrets, blueprints]
steps:
  have: {{ do: shell, env: {{ KEY: {{ secret: stripe }} }}, with: {{ cmd: {key} }} }}
  told: {{ do: shell, with: {{ cmd: {told} }} }}
  child: {{ call: inner }}
outputs:
  key: ${{ steps.have.out.stdout }}
  told: ${{ steps.told.out.stdout }}
  name: ${{ environment.name }}
  vars: ${{ environment.vars }}
on:
  poke:
    event: s.poke
    steps:
      one: {{ do: noop }}
"
    )
}

const INNER: &str = "blueprint: inner\nsteps:\n  one: { do: noop }\non:\n  poke:\n    event: s.poke\n    steps:\n      one: { do: noop }\n";

async fn apply(rig: &Rig, token: &str, body: Value) -> (StatusCode, Value) {
    rig.call(Method::POST, "/api/apply", Some(token), Some(body)).await
}

#[tokio::test]
async fn what_is_applied_under_an_environment_reads_it_is_told_it_and_is_handed_its_secrets() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let saved = json!({ "path": "@blueprints/inner.bp", "content": INNER });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // A project with environments runs everything under one: naming none,
    // or one it has not got, is refused, saying which there are.
    let (status, refused) = apply(&rig, &rig.token, json!({ "source": deploy(), "project": "shop" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("shop runs everything under one of its environments, and this names none: say one of prod (`--environment prod`)"), "{refused}");
    let body = json!({ "source": deploy(), "project": "shop", "environment": "stage" });
    let (status, refused) = apply(&rig, &rig.token, body).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
    let (status, refused) = apply(&rig, &rig.token, json!({ "source": INNER, "environment": "prod" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");

    // Under prod: it reads it, its program is told it and handed prod's
    // `stripe`, written down as that, and its child is under it too.
    let body = json!({ "source": deploy(), "project": "shop", "environment": "prod", "wait": true });
    let (status, applied) = apply(&rig, &rig.token, body).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    let outputs = &applied["finished"]["outputs"];
    assert_eq!(outputs["name"], "prod");
    assert_eq!(outputs["vars"], json!({ "api": "https://shop.example", "region": "eu" }));
    assert!(outputs["told"].as_str().unwrap().contains("FACTORY_ENVIRONMENT=prod"), "{outputs}");
    assert!(
        outputs["key"].as_str().unwrap().contains("KEY=[the secret `stripe` of the `prod` environment]"),
        "{outputs}"
    );
    let text = applied.to_string();
    assert!(!text.contains(PROD) && !text.contains(FACTORY), "{text}");
    let instances = rig.read("/api/instances?project=shop&live=true").await;
    let under: Vec<(&str, &str)> = instances["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["key"].as_str().unwrap(), one["environment"].as_str().unwrap()))
        .collect();
    assert!(under.contains(&("deploy", "prod")) && under.contains(&("child", "prod")), "{under:?}");

    // Applied again naming another, it is refused; naming none, it goes by its own.
    rig.send("/api/projects/shop/environments", json!({ "name": "stage" })).await;
    let body = json!({ "source": deploy(), "project": "shop", "environment": "stage" });
    let (status, refused) = apply(&rig, &rig.token, body).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("`deploy` runs under prod, and this asks for stage"), "{refused}");
    let (status, again) =
        apply(&rig, &rig.token, json!({ "source": deploy(), "project": "shop", "force": true })).await;
    assert_eq!(status, StatusCode::OK, "{again}");

    // A value changed is read the next time it is worked out, and nothing runs again.
    let vars = "/api/projects/shop/environments/prod/vars/api";
    rig.call(Method::PUT, vars, Some(&rig.token), Some(json!({ "value": "https://new.example" }))).await;
    let body =
        json!({ "source": deploy(), "project": "shop", "environment": "prod", "key": "again", "wait": true });
    let (_, later) = apply(&rig, &rig.token, body).await;
    assert_eq!(later["finished"]["outputs"]["vars"]["api"], "https://new.example", "{later}");

    // Not taken away while anything runs under it, and where it is named said.
    let named = rig.read("/api/projects/shop/environments/prod/secrets").await;
    let instances = &named["secrets"][0]["named"]["instances"];
    assert!(
        instances.to_string().contains("\"deploy\"") && instances.to_string().contains("\"again\""),
        "{named}"
    );
    let factory = rig.read("/api/secrets?named=true").await;
    assert_eq!(
        factory["secrets"][0]["named"]["instances"],
        json!([]),
        "prod's own is not the factory's: {factory}"
    );
    let (status, refused) =
        rig.call(Method::DELETE, "/api/projects/shop/environments/prod", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert!(said(&refused).contains("run under the environment `prod`"), "{refused}");
}

#[tokio::test]
async fn what_an_agent_applies_under_an_environment_reads_and_is_handed_only_what_is_marked_for_agents() {
    let rig = Rig::new().await;
    shop(&rig).await;
    let scopes = ["projects", "blueprints", "shell", "secrets:use"];
    let invited =
        rig.send("/api/agent/invite", json!({ "name": "scout", "project": "shop", "scopes": scopes })).await;
    let (status, joined) = rig
        .call(Method::POST, "/api/agent/join", None, Some(json!({ "invitation": invited["invitation"] })))
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    let scout = joined["token"].as_str().unwrap().to_string();
    let reads = "blueprint: reads\nsteps:\n  one: { do: noop }\noutputs:\n  vars: ${ environment.vars }\n";
    let body = json!({ "source": reads, "project": "shop", "environment": "prod", "wait": true });
    let (status, applied) = apply(&rig, &scout, body).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["finished"]["outputs"]["vars"], json!({ "region": "eu" }), "{applied}");

    // Its program is not handed prod's `stripe` until it is marked, nor the factory's in its place.
    let source = deploy().replace("  child: { call: inner }\n", "");
    let body =
        json!({ "source": source, "project": "shop", "environment": "prod", "key": "handed", "wait": true });
    let (_, refused) = apply(&rig, &scout, body.clone()).await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(
        refused["finished"]["error"]
            .as_str()
            .unwrap()
            .contains("does not hand its secret `stripe` to what an agent applied"),
        "{refused}"
    );
    let mark = "/api/projects/shop/environments/prod/secrets/stripe/agents";
    rig.call(Method::PUT, mark, Some(&rig.token), Some(json!({ "agents": "STRIPE_KEY" }))).await;
    let body =
        json!({ "source": source, "project": "shop", "environment": "prod", "key": "marked", "wait": true });
    let (_, handed) = apply(&rig, &scout, body).await;
    assert_eq!(handed["finished"]["status"], "ok", "{handed}");
    assert!(
        handed["finished"]["outputs"]["key"].as_str().unwrap().contains("of the `prod` environment"),
        "{handed}"
    );
}

/// An apply under `environment` of `shop`, asked of the runner itself.
fn under(
    rig: &Rig,
    environment: &str,
    key: &str,
) -> tokio::task::JoinHandle<kernel::Result<conductor::Applied>> {
    let engine = rig.node().app.engine.clone();
    let (environment, key, identity) =
        (environment.to_string(), key.to_string(), rig.node().app.studio.identity.clone());
    let token = rig.token.clone();
    tokio::spawn(async move {
        let who = identity.authenticate(&token).await?;
        let source =
            "blueprint: stays\nsteps:\n  one: { do: noop }\non:\n  poke: { event: s.poke, do: noop }\n";
        let request = conductor::Apply {
            source: Some(source.to_string()),
            key: Some(key),
            project: Some("shop".into()),
            environment: Some(environment),
            ..conductor::Apply::new("stays", who)
        };
        engine.apply(request).await
    })
}

/// A taking away of `environment` of `shop`, asked of the runner itself.
fn taken_away(rig: &Rig, environment: &str) -> tokio::task::JoinHandle<kernel::Result<String>> {
    let engine = rig.node().app.engine.clone();
    let environment = environment.to_string();
    tokio::spawn(async move {
        let by = kernel::Principal::system();
        let source = kernel::Source::actor("system");
        let agents = async { Ok(Vec::new()) };
        engine.remove_environment(("shop", &environment), agents, &by, source).await.map(|was| was.name)
    })
}

#[tokio::test]
async fn nothing_is_applied_under_an_environment_between_the_look_that_nothing_runs_under_it_and_its_taking_away()
 {
    let rig = Rig::new().await;
    shop(&rig).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "stage" })).await;
    let engine = rig.node().app.engine.clone();
    let held = engine.environments();

    // Somebody has the turn, so what asks next waits behind them in the
    // order it asked: an apply under prod, and then prod taken away. The
    // apply's row is written before the look, which finds it there.
    let turn = held.changing().await;
    let apply = under(&rig, "prod", "first");
    crate::soon("the first asker to wait for the turn", held.waiting_for(1)).await;
    let removal = taken_away(&rig, "prod");
    crate::soon("the second asker to wait behind it", held.waiting_for(2)).await;
    drop(turn);
    let applied = apply.await.unwrap().expect("applied under prod");
    let refused = removal.await.unwrap().unwrap_err();
    assert!(refused.message.contains("`first` runs under the environment `prod`"), "{}", refused.message);
    let info = engine.instance(&applied.instance).await.unwrap();
    assert_eq!(info.environment, "prod");

    // The other way round: stage taken away first, and then an apply under
    // it, which finds it gone and makes nothing.
    let turn = held.changing().await;
    let removal = taken_away(&rig, "stage");
    crate::soon("the first asker to wait for the turn", held.waiting_for(1)).await;
    let apply = under(&rig, "stage", "second");
    crate::soon("the second asker to wait behind it", held.waiting_for(2)).await;
    drop(turn);
    assert_eq!(removal.await.unwrap().expect("taken away"), "stage");
    let refused = apply.await.unwrap().unwrap_err();
    assert!(refused.message.contains("has no environment called `stage`"), "{}", refused.message);
    let live = rig.read("/api/instances?project=shop&live=true").await;
    assert!(!live.to_string().contains("\"second\""), "{live}");
}

#[tokio::test]
async fn what_a_project_starts_with_is_given_it_under_no_environment_though_it_has_some() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // As the client's switch gives them: under no environment, since they
    // look after the project's tables, which are its whatever it deploys to.
    let given =
        rig.send("/api/projects/shop/starts", json!({ "names": ["board", "fleet", "features"] })).await;
    assert!(given["left"].is_null(), "{given}");
    for start in ["board", "fleet", "features"] {
        let instance = given[start].as_str().unwrap_or_else(|| panic!("{start}: {given}"));
        let info = rig.read(&format!("/api/instances/{instance}")).await;
        assert_eq!(info["instance"]["environment"], "", "{start}: {info}");
    }
    // Saying none on purpose is the daemon's own, and no request's.
    let body = json!({ "source": INNER, "project": "shop", "environment": "" });
    let (status, refused) = apply(&rig, &rig.token, body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("say one of prod"), "{refused}");
}

#[tokio::test]
async fn an_agent_works_under_the_environment_of_what_brings_it_up_or_the_one_said_by_hand() {
    let rig = Rig::new().await;
    shop(&rig).await;
    // Brought up by something applied under prod, it works under prod.
    let brings = "blueprint: brings\npermissions: [agents]\nsteps:\n  up: { do: agent.ensure, with: { name: scout } }\n";
    let body = json!({ "source": brings, "project": "shop", "environment": "prod", "wait": true });
    let (status, applied) = apply(&rig, &rig.token, body).await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    let agents = rig.read("/api/agents").await;
    let scout = agents["agents"].as_array().unwrap().iter().find(|one| one["name"] == "scout").unwrap();
    assert_eq!(scout["environment"], "prod", "{agents}");

    // Added by hand, it says which, or is refused naming them.
    let (status, refused) = rig.post("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(said(&refused).contains("say one of prod"), "{refused}");
    let body = json!({ "name": "helper", "project": "shop", "environment": "prod", "secrets": ["STRIPE_KEY=stripe"] });
    let made = rig.send("/api/agents", body).await;
    assert_eq!(made["environment"], "prod", "{made}");

    // Where each of prod's secrets is named says the agents that work under it.
    let named = rig.read("/api/projects/shop/environments/prod/secrets").await;
    let stripe = named["secrets"].as_array().unwrap().iter().find(|one| one["name"] == "stripe").unwrap();
    assert_eq!(stripe["named"]["agents"], json!([{ "name": "helper", "variable": "STRIPE_KEY" }]), "{named}");

    // Nor is it taken away while an agent works under it.
    let would = rig.read("/api/projects/shop/environments/prod/remove").await;
    let running = would["running"].to_string();
    assert!(running.contains("the agent `helper`") && running.contains("the agent `scout`"), "{would}");
    let (status, refused) =
        rig.call(Method::DELETE, "/api/projects/shop/environments/prod", Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
}
