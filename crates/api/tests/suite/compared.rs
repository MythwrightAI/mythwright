//! Comparing ways of doing what a fleet does, over HTTP and with stand-in
//! agents: the two questions somebody running a fleet asks first. Which of
//! two agents, each on a model of its own, does a card better; and whether
//! a card is better done in one turn by one of them or in smaller turns by
//! the other. Neither is written in any blueprint: each is said from
//! outside of the round that ships, and the daemon does the rest.
//!
//! What is asserted is what was counted under each way. Which is better is
//! not something the daemon says.

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, given, rig, shop};
use crate::lanes::{move_card, moved};

/// Two who work, each a row of its own with a model of its own, and one
/// who reviews.
fn two() -> Value {
    json!([
        { "title": "big", "role": "worker", "count": 1, "adapter": "writer", "model": "big" },
        { "title": "small", "role": "worker", "count": 1, "adapter": "writer", "model": "small" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
    ])
}

/// A card added, made ready, worked, reviewed and done: its id.
async fn done(rig: &Rig, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    move_card(rig, &id, "ready", None).await;
    moved(rig, &id, "done", 0).await;
    id
}

/// How often `name` was measured under each way of the comparison, and by whom.
async fn counted(rig: &Rig, comparison: &str, name: &str) -> Vec<(String, String, u64)> {
    let path = format!("/api/metrics?name={name}&by=way,slot&labels=compared:{comparison}");
    let answer = rig.read(&path).await;
    let mut all: Vec<(String, String, u64)> = answer["series"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            let said = |label: &str| one["labels"][label].as_str().unwrap_or_default().to_string();
            (said("way"), said("slot"), one["total"]["count"].as_u64().unwrap())
        })
        .collect();
    all.sort();
    all
}

/// What was counted of what came of the things of a comparison: the way,
/// the count with what it came to, and how many.
async fn came(rig: &Rig, comparison: &str) -> Vec<(String, String, u64)> {
    let shown = rig.read(&format!("/api/comparisons/{comparison}")).await;
    let mut all: Vec<(String, String, u64)> = shown["measured"]
        .as_array()
        .unwrap()
        .iter()
        // How many things each way was given is counted beside these, and read from the columns.
        .filter(|one| one["name"] == "system.comparison.came" && one["labels"]["counted"] != "given")
        .map(|one| {
            let said = |label: &str| one["labels"][label].as_str().unwrap_or_default().to_string();
            assert!(one["total"]["max"].as_f64().unwrap() >= 0.0 && !said("text").is_empty(), "{one}");
            (
                said("way"),
                format!("{} {}", said("counted"), said("came")),
                one["total"]["count"].as_u64().unwrap(),
            )
        })
        .collect();
    all.sort();
    all
}

fn row(way: &str, slot: &str, count: u64) -> (String, String, u64) {
    (way.to_string(), slot.to_string(), count)
}

