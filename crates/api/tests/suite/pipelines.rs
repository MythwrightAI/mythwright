//! A project's pipelines run as the rows of its `triggers` say
//! (`std/pipeline`): by hand, as a card comes to a column, as a branch
//! moves, so long after the last run and at a time of day; one asked for
//! under a key another runs under waits for that one; somebody who asks
//! by hand runs one only where they hold what it runs with; and a row
//! runs its pipeline under the environment it says, held there to that
//! environment's rules.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::{git, move_card};

/// A pipeline of the project's own: it says that it ran, with what it was
/// handed, and where it is told to it waits to be let go first.
pub(crate) const GATES: &str = r#"blueprint: pipelines/gates
description: Say that it ran, with what it was handed.
permissions: [projects:read]

params:
  note: { type: string, default: "" }
  item: { type: string, default: "" }
  branch: { type: string, default: "" }
  commit: { type: string, default: "" }
  hold: { type: bool, default: false }

steps:
  held:
    when: ${params.hold}
    do: wait
    with: { event: gates.go }

  ran:
    needs: [held]
    do: emit
    with: { topic: gates.ran, note: "${params.note}", item: "${params.item}", branch: "${params.branch}", commit: "${params.commit}" }
"#;

/// A pipeline of the project's own that cannot be run without being told
/// where to: it says that it ran, and where.
pub(crate) const DEPLOY: &str = r#"blueprint: pipelines/deploy
description: Say that it ran, and where to.
permissions: [projects:read]

params:
  target: { type: string }
  times: { type: int }

steps:
  ran:
    do: emit
    with: { topic: deploy.ran, target: "${params.target}", times: ${params.times} }
"#;

/// A shop with a board, its own pipeline `pipelines/gates`, and what runs
/// its pipelines applied for it.
pub(crate) async fn shop(rig: &Rig, path: Option<&PathBuf>) {
    let mut asked = json!({ "name": "Shop" });
    if let Some(path) = path {
        asked["path"] = json!(path);
    }
    rig.send("/api/projects", asked).await;
    rig.board("shop").await;
    let saved = json!({ "path": format!("@blueprints/pipelines/{}", brand::blueprint_file("gates")), "content": GATES });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let applied = json!({ "blueprint": "std/pipeline", "project": "shop", "params": { "project": "shop" }, "wait": true });
    rig.send("/api/apply", applied).await;
}

/// Adds a row of the project's triggers: what it runs by is read as
/// each thing happens, so nothing is waited for.
pub(crate) async fn row(rig: &Rig, fields: Value) -> String {
    let added = rig.send("/api/collections/shop/triggers/records", json!({ "fields": fields })).await;
    added["id"].as_str().unwrap().to_string()
}

/// Waits for what waits for a row of `every` to have begun its wait,
/// which it says (`step.waiting`, a wait for so long, which waits for
/// no event and nothing to end): from there, the clock moved is moved
/// past it.
async fn waiting(rig: &Rig, mark: u64) {
    let so_long = |event: &kernel::Event| event.data["event"].is_null() && event.data["ended"].is_null();
    rig.event("step.waiting", |event| event.seq > mark && so_long(event)).await;
}

/// What the pipeline was handed the next time it says it ran after `mark`.
async fn ran(rig: &Rig, mark: u64) -> Value {
    rig.event("gates.ran", |event| event.seq > mark).await.data.clone()
}

