//! What a stage does with a card may be a blueprint of the project's own.
//!
//! A row of `stages` says what is done with the cards of its column. Three
//! words ship (`plan`, `work`, `review`); any other is the name of a
//! blueprint, which the stage starts for each card that comes. Nothing of
//! the program is changed for a fourth thing: it is a row and a file.

use axum::http::{Method, StatusCode};
use serde_json::json;

use crate::fleets::{add, change, rig, says, shop, shop_with};
use crate::lanes::{history, move_card, moved};

/// What a stage that weighs does with one card: it says so on the card as
/// it sends it on. It is handed what any such blueprint is.
const WEIGHED: &str = r#"blueprint: weighed
description: Weigh one card, and send it on.
permissions: [board]

params:
  project: { type: string }
  item: { type: string }
  channel: { type: string, default: "" }
  role: { type: string, default: "" }
  picks: { type: string, default: first }
  from: { type: string }
  to: { type: string }
  pool: { type: string, default: "" }
  note: { type: string, default: "" }

steps:
  on:
    do: board.move
    with: { id: "${params.item}", to: "${params.to}", note: "weighed, in ${params.from}" }
"#;

/// A shop where a card that was worked is weighed before anyone looks at
/// it. `weighed` is a word nothing in the program knows.
async fn weighing(rig: &crate::Rig) {
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "weighing" },
        { "title": "weigh", "column": "weighing", "does": "weighed", "role": "worker", "pass": "review" },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer",
            "pass": "done", "fail": "doing", "merge": true,
        },
    ]);
    let columns = ["draft", "ready", "doing", "weighing", "review", "done"];
    shop_with(rig, &columns, json!({ "reviewers": 0, "stages": stages })).await;
    let saved =
        json!({ "path": format!("@blueprints/{}", brand::blueprint_file("weighed")), "content": WEIGHED });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
}

#[tokio::test]
async fn a_card_moved_or_added_where_a_stage_does_a_fourth_thing_as_the_daemon_went_with_nobody_told_has_it_done_when_it_is_back()
 {
    let (mut rig, _at) = rig().await;
    weighing(&rig).await;
    // A card is put where cards are weighed, and another is added there.
    // The daemon goes before either is said.
    let put = add(&rig, "Add a cart", &[]).await;
    crate::late::moved_and_not_said(&rig, &put, "weighing").await;
    let added = crate::late::added_and_not_said(&rig, &put, "Add a till", "weighing").await;

    rig.restart().await;

    // Each is said late when the daemon is back, and the stage starts the
    // project's blueprint for it as it does for a card that comes.
    for id in [&put, &added] {
        history(&rig, id, "conductor moved it from weighing to review, saying: weighed, in weighing").await;
    }
}

#[tokio::test]
async fn a_fourth_thing_a_stage_does_is_a_row_and_a_blueprint_of_the_projects_own_and_nothing_of_the_program()
{
    let (rig, _at) = rig().await;
    weighing(&rig).await;

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;

    // The stage started the project's blueprint for the card when it came,
    // called after what the stage does, and what it did is in the card's
    // history as anything a stage does is.
    history(&rig, &id, "conductor moved it from weighing to review, saying: weighed, in weighing").await;
    let instances = rig.read("/api/instances?project=shop&all=true").await;
    let named = format!("weighed-{id}");
    assert!(
        instances["instances"].as_array().unwrap().iter().any(|one| one["key"] == named.as_str()),
        "{instances}"
    );
}

/// The table of stages as a fleet declared it when what a stage does was
/// one of three words, with the two rows it began with: what a project
/// that had a fleet before has.
const AS_IT_WAS: &str = r#"blueprint: before
permissions: [projects, records, agents]
steps:
  stages:
    do: collection.ensure
    with:
      name: stages
      takes: agents:write
      fields:
        title: { type: text, required: true }
        column: { type: text }
        does: { type: choice, of: [plan, work, review] }
        role: { type: text }
        picks: { type: choice, of: [first, turn, least, rank] }
        seats: { type: number }
        needed: { type: number }
        pass: { type: text }
        fail: { type: text }
        rounds: { type: number }
        merge: { type: flag }
        in_flight: { type: choice, of: [finish, restart] }
      views:
        stages: { kind: table }

  work:
    needs: [stages]
    do: record.add
    with: { collection: stages, fields: { title: work, column: ready, does: work, role: worker, pass: review } }

  review:
    needs: [work]
    do: record.add
    with:
      collection: stages
      fields: { title: review, column: review, does: review, role: reviewer, pass: done, fail: doing, merge: true }
"#;