#[tokio::test]
async fn which_of_two_agents_works_a_card_better_is_compared_by_handing_each_way_one_of_them() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "agents": two() })).await;

    // mw compare start models --project shop --of std/lane-round --per item --also std/review \
    //     --way big --with big.agent=shop-big-1 --way small --with small.agent=shop-small-1
    let asked = json!({
        "name": "models", "project": "shop", "of": "std/lane-round", "per": "item", "also": ["std/review"],
        // What comes of each card: the usual counts for a board's cards, asked for by their word.
        "counts": "cards",
        "ways": [
            { "name": "big", "with": { "agent": "shop-big-1" } },
            { "name": "small", "with": { "agent": "shop-small-1" } },
        ],
    });
    let started = rig.send("/api/comparisons", asked).await;
    assert_eq!(started["of"], "std/lane-round", "{started}");
    assert_eq!(started["counts"].as_array().map(Vec::len), Some(11), "{started}");
    assert_eq!(
        started["counts"][0],
        json!({ "name": "moved", "topic": "board.item.moved", "thing": "id", "by": ["from", "to"], "accepts": ["done"] })
    );

    for title in ["Add a cart", "Add a till", "Add a shelf", "Add a door"] {
        done(&rig, title).await;
    }
    // Each card went to one of them, turn about, and nobody chose which.
    assert_eq!((given(&rig, "shop-big-1").await.len(), given(&rig, "shop-small-1").await.len()), (2, 2));
    let shown = rig.read("/api/comparisons/models").await;
    let went: Vec<(&str, u64)> = shown["comparison"]["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|way| (way["name"].as_str().unwrap(), way["went"].as_u64().unwrap()))
        .collect();
    assert_eq!(went, [("big", 2), ("small", 2)]);
    // Their turns side by side, and beside each the turns of whoever
    // reviewed what that way made: the review is started as it is written
    // and counted with the card's way.
    assert_eq!(
        counted(&rig, "models", "agent.turn").await,
        [
            row("big", "shop-big-1", 2),
            row("big", "shop-reviewer-1", 2),
            row("small", "shop-reviewer-1", 2),
            row("small", "shop-small-1", 2),
        ]
    );
    // What is shown of it holds every measurement under each way, by its name.
    let names: Vec<&str> =
        shown["measured"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"agent.turn") && names.contains(&"step"), "{names:?}");

    // What came of the cards, under the way each went: taken, handed to
    // review, passed and done, two of each a way, and for each how long
    // since the card was given its way. Nothing says which way did better.
    let mut wanted: Vec<(String, String, u64)> = ["big", "small"]
        .iter()
        .flat_map(|way| {
            // And each card once as accepted, having passed and come to done.
            ["accepted ", "moved doing>review", "moved ready>doing", "moved review>done", "verdict passed"]
                .map(|came| (way.to_string(), came.to_string(), 2))
        })
        .collect();
    // And what came after each card's work was merged: every card's work changes the one file, so
    // each later card's merge changes again the files of every card before it, under that card's
    // way. Worked one after another, turn about, big has the first and the third (three and one),
    // small the second and the last (two and none).
    wanted.extend(
        [("big", 4), ("small", 2)].map(|(way, count)| (way.into(), "after changed_again".into(), count)),
    );
    wanted.sort();
    crate::soon("what came of each card to be counted", async {
        while came(&rig, "models").await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;

    // The same laid beside each other, as the command line and the client show it: a column for
    // each way with how many cards it was given, and under it what came of them in the order it
    // comes about, then what those who reviewed chose, then what else was measured.
    let laid = rig.read("/api/comparisons/models").await["beside"].clone();
    let columns: Vec<(&str, u64)> = laid["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|column| (column["way"].as_str().unwrap(), column["given"].as_u64().unwrap()))
        .collect();
    assert_eq!(columns, [("big", 2), ("small", 2)], "{laid}");
    let rows: Vec<(&str, &str, Vec<u64>)> = laid["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let by = row["by"].as_array().unwrap().iter().map(|cell| cell["count"].as_u64().unwrap_or(0));
            (row["kind"].as_str().unwrap(), row["what"].as_str().unwrap(), by.collect())
        })
        .collect();
    let came: Vec<&str> = rows.iter().filter(|row| row.0 == "came").map(|row| row.1).collect();
    assert_eq!(came.first(), Some(&"moved ready>doing"), "taken first, as it comes about: {came:?}");
    assert_eq!(came.len(), 5, "{came:?}");
    assert!(rows.contains(&("chose", "agent.turn approve", vec![2, 2])), "{rows:?}");
    assert!(rows.iter().any(|row| row.0 == "took" && row.1 == "step"), "{rows:?}");
    // Every card of each was accepted, of the two it was given.
    assert!(rows.contains(&("accepted", "accepted", vec![2, 2])), "{rows:?}");
    let kinds: Vec<&str> = rows.iter().map(|row| row.0).collect();
    let order = ["came", "chose", "spent", "accepted", "per_accepted", "took"];
    assert!(kinds.is_sorted_by_key(|kind| order.iter().position(|one| one == kind)), "{kinds:?}");
    assert!(laid.to_string().find("better").is_none() && laid.get("winner").is_none(), "nothing is scored");

    // Ended saying which way everything goes from now on: no longer counted.
    let ended = rig.send("/api/comparisons/models/end", json!({ "goes": "small" })).await;
    assert_eq!(ended["goes"], "small", "{ended}");
    done(&rig, "Add a sign").await;
    done(&rig, "Add a bell").await;
    assert_eq!((given(&rig, "shop-big-1").await.len(), given(&rig, "shop-small-1").await.len()), (2, 4));
    assert_eq!(counted(&rig, "models", "agent.turn").await.iter().map(|one| one.2).sum::<u64>(), 8);
}

#[tokio::test]
async fn a_card_left_for_a_person_is_counted_under_its_way_with_the_usual_counts_of_a_boards_cards() {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "agents": two() })).await;
    // Whoever reviews neither approves nor sends back, so the card is left for a person.
    std::fs::write(at.join("says-shop-reviewer-1"), "I am not sure what to make of this.\n").unwrap();
    let asked = json!({
        "name": "left", "project": "shop", "of": "std/lane-round", "per": "item", "also": ["std/review"],
        "counts": "cards",
        "ways": [
            { "name": "big", "with": { "agent": "shop-big-1" } },
            { "name": "small", "with": { "agent": "shop-small-1" } },
        ],
    });
    let started = rig.send("/api/comparisons", asked).await;
    let counts = started["counts"].as_array().unwrap();
    assert!(counts.contains(&json!({ "name": "left", "topic": "card.left", "thing": "card" })), "{started}");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    rig.event("card.left", move |event| event.data["card"] == id).await;
    // The first card goes to the first way: taken, handed to review, undecided and left there,
    // which is a count of its own beside what the review came to. Nothing of it was accepted.
    let wanted: Vec<(String, String, u64)> =
        ["left left", "moved doing>review", "moved ready>doing", "verdict undecided"]
            .map(|came| ("big".to_string(), came.to_string(), 1))
            .into();
    crate::soon("the card left for a person to be counted", async {
        while came(&rig, "left").await != wanted {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

/// The work of a round as it ships, as somebody's own: before the work
/// its agent is asked only how it would do it, so that the work is a
/// second, smaller turn.
fn in_parts() -> String {
    let ships = include_str!("../../../../blueprints/std/work.bp");
    let whole = "    needs: [asks]\n    call: std/turn\n";
    let parts = "    needs: [asks, outline]\n    call: std/turn\n";
    let outline = "  outline:
    description: Asked first only how it would do the card; the work is a turn of its own after it.
    needs: [unplaced]
    do: agent.task
    with:
      slot: \"${params.agent}\"
      thread: \"lane:${params.item}\"
      cwd: \"${steps.made.out.dir}\"
      prompt: \"Card ${params.item}: ${steps.card.out.title}. Say in a few lines how you would do it, and change nothing yet.\"

  work:
";
    assert!(ships.contains(whole), "the work's own step is no longer written this way");
    ships.replace("blueprint: std/work", "blueprint: std/work-in-parts").replace(whole, parts).replacen(
        "  work:\n",
        outline,
        1,
    )
}

#[tokio::test]
async fn one_turn_by_one_agent_is_compared_with_smaller_turns_by_another_in_a_round_of_ones_own() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "agents": two() })).await;
    // Their own way of working a card is a file of the project's, beside
    // the one that ships, so that what it tells its agent is the files that
    // ship beside that one (`std/prompts/`).
    let path = format!("@blueprints/std/{}", brand::blueprint_file("work-in-parts"));
    let body = json!({ "path": path, "content": in_parts() });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // mw compare start turns --project shop --of std/work --per item \
    //     --way whole --with whole.agent=shop-big-1 \
    //     --way parts=std/work-in-parts --with parts.agent=shop-small-1
    let asked = json!({
        "name": "turns", "project": "shop", "of": "std/work", "per": "item",
        "ways": [
            { "name": "whole", "with": { "agent": "shop-big-1" } },
            { "name": "parts", "blueprint": "std/work-in-parts", "with": { "agent": "shop-small-1" } },
        ],
    });
    rig.send("/api/comparisons", asked).await;
    for title in ["Add a cart", "Add a till", "Add a shelf", "Add a door"] {
        done(&rig, title).await;
    }
    // Two cards each: one turn a card the one way, two the other. All four
    // were worked, kept, reviewed and done by the fleet as it stands.
    assert_eq!(
        counted(&rig, "turns", "agent.turn").await,
        [row("parts", "shop-small-1", 4), row("whole", "shop-big-1", 2)]
    );
    let outlined = given(&rig, "shop-small-1").await;
    assert!(outlined[0].contains("Say in a few lines how you would do it"), "{outlined:?}");

    // Ended with nothing said, everything is the work that ships again.
    rig.send("/api/comparisons/turns/end", json!({})).await;
    done(&rig, "Add a sign").await;
    assert_eq!(counted(&rig, "turns", "agent.turn").await.iter().map(|one| one.2).sum::<u64>(), 6);
}