#[tokio::test]
async fn a_pipeline_runs_by_hand_for_somebody_who_holds_what_it_runs_with_and_not_for_somebody_who_does_not()
{
    let rig = Rig::new().await;
    shop(&rig, None).await;
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "hand" })).await;

    let mark = rig.mark();
    let asked = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "note": "by Ada" } });
    rig.send("/api/emit/pipeline.asked", asked.clone()).await;
    assert_eq!(ran(&rig, mark).await["note"], "by Ada");

    // A key that may say it is asked and holds nothing the pipeline runs with.
    let narrow = crate::held::key(&rig, &["emit:pipeline.asked"]).await;
    let mark = rig.mark();
    let (status, answer) =
        rig.call(Method::POST, "/api/emit/pipeline.asked", Some(&narrow), Some(asked)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let refused = rig.event("pipeline.refused", |event| event.seq > mark).await;
    let lacks: Vec<&str> =
        refused.data["lacks"].as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_eq!(lacks, ["blueprints:run", "projects:read"], "{}", refused.data);
    assert_eq!(rig.count_of("gates.ran").await, 1);

    // Asked for a pipeline no row runs by hand, it says what to add.
    let mark = rig.mark();
    let other = json!({ "project": "shop", "pipeline": "pipelines/other" });
    rig.send("/api/emit/pipeline.asked", other).await;
    let refused = rig.event("pipeline.refused", |event| event.seq > mark).await;
    let says = refused.data["says"].as_str().unwrap();
    assert!(says.starts_with("`pipelines/other` was not run: ") && says.contains("on: hand"), "{says}");
    // And whoever asked is told so in their inbox.
    rig.event("inbox.told", |event| event.seq > mark).await;
    let inbox = rig.read("/api/inbox").await;
    let told: Vec<&str> =
        inbox["lines"].as_array().unwrap().iter().filter_map(|line| line["says"].as_str()).collect();
    assert_eq!(told, [says], "{inbox}");
    // And handed what it does not take, it says which.
    let mark = rig.mark();
    let odd = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "colour": "red" } });
    rig.send("/api/emit/pipeline.asked", odd).await;
    let refused = rig.event("pipeline.refused", |event| event.seq > mark).await;
    assert!(refused.data["says"].as_str().unwrap().contains("`colour`"), "{}", refused.data);
}

#[tokio::test]
async fn a_card_coming_to_a_column_runs_the_pipeline_its_row_says_and_one_under_the_same_key_waits_for_it() {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    let fields = json!({ "pipeline": "pipelines/gates", "on": "column", "column": "review", "key": "gates" });
    row(&rig, fields).await;
    // The first run waits to be let go; the second is asked for meanwhile.
    let hold = json!({ "pipeline": "pipelines/gates", "on": "hand", "key": "gates" });
    row(&rig, hold).await;
    let mark = rig.mark();
    let asked = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "hold": true, "note": "first" } });
    rig.send("/api/emit/pipeline.asked", asked).await;
    rig.event("step.waiting", |event| event.seq > mark).await;

    let card = crate::fleets::add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &card, "review", None).await;
    // Its run is due, and waits for the one under its key: it has asked
    // the pool for nothing yet, and nothing ran.
    rig.event("pipeline.fired", |event| event.seq > mark && event.data["with"]["item"] == card.as_str())
        .await;
    rig.count("pool.asked", 1).await;
    assert_eq!(rig.count_of("gates.ran").await, 0);

    rig.send("/api/emit/gates.go", json!({})).await;
    assert_eq!(ran(&rig, mark).await["note"], "first");
    let second =
        rig.event("gates.ran", |event| event.seq > mark && event.data["item"] == card.as_str()).await;
    assert_eq!(second.data["note"], "");
    rig.count("pool.released", 2).await;
}

/// The run an asking named `asked` began, as Run's page asks for it.
async fn begun(rig: &Rig, asked: &str) -> Option<String> {
    let answer = rig.read(&format!("/api/pipelines/begun?project=shop&asked={asked}")).await;
    answer["run"].as_str().map(str::to_string)
}