#[tokio::test]
async fn a_fleet_whose_stages_were_declared_when_a_stage_did_one_of_three_things_is_applied_again_with_its_rows_as_they_were_and_nothing_to_press()
 {
    let (rig, _at) = rig().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    for args in [
        &["init", "--quiet", "."][..],
        &["config", "user.name", "Ada"],
        &["config", "user.email", "ada@example.com"],
        &["add", "-A"],
        &["commit", "--quiet", "-m", "first"],
    ] {
        crate::lanes::git(&dir, args);
    }
    let board = json!({ "blueprint": "std/board", "project": "shop", "wait": true });
    assert_eq!(rig.send("/api/apply", board).await["finished"]["status"], "ok");
    let before =
        rig.send("/api/apply", json!({ "source": AS_IT_WAS, "project": "shop", "wait": true })).await;
    assert_eq!(before["finished"]["status"], "ok", "{before}");

    // The fleet as it ships now, over the table as it was: what a stage
    // does is any word now, and the two it had are words like any other.
    crate::fleets::fleet(&rig, json!({})).await;
    let rows = rig.read("/api/collections/shop/stages/records").await;
    let does: Vec<&str> =
        rows["records"].as_array().unwrap().iter().map(|row| row["data"]["does"].as_str().unwrap()).collect();
    assert_eq!(does, ["work", "review"], "the rows it had, and no others: {rows}");
    // The cell offers the three that ship from then, to whoever fills it:
    // the table says so, which as it was declared it never did.
    let tables = rig.read("/api/collections?project=shop").await;
    let stages =
        tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "stages").unwrap();
    let said =
        |name: &str| stages["fields"].as_array().unwrap().iter().find(|field| field["name"] == name).unwrap();
    assert_eq!(said("does")["suggests"], json!(["plan", "work", "review"]), "{stages}");
    assert_eq!(said("role")["suggests"], json!(["architect", "worker", "reviewer"]), "{stages}");
    // And it says whose records a stage steers, which it never did: the
    // rows it had are there, written by nobody again.
    assert_eq!(stages["steers"], "agents", "{stages}");
    // And it has what a column offers to be pressed, which it never had:
    // a table with no row, which holds no card anywhere.
    let actions =
        tables["collections"].as_array().unwrap().iter().find(|one| one["name"] == "actions").unwrap();
    assert_eq!((&actions["model"], &actions["beside"]), (&json!("action"), &json!("board")), "{actions}");
    assert_eq!(rig.read("/api/collections/shop/actions/records").await["records"], json!([]));

    // And it works a card and has it reviewed, by those rows.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
}

#[tokio::test]
async fn what_a_review_comes_to_is_said_in_the_channel_only_where_the_stages_row_says_so() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    // What the channel was told that begins with a card's id.
    let said_there = async |card: &str| -> Vec<String> {
        crate::lanes::said(&rig)
            .await
            .lines()
            .filter(|line| line.starts_with(card))
            .map(str::to_string)
            .collect()
    };

    // As the fleet starts, a card that passes is said on the card and
    // nowhere else: the channel is told what goes wrong and no more.
    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    moved(&rig, &first, "done", 0).await;
    history(&rig, &first, "shop-reviewer-1 approved it: It does what the card asks.").await;
    assert_eq!(said_there(&first).await, Vec::<String>::new());

    // The row says otherwise, and from then what each review comes to is
    // said there too, by the factory's bot. The reviewer waits to be let
    // go each time it looks, so that what it is to say is settled first.
    change(&rig, "stages", "review", json!({ "reports": "verdicts" })).await;
    let pipe = at.join("hold-shop-reviewer-1");
    crate::hold(&pipe);
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let second = add(&rig, "Add a till", &[]).await;
    let now = rig.mark();
    move_card(&rig, &second, "ready", None).await;
    let told = |words: &'static str| {
        rig.event("channel.message", move |event| {
            event.seq > now && event.data["body"].as_str().unwrap_or_default().contains(words)
        })
    };

    // Sent back, with what was said is wrong;
    crate::let_go("the reviewer to be looking at the card", &pipe).await;
    let back = told("was sent back").await;
    assert_eq!(
        back.data["body"],
        format!("{second} was sent back to doing: shop-reviewer-1: The total is wrong.")
    );
    assert!(back.data["author"].as_str().is_some_and(|author| author.starts_with("bot:")), "{back:?}");

    // and passed, with who approved it and why.
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    crate::let_go("the reviewer to be looking at the card again", &pipe).await;
    let passed = told("was approved").await;
    assert_eq!(
        passed.data["body"],
        format!("{second} was approved by shop-reviewer-1: It does what the card asks.")
    );
}