/// Has the work done for each thing that is wanted.
const WANTED: &str = "\
blueprint: wanted
on:
  wanted:
    event: shop.wanted
    steps:
      done: { call: work, as: \"work-${ event.data.id }\", with: { item: ${ event.data.id } } }
";

/// The work: it says what it spent, in what its names end in, and what it made.
const WORK: &str = "\
blueprint: work
params:
  item: { type: int }
steps:
  spent: { do: measure, with: { name: shop.spent_usd, value: 0.5 } }
  read: { do: measure, with: { name: shop.read_tokens, value: ${ params.item * 1000 } } }
  made: { do: emit, with: { topic: shop.made, id: ${ params.item }, how: well } }
";

#[tokio::test]
async fn what_is_not_a_time_is_added_up_under_each_way_and_said_for_each_thing_it_was_given() {
    let (rig, _at) = rig().await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for (name, text) in [("wanted", WANTED), ("work", WORK)] {
        let file = format!("@blueprints/{}", brand::blueprint_file(name));
        let (status, kept) = rig
            .call(
                Method::PUT,
                "/api/projects/shop/file",
                Some(&rig.token),
                Some(json!({ "path": file, "content": text })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{kept} in {project}");
    }
    rig.send("/api/apply", json!({ "blueprint": "wanted", "project": "shop", "wait": true })).await;
    let asked = json!({
        "name": "spending", "project": "shop", "of": "work", "per": "item",
        "counts": [{ "name": "made", "topic": "shop.made", "thing": "id", "by": ["how"] }],
        "ways": [{ "name": "a" }, { "name": "b", "share": 3 }],
    });
    rig.send("/api/comparisons", asked).await;
    for id in 1..=4 {
        let after = rig.mark();
        rig.send("/api/emit/shop.wanted", json!({ "id": id })).await;
        rig.event("shop.made", move |event| event.seq > after).await;
    }
    // A measurement says what it is of by how its name ends, wherever it is handed out.
    let spent = rig.read("/api/metrics?name=shop.spent_usd&by=way&labels=compared:spending").await;
    let series = spent["series"].as_array().unwrap();
    assert!(series.iter().all(|one| one["unit"] == "usd"), "{spent}");
    assert_eq!(series.iter().map(|one| one["total"]["sum"].as_f64().unwrap()).sum::<f64>(), 2.0, "{spent}");
    let timed = rig.read("/api/metrics?name=step&labels=compared:spending").await;
    assert!(timed["series"][0].get("unit").is_none(), "a time says nothing: {timed}");

    // Laid beside each other: in all, and for each thing the way was given. One thing went `a`
    // and three `b`, by their shares.
    let laid = crate::soon("what came of each to be counted", async {
        loop {
            let laid = rig.read("/api/comparisons/spending").await["beside"].clone();
            let came = laid["rows"].as_array().unwrap().iter().find(|row| row["kind"] == "came").cloned();
            if came.is_some_and(|row| row["by"][0]["count"] == 1 && row["by"][1]["count"] == 3) {
                return laid;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let given: Vec<u64> =
        laid["columns"].as_array().unwrap().iter().map(|column| column["given"].as_u64().unwrap()).collect();
    assert_eq!(given, [1, 3], "{laid}");
    let row = |what: &str| {
        let rows = laid["rows"].as_array().unwrap();
        rows.iter().find(|row| row["what"] == what).unwrap_or_else(|| panic!("no row {what}: {laid}")).clone()
    };
    let usd = row("shop.spent_usd");
    assert_eq!((&usd["kind"], &usd["unit"]), (&json!("spent"), &json!("usd")), "{usd}");
    assert_eq!((usd["by"][0]["sum"].as_f64(), usd["by"][0]["per_thing"].as_f64()), (Some(0.5), Some(0.5)));
    assert_eq!((usd["by"][1]["sum"].as_f64(), usd["by"][1]["per_thing"].as_f64()), (Some(1.5), Some(0.5)));
    let tokens = row("shop.read_tokens");
    assert_eq!(tokens["unit"], "tokens", "{tokens}");
    let all: f64 = tokens["by"].as_array().unwrap().iter().map(|cell| cell["sum"].as_f64().unwrap()).sum();
    assert_eq!(all, 10_000.0, "{tokens}");
    // What came of them is no amount, and neither is a time: nothing is said for each thing.
    assert!(
        row("made well")["by"][0].get("per_thing").is_none()
            && row("step")["by"][0].get("per_thing").is_none()
    );
}

#[tokio::test]
async fn two_prompts_for_those_who_review_are_compared_with_nothing_else_changed() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "agents": two() })).await;
    // Other words for whoever reviews, a file of the project's own, saved by its owner.
    let strict = "Card ${item}: ${title}\n\nBe strict with it. ${where} Choose `approve` or `send_back`.\n";
    let saved = json!({ "path": "@blueprints/prompts/strict.md", "content": strict });
    let (status, answer) =
        rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");

    // mw compare start prompts --project shop --of std/review --per item \
    //     --way usual --way strict --prompt strict.std/prompts/review-seat.md=prompts/strict.md
    let asked = json!({
        "name": "prompts", "project": "shop", "of": "std/review", "per": "item", "counts": "cards",
        "ways": [
            { "name": "usual" },
            { "name": "strict", "prompts": { "std/prompts/review-seat.md": "prompts/strict.md" } },
        ],
    });
    let started = rig.send("/api/comparisons", asked).await;
    let reads = &started["ways"][1]["prompts"]["std/prompts/review-seat.md"];
    assert_eq!(reads["name"], "prompts/strict.md", "{started}");
    assert!(
        reads["handed"].as_array().unwrap().contains(&json!("where")),
        "what the seat hands its file: {reads}"
    );

    for title in ["Add a cart", "Add a till", "Add a shelf", "Add a door"] {
        done(&rig, title).await;
    }
    // Each card's review went one way: two were asked in the words that ship, two in the project's.
    let asked_how = async || {
        let all = given(&rig, "shop-reviewer-1").await;
        let strict = all.iter().filter(|prompt| prompt.contains("Be strict with it.")).count();
        let usual = all.iter().filter(|prompt| prompt.contains("This card has been worked on")).count();
        (usual, strict)
    };
    assert_eq!(asked_how().await, (2, 2));
    // Shown under each way: which words it was run with, and what came of its cards.
    let shown = rig.read("/api/comparisons/prompts").await;
    let columns = shown["beside"]["columns"].as_array().unwrap();
    assert_eq!((&columns[0]["way"], &columns[0]["reads"]), (&json!("usual"), &json!([])), "{columns:?}");
    let read = &columns[1]["reads"][0];
    assert_eq!(
        (&columns[1]["way"], &read["for"], &read["name"]),
        (&json!("strict"), &json!("std/prompts/review-seat.md"), &json!("prompts/strict.md")),
        "{columns:?}"
    );
    assert_eq!(read["words"].as_str().map(str::len), Some(8), "{read}");
    assert_eq!((&columns[0]["given"], &columns[1]["given"]), (&json!(2), &json!(2)));
    crate::soon("what the reviews came to under each way to be counted", async {
        loop {
            let passed: Vec<u64> = came(&rig, "prompts")
                .await
                .iter()
                .filter(|one| one.1 == "verdict passed")
                .map(|one| one.2)
                .collect();
            if passed == [2, 2] {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;

    // Ended saying the project's words go on: every review is asked in them from now on.
    let ended = rig.send("/api/comparisons/prompts/end", json!({ "goes": "strict" })).await;
    assert_eq!(ended["goes"], "strict", "{ended}");
    done(&rig, "Add a sign").await;
    done(&rig, "Add a bell").await;
    assert_eq!(asked_how().await, (2, 4));
    // What is wrong with what a way is to read is said when it is started.
    let wrong = json!({
        "name": "other", "project": "shop", "of": "std/lane-round",
        "ways": [{ "name": "a", "prompts": { "std/prompts/review-seat.md": "prompts/strict.md" } }],
    });
    let (status, refused) = rig.call(Method::POST, "/api/comparisons", Some(&rig.token), Some(wrong)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("nothing this comparison starts links that file"), "{refused}");
}

#[tokio::test]
async fn a_comparison_is_started_by_whoever_may_apply_and_refused_with_what_is_wrong_with_it() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let ask = |asked: Value| rig.call(Method::POST, "/api/comparisons", Some(&rig.token), Some(asked));
    let (status, answer) =
        ask(json!({ "name": "x", "project": "shop", "of": "std/nothing", "ways": [{ "name": "a" }] })).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{answer}");
    let hands = json!({
        "name": "x", "project": "shop", "of": "std/lane-round",
        "ways": [{ "name": "a", "with": { "volume": 11 } }],
    });
    let (status, answer) = ask(hands).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(answer.to_string().contains("hands `volume`, which `std/lane-round` does not take"), "{answer}");
    // What is counted is each count, or the one word for a board's cards; and a count is of an event's own fields.
    let word = json!({ "name": "x", "project": "shop", "of": "std/lane-round", "ways": [{ "name": "a" }], "counts": "everything" });
    let (status, answer) = ask(word).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(answer.to_string().contains("or `cards` for the usual ones of a board's cards"), "{answer}");
    let no_thing = json!({
        "name": "x", "project": "shop", "of": "std/lane-round", "ways": [{ "name": "a" }],
        "counts": [{ "name": "moved", "topic": "board.item.moved", "thing": "" }],
    });
    let (status, answer) = ask(no_thing).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    // Nobody who is not signed in says how anything is started.
    let asked = json!({ "name": "x", "of": "std/lane-round", "ways": [{ "name": "a" }] });
    let (status, _) = rig.call(Method::POST, "/api/comparisons", None, Some(asked)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(rig.read("/api/comparisons?project=shop").await["comparisons"], json!([]));
}

/// Starts the work for each thing that is wanted, naming the comparison
/// and the way the asking names, and counts it as done.
const ASKING: &str = "\
blueprint: asking
on:
  wanted:
    event: shop.wanted
    steps:
      done:
        call: tuned
        as: \"tuned-${ event.data.id }\"
        with: { id: \"${ event.data.id }\", compared: \"${ event.data.compared ?? '' }\", way: \"${ event.data.way ?? '' }\" }
";

/// Work that takes the two params a start names a comparison by.
const TUNED: &str = "\
blueprint: tuned
params: { id: '', tone: plain, compared: '', way: '' }
steps:
  made: { do: emit, with: { topic: shop.tuned, id: \"${ params.id }\", tone: \"${ params.tone }\" } }
";

#[tokio::test]
async fn a_comparison_asked_for_over_http_gives_a_start_the_way_it_names_and_is_asked_for_again_with_a_way_more()
 {
    let (rig, _at) = rig().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    for (name, text) in [("asking", ASKING), ("tuned", TUNED)] {
        let file = format!("@blueprints/{}", brand::blueprint_file(name));
        let saved = json!({ "path": file, "content": text });
        let (status, kept) =
            rig.call(Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
        assert_eq!(status, StatusCode::OK, "{kept}");
    }
    rig.send("/api/apply", json!({ "blueprint": "asking", "project": "shop", "wait": true })).await;
    let ways = |names: &[&str]| -> Value {
        names.iter().map(|name| json!({ "name": name, "with": { "tone": name } })).collect()
    };
    let asked = json!({ "name": "tones", "project": "shop", "of": "tuned", "per": "id", "asked": true, "ways": ways(&["loud", "quiet"]) });
    let started = rig.send("/api/comparisons", asked.clone()).await;
    assert_eq!(started["asked"], true, "{started}");
    // One that shares its ways out is in force beside it for the same blueprint.
    let shared = json!({ "name": "shares", "project": "shop", "of": "tuned", "ways": ways(&["kind"]) });
    rig.send("/api/comparisons", shared).await;
    // Asked for again with a way more: the same comparison, with that way added.
    let mut more = asked.clone();
    more["ways"] = ways(&["loud", "quiet", "soft"]);
    let again = rig.send("/api/comparisons", more).await;
    assert_eq!(
        (again["started"].clone(), again["ways"].as_array().unwrap().len()),
        (started["started"].clone(), 3),
        "{again}"
    );

    // Each start goes the way it names, whatever the shares; one that names nothing goes `shares`'s.
    for (id, way) in [("1", "soft"), ("2", "quiet"), ("3", "")] {
        let after = rig.mark();
        let mut wanted = json!({ "id": id });
        if !way.is_empty() {
            wanted["compared"] = json!("tones");
            wanted["way"] = json!(way);
        }
        rig.send("/api/emit/shop.wanted", wanted).await;
        let made = rig.event("shop.tuned", move |event| event.seq > after).await;
        let tone = if way.is_empty() { "kind" } else { way };
        assert_eq!(made.data["tone"], tone, "{}", made.data);
    }
    let one = rig.read("/api/comparisons/tones").await;
    let went: Vec<(String, u64)> = one["comparison"]["ways"]
        .as_array()
        .unwrap()
        .iter()
        .map(|way| (way["name"].as_str().unwrap().to_string(), way["went"].as_u64().unwrap()))
        .collect();
    let wanted: Vec<(String, u64)> = vec![("loud".into(), 0), ("quiet".into(), 1), ("soft".into(), 1)];
    assert_eq!(went, wanted, "{one}");
    // Nothing goes one way after it: a start names its way.
    let (status, refused) = rig.post("/api/comparisons/tones/end", json!({ "goes": "loud" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
}