#[tokio::test]
async fn each_asking_by_hand_finds_the_run_it_began_and_never_another_asked_for_at_the_same_moment() {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "hand", "key": "gates" })).await;
    // Two askings at once under one key: the first waits to be let go, the
    // second waits for the first, and neither has begun anything of the
    // other's.
    let mark = rig.mark();
    let first = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "hold": true, "note": "first" }, "asked": "one" });
    let second = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "note": "second" }, "asked": "two" });
    rig.send("/api/emit/pipeline.asked", first).await;
    rig.send("/api/emit/pipeline.asked", second).await;
    rig.event("step.waiting", |event| event.seq > mark && event.data["event"] == "gates.go").await;
    let one = begun(&rig, "one").await.expect("the first asking began a run");
    assert_eq!(begun(&rig, "two").await, None, "the second has begun nothing while the first holds its key");
    assert_eq!(begun(&rig, "three").await, None);

    rig.send("/api/emit/gates.go", json!({})).await;
    rig.event("gates.ran", |event| event.seq > mark && event.data["note"] == "second").await;
    let two = begun(&rig, "two").await.expect("the second asking began a run");
    assert_ne!(one, two);
    assert_eq!(begun(&rig, "one").await.as_deref(), Some(one.as_str()));
    // Each is the run of what was handed to it: the first was held, the second not.
    for (run, held) in [(&one, true), (&two, false)] {
        let read = rig.read(&format!("/api/runs/{run}")).await;
        assert_eq!((read["flow"].as_str(), read["status"].as_str()), (Some("main"), Some("ok")), "{read}");
        let passed_over = read["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["step"] == "held" && step["status"] == "skipped");
        assert_eq!(passed_over, !held, "{read}");
    }

    // And a refusal says which asking it refused.
    let mark = rig.mark();
    let odd = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "colour": "red" }, "asked": "four" });
    rig.send("/api/emit/pipeline.asked", odd).await;
    let refused = rig.event("pipeline.refused", |event| event.seq > mark).await;
    assert_eq!(refused.data["asked"], "four", "{}", refused.data);
}

#[tokio::test]
async fn the_run_an_asking_began_is_found_where_the_daemon_stopped_between_the_asking_and_the_run() {
    let mut rig = Rig::new().await;
    shop(&rig, None).await;
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "hand", "key": "gates" })).await;
    // The daemon goes once the pipeline is written down to be run, before
    // its run is: the run is begun by whichever comes to it first as the
    // daemon is back, the conductor's start or the step that asked.
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| {
        event.topic == "instance.started" && event.data["blueprint"] == "pipelines/gates"
    });
    let asked = json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "note": "stopped" }, "asked": "one" });
    rig.send("/api/emit/pipeline.asked", asked).await;
    journal.shut().await;
    rig.stop().await;
    rig.start().await;

    rig.event("gates.ran", |event| event.data["note"] == "stopped").await;
    // And the step that asked, taken up, has what it asked for, whichever began it.
    let asked =
        |event: &kernel::Event| event.data["blueprint"] == "std/pipeline" && event.data["flow"] == "on.go";
    rig.event("run.finished", asked).await;
    let one = begun(&rig, "one").await.expect("the asking's run is found");
    let read = rig.read(&format!("/api/runs/{one}")).await;
    let of = rig.read(&format!("/api/instances/{}", read["instance"].as_str().unwrap())).await;
    assert_eq!(
        (read["flow"].as_str(), of["instance"]["blueprint"].as_str()),
        (Some("main"), Some("pipelines/gates")),
        "{read}"
    );
    assert_eq!(rig.count_of("gates.ran").await, 1);
}

#[tokio::test]
async fn the_run_an_asking_began_is_found_by_its_name_however_many_runs_there_are_since() {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "hand", "key": "gates" })).await;
    let asking = |name: String| json!({ "project": "shop", "pipeline": "pipelines/gates", "with": { "note": name.clone() }, "asked": name });
    rig.send("/api/emit/pipeline.asked", asking("first".into())).await;
    rig.event("gates.ran", |event| event.data["note"] == "first").await;
    let first = begun(&rig, "first").await.expect("the first asking's run");
    // More runs of what the triggers call than any list of the newest would hold.
    for nth in 0..MANY {
        rig.send("/api/emit/pipeline.asked", asking(format!("then-{nth}"))).await;
    }
    let last = format!("then-{}", MANY - 1);
    rig.event("gates.ran", |event| event.data["note"] == last.as_str()).await;
    assert_eq!(begun(&rig, "first").await.as_deref(), Some(first.as_str()));
    let then = begun(&rig, &last).await.expect("the last asking's run");
    assert_ne!(then, first);
}

