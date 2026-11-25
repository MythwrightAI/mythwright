//! What a project's pipelines are looked for by: the instances of one
//! blueprint (`blueprint`, what `pipelines/<name>` ran as) and those under
//! one environment of their project's (`environment`), beside `project`.
//! They narrow a list and widen nothing: whoever may read what runs is
//! answered as anybody is, and whoever may not is refused as without them.

use reqwest::{Method, StatusCode};
use serde_json::json;

use crate::Rig;
use crate::apart::ids;

/// A pipeline of one step, called `name`.
fn pipeline(name: &str) -> String {
    format!("blueprint: {name}\nsteps:\n  say: {{ do: log, with: {{ message: ran }} }}\n")
}

#[tokio::test]
async fn a_projects_pipeline_runs_are_found_by_blueprint_and_environment_and_for_nobody_who_may_not_read_runs()
 {
    let rig = Rig::new().await;
    for name in ["Shop", "Yard"] {
        rig.send("/api/projects", json!({ "name": name })).await;
    }
    for environment in ["dev", "prod"] {
        rig.send("/api/projects/shop/environments", json!({ "name": environment })).await;
    }
    let apply =
        |project: &'static str, name: &'static str, environment: Option<&'static str>, key: &'static str| {
            let rig = &rig;
            async move {
                let mut asked =
                    json!({ "source": pipeline(name), "project": project, "key": key, "wait": true });
                if let Some(environment) = environment {
                    asked["environment"] = json!(environment);
                }
                let applied = rig.send("/api/apply", asked).await;
                assert_eq!(applied["finished"]["status"], "ok", "{applied}");
                applied["instance"].as_str().unwrap().to_string()
            }
        };
    let check_dev = apply("shop", "pipelines/check", Some("dev"), "check-dev").await;
    let check_prod = apply("shop", "pipelines/check", Some("prod"), "check-prod").await;
    let deploy_prod = apply("shop", "pipelines/deploy", Some("prod"), "deploy-prod").await;
    let yards = apply("yard", "pipelines/check", None, "check").await;

    let of = |token: String, query: &'static str| {
        let rig = &rig;
        async move {
            let (status, listed) =
                rig.call(Method::GET, &format!("/api/instances?{query}"), Some(&token), None).await;
            assert_eq!(status, StatusCode::OK, "{query}: {listed}");
            let mut found = ids(&listed);
            found.sort();
            found
        }
    };
    let sorted = |mut all: Vec<String>| {
        all.sort();
        all
    };
    let owner = rig.token.clone();
    // A key that may read what runs and nothing more is answered as the owner is: the filters narrow and widen nothing.
    let member = crate::told::key(&rig, &["blueprints:read"]).await;
    for token in [owner, member] {
        assert_eq!(
            of(token.clone(), "project=shop&blueprint=pipelines/check").await,
            sorted(vec![check_dev.clone(), check_prod.clone()])
        );
        assert_eq!(
            of(token.clone(), "blueprint=pipelines/check").await,
            sorted(vec![check_dev.clone(), check_prod.clone(), yards.clone()])
        );
        assert_eq!(
            of(token.clone(), "project=shop&environment=prod").await,
            sorted(vec![check_prod.clone(), deploy_prod.clone()])
        );
        assert_eq!(
            of(token.clone(), "project=shop&blueprint=pipelines/deploy&environment=prod").await,
            vec![deploy_prod.clone()]
        );
        assert_eq!(
            of(token.clone(), "project=shop&blueprint=pipelines/deploy&environment=dev").await,
            Vec::<String>::new()
        );
        assert_eq!(
            of(token.clone(), "project=shop&project=yard&blueprint=pipelines/check&environment=dev").await,
            vec![check_dev.clone()]
        );
        // What is under none is said as the empty name, which no environment is called.
        assert_eq!(of(token.clone(), "project=yard&environment=").await, vec![yards.clone()]);
        assert_eq!(of(token.clone(), "blueprint=pipelines/nowhere").await, Vec::<String>::new());
    }
    // Whoever may not read what runs is refused, whatever is asked for.
    let reader = crate::told::key(&rig, &["channels:read"]).await;
    for path in ["/api/instances?project=shop&blueprint=pipelines/check", "/api/instances?environment=prod"] {
        let (status, answer) = rig.call(Method::GET, path, Some(&reader), None).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}: {answer}");
    }
}

/// A step that says `lines` lines, one, two and so on.
fn saying(lines: usize) -> String {
    let words: Vec<String> = (1..=lines).flat_map(|at| ["say".to_string(), at.to_string()]).collect();
    let cmd = crate::standin::cmd(&words.iter().map(String::as_str).collect::<Vec<_>>());
    format!("blueprint: pipelines/long\nsteps:\n  say: {{ do: shell, with: {{ cmd: {cmd} }} }}\n")
}

#[tokio::test]
async fn a_steps_output_is_read_from_its_end_and_what_came_before_it_by_where_that_begins() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied =
        rig.send("/api/apply", json!({ "source": saying(30), "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let run = applied["run"].as_str().unwrap().to_string();
    let read = |token: String, query: &'static str| {
        let (rig, run) = (&rig, run.clone());
        async move {
            rig.call(Method::GET, &format!("/api/runs/{run}/steps/say/log?{query}"), Some(&token), None).await
        }
    };
    let said = |lines: &serde_json::Value| -> Vec<(u64, String)> {
        let all = lines["lines"].as_array().unwrap_or_else(|| panic!("lines: {lines}"));
        all.iter()
            .map(|one| (one["line"].as_u64().unwrap(), one["text"].as_str().unwrap().to_string()))
            .collect()
    };
    let numbered =
        |from: u64, to: u64| -> Vec<(u64, String)> { (from..=to).map(|at| (at, at.to_string())).collect() };
    let member = crate::told::key(&rig, &["blueprints:read"]).await;
    for token in [rig.token.clone(), member] {
        // The last five, each with where it is in the whole.
        let (status, end) = read(token.clone(), "tail=5").await;
        assert_eq!(status, StatusCode::OK, "{end}");
        assert_eq!(said(&end), numbered(26, 30));
        // What came before them, by where it begins and how many.
        let (_, before) = read(token.clone(), "after=20&limit=5").await;
        assert_eq!(said(&before), numbered(21, 25));
        // More than there are is all of it.
        let (_, all) = read(token.clone(), "tail=500").await;
        assert_eq!(said(&all), numbered(1, 30));
    }
    // Whoever may not read what runs reads none of it, from its end or its beginning.
    let reader = crate::told::key(&rig, &["channels:read"]).await;
    for query in ["tail=5", "after=0&limit=5"] {
        let (status, answer) = read(reader.clone(), query).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{query}: {answer}");
    }
}