/// More runs than the 200 newest that were once looked through for an asking's.
const MANY: usize = 205;

#[tokio::test]
async fn a_pipeline_that_must_be_told_something_is_run_with_what_its_row_says_in_with() {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    let saved = json!({ "path": format!("@blueprints/pipelines/{}", brand::blueprint_file("deploy")), "content": DEPLOY });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let fields = json!({ "pipeline": "pipelines/deploy", "on": "column", "column": "done", "with": ["target=prod", "times=3"] });
    row(&rig, fields).await;
    let mark = rig.mark();
    let card = crate::fleets::add(&rig, "Ship it", &[]).await;
    move_card(&rig, &card, "done", None).await;
    let ran = rig.event("deploy.ran", |event| event.seq > mark).await;
    assert_eq!((ran.data["target"].as_str(), ran.data["times"].as_i64()), (Some("prod"), Some(3)));

    // What it is told is held to what writing a row takes: a key that may
    // write the table and holds no `shell` is refused the cell.
    let narrow = crate::held::key(&rig, &["records", "blueprints:run"]).await;
    let asked =
        json!({ "fields": { "pipeline": "pipelines/deploy", "on": "hand", "with": ["target=prod"] } });
    let (status, answer) =
        rig.call(Method::POST, "/api/collections/shop/triggers/records", Some(&narrow), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
}

/// A pipeline of the project's own that says which environment it ran under.
const WHERE: &str = r#"blueprint: pipelines/where
description: Say which environment it ran under.
permissions: [projects:read]

steps:
  ran:
    do: emit
    with: { topic: where.ran, under: "${ environment.name ?? '' }" }
"#;

#[tokio::test]
async fn a_row_runs_its_pipeline_under_the_environment_it_says_held_there_to_that_environments_rules() {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    let saved = json!({ "path": format!("@blueprints/pipelines/{}", brand::blueprint_file("where")), "content": WHERE });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    for name in ["staging", "prod"] {
        rig.send("/api/projects/shop/environments", json!({ "name": name })).await;
    }
    // What goes under prod is Grace's to let go on, and whoever applied the triggers is not her.
    let grace = crate::kept::person(&rig, "Grace").await;
    let rules = json!({ "approve": ["Grace"] });
    let (status, set) = rig
        .call(Method::PUT, "/api/projects/shop/environments/prod/rules", Some(&rig.token), Some(rules))
        .await;
    assert_eq!(status, StatusCode::OK, "{set}");

    // A row by hand under staging: the pipeline runs there, under a key of the pipeline's and the environment's.
    row(&rig, json!({ "pipeline": "pipelines/where", "on": "hand", "environment": "staging" })).await;
    let mark = rig.mark();
    rig.send("/api/emit/pipeline.asked", json!({ "project": "shop", "pipeline": "pipelines/where" })).await;
    let ran = rig.event("where.ran", |event| event.seq > mark).await;
    assert_eq!(ran.data["under"], "staging", "{:?}", ran.data);
    let listed = rig.read("/api/instances?project=shop&blueprint=pipelines/where").await;
    let under: Vec<(Value, Value)> = listed["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (one["key"].clone(), one["environment"].clone()))
        .collect();
    assert_eq!(under, [(json!("pipelines-where-staging"), json!("staging"))], "{listed}");

    // A row as a card comes to a column, under prod: started as whoever
    // applied the triggers, who is not Grace, it waits for her go-ahead,
    // and runs there once she gives it.
    row(
        &rig,
        json!({ "pipeline": "pipelines/where", "on": "column", "column": "review", "environment": "prod" }),
    )
    .await;
    let mark = rig.mark();
    let card = crate::fleets::add(&rig, "Ship the cart", &[]).await;
    move_card(&rig, &card, "review", None).await;
    let asked = rig.event("project.approval.asked", |event| event.seq > mark).await;
    assert_eq!(asked.data["environment"], "prod", "{:?}", asked.data);
    assert_eq!(rig.count_of("where.ran").await, 1);
    let ask = asked.data["ask"].as_str().unwrap();
    let (status, approved) = rig
        .call(Method::POST, &format!("/api/asked/{ask}/approve"), Some(&grace.token), Some(json!({})))
        .await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let ran = rig.event("where.ran", |event| event.seq > mark).await;
    assert_eq!(ran.data["under"], "prod", "{:?}", ran.data);
    rig.event("pool.released", |event| event.seq > mark).await;

    // Saying a row's environment takes what saying its rules takes: a key
    // that may write the table and run blueprints is refused the cell.
    let narrow = crate::held::key(&rig, &["records", "blueprints:run"]).await;
    let asked = json!({ "fields": { "pipeline": "pipelines/where", "on": "hand", "environment": "prod" } });
    let (status, answer) =
        rig.call(Method::POST, "/api/collections/shop/triggers/records", Some(&narrow), Some(asked)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.to_string().contains("environments:secrets"), "{answer}");
}

#[tokio::test]
async fn a_row_somebody_the_environments_rule_does_not_name_wrote_waits_for_a_go_ahead_though_whoever_applied_the_triggers_is_named()
 {
    let rig = Rig::new().await;
    shop(&rig, None).await;
    let saved = json!({ "path": format!("@blueprints/pipelines/{}", brand::blueprint_file("where")), "content": WHERE });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    rig.send("/api/tier", json!({ "tier": "premium" })).await;
    rig.send("/api/projects/shop/environments", json!({ "name": "prod" })).await;
    // What goes under prod is Ada's to let go on, and she applied the triggers.
    let rules = json!({ "approve": ["ada@example.com"] });
    let (status, set) = rig
        .call(Method::PUT, "/api/projects/shop/environments/prod/rules", Some(&rig.token), Some(rules))
        .await;
    assert_eq!(status, StatusCode::OK, "{set}");

    // A key that holds less, and holds what writing a row that names prod takes, writes one.
    let narrow = crate::held::key(&rig, &["records", "blueprints:run", "environments:secrets"]).await;
    let fields =
        json!({ "pipeline": "pipelines/where", "on": "column", "column": "review", "environment": "prod" });
    let (status, written) = rig
        .call(
            Method::POST,
            "/api/collections/shop/triggers/records",
            Some(&narrow),
            Some(json!({ "fields": fields })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{written}");
    let id = written["id"].as_str().unwrap().to_string();
    assert!(written["data"]["environment_by"].as_str().unwrap().starts_with("key:"), "{written}");
    // Nobody says who wrote it but the factory, the key and the owner alike.
    let claimed = json!({ "fields": { "environment_by": "user:ada" } });
    for token in [&narrow, &rig.token] {
        let (status, refused) =
            rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(token), Some(claimed.clone())).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    }

    // A card comes to review: the run does not go on on Ada's standing, she is asked first.
    let mark = rig.mark();
    let card = crate::fleets::add(&rig, "Ship the cart", &[]).await;
    move_card(&rig, &card, "review", None).await;
    let asked = rig.event("project.approval.asked", |event| event.seq > mark).await;
    assert_eq!(asked.data["environment"], "prod", "{:?}", asked.data);
    assert_eq!(rig.count_of("where.ran").await, 0);
    let ask = asked.data["ask"].as_str().unwrap();
    let (status, approved) =
        rig.call(Method::POST, &format!("/api/asked/{ask}/approve"), Some(&rig.token), Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{approved}");
    let ran = rig.event("where.ran", |event| event.seq > mark).await;
    assert_eq!(ran.data["under"], "prod", "{:?}", ran.data);
    rig.event("pool.released", |event| event.seq > mark).await;

    // Written by Ada herself, the row runs there with nobody asked.
    let mine = json!({ "fields": { "environment": "" } });
    rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), Some(mine)).await;
    let mine = json!({ "fields": { "environment": "prod" } });
    let (status, rewritten) =
        rig.call(Method::PATCH, &format!("/api/records/{id}"), Some(&rig.token), Some(mine)).await;
    assert_eq!(status, StatusCode::OK, "{rewritten}");
    assert!(rewritten["data"]["environment_by"].as_str().unwrap().starts_with("user:"), "{rewritten}");
    let mark = rig.mark();
    move_card(&rig, &card, "doing", None).await;
    move_card(&rig, &card, "review", None).await;
    rig.event("where.ran", |event| event.seq > mark).await;
    assert_eq!(rig.count_of("project.approval.asked").await, 1);
}

#[tokio::test]
async fn a_branch_that_moves_is_said_for_a_project_whose_branches_are_followed_and_runs_the_pipeline_its_row_says()
 {
    let rig = Rig::new().await;
    let dir = rig.dir.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "first"]);
    shop(&rig, Some(&dir)).await;
    // The branches are followed once the row is gone by (`changed`, which
    // calls what follows them), which is where they are kept from.
    let before = rig.mark();
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "branch", "branch": "main" })).await;
    let gone_by = |event: &kernel::Event| {
        event.seq > before && event.data["blueprint"] == "std/pipeline" && event.data["flow"] == "on.changed"
    };
    rig.event("run.finished", gone_by).await;

    let mark = rig.mark();
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "second"]);
    let commit = git(&dir, &["rev-parse", "HEAD"]);
    let moved = rig
        .event("project.branch.moved", |event| event.seq > mark && event.data["to"] == commit.as_str())
        .await;
    assert_eq!(moved.data["branch"], "main");
    assert_eq!(moved.data["project"], "shop");
    let handed = ran(&rig, mark).await;
    assert_eq!((handed["branch"].as_str(), handed["commit"].as_str()), (Some("main"), Some(commit.as_str())));

    // Another branch is said and runs nothing.
    let mark = rig.mark();
    git(&dir, &["branch", "other"]);
    rig.event("project.branch.moved", |event| event.seq > mark && event.data["branch"] == "other").await;
    assert_eq!(rig.count_of("gates.ran").await, 1);
}

#[tokio::test]
async fn a_row_of_every_runs_its_pipeline_each_time_so_long_has_gone_and_one_of_at_at_its_time_of_day() {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let rig =
        Rig::owned(Rig::with("api-pipelines", move |config, _| config.clock = Some(Arc::new(moved))).await)
            .await;
    shop(&rig, None).await;
    let mark = rig.mark();
    row(&rig, json!({ "pipeline": "pipelines/gates", "on": "every", "every": "1h", "key": "hourly" })).await;
    waiting(&rig, mark).await;
    let mark = rig.mark();
    clock.advance(Duration::from_secs(3600));
    ran(&rig, mark).await;
    // It waits again, and runs again an hour on.
    waiting(&rig, mark).await;
    let again = rig.mark();
    clock.advance(Duration::from_secs(3600));
    ran(&rig, again).await;
    assert_eq!(rig.count_of("gates.ran").await, 2);

    // The clock started at midnight UTC on the first of January, and is
    // two hours on.
    let mark = rig.mark();
    // Made before the time comes, it fires for it whenever it comes to listen.
    let nightly = json!({ "pipeline": "pipelines/gates", "on": "at", "at": "03:30 UTC", "key": "nightly" });
    let nightly = row(&rig, nightly).await;
    rig.event("instance.started", |event| event.seq > mark && event.data["blueprint"] == "std/pipeline-at")
        .await;
    clock.advance(Duration::from_secs(5400));
    let due = rig.event("pipeline.due", |event| event.seq > mark).await;
    assert_eq!(due.data["at"], json!(1_767_238_200_000_i64));
    let fired =
        rig.event("pipeline.fired", |event| event.seq > mark && event.data["row"] == nightly.as_str()).await;
    assert_eq!(fired.data["key"], "nightly");
}
