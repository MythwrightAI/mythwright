//! A board's columns somebody added are the board's own, as a column of
//! a table is: its blueprint applied again, and a new version
//! of it taken, keeps them where they were, with the cards in them, and
//! puts a column of its own that is new after the one it follows.
//! `mw board columns`, the route and the board's page are one door
//! (`POST /api/board/<project>/columns`), which is what is played here.

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

/// The board's text as it shipped before: what ships now under the
/// version before it, starting with one column fewer.
fn board_before() -> String {
    let now = include_str!("../../../../blueprints/std/board.bp");
    let said = now.lines().find(|line| line.starts_with("version: ")).expect("its version");
    let number: u64 = said["version: ".len()..].parse().expect("a number");
    let usual = "default: [backlog, ready, doing, review, done]";
    assert!(now.contains(usual), "the columns a board starts with");
    now.replace(said, &format!("version: {}", number - 1))
        .replace(usual, "default: [backlog, ready, doing, done]")
}

/// What the shop's board stands as: its columns, and the cards by title with their column.
async fn standing(rig: &Rig) -> (Vec<String>, Vec<(String, String)>) {
    let board = rig.read("/api/board/shop").await;
    let columns =
        board["columns"].as_array().unwrap().iter().map(|one| one.as_str().unwrap().to_string()).collect();
    let mut cards: Vec<(String, String)> = board["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|card| {
            (card["title"].as_str().unwrap().to_string(), card["status"].as_str().unwrap().to_string())
        })
        .collect();
    cards.sort();
    (columns, cards)
}

/// `std/board` applied for the shop with `params`, taking what ships where `take` says: the version it then runs.
async fn applied(rig: &Rig, params: Value, take: bool) -> String {
    let body =
        json!({ "blueprint": "std/board", "project": "shop", "params": params, "take": take, "wait": true });
    let (status, answer) = rig.call(Method::POST, "/api/apply", Some(&rig.token), Some(body)).await;
    // Applied as it is, with nothing changed, nothing runs.
    let ran = answer["finished"]["status"] == "ok" || (answer["changed"] == false && answer["run"].is_null());
    assert!(status == StatusCode::OK && ran, "{answer}");
    let listed = rig.read("/api/instances?project=shop&live=true").await;
    let board = listed["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == "std/board");
    board.expect("the board's instance")["version"].as_str().unwrap_or("none").to_string()
}

fn words(all: &[&str]) -> Vec<String> {
    all.iter().map(|one| one.to_string()).collect()
}

#[tokio::test]
async fn columns_added_to_a_board_stand_where_they_were_through_its_blueprint_applied_again_and_a_new_version_taken()
 {
    let mut at = None;
    let rig = Rig::with("api-kept-columns", |config, dir| {
        let std = dir.join("before").join("std");
        std::fs::create_dir_all(&std).unwrap();
        std::fs::write(std.join(brand::blueprint_file("board")), board_before()).unwrap();
        config.blueprints_dir = Some(dir.join("before"));
        at = Some(dir.to_path_buf());
    })
    .await;
    let rig = Rig::owned(rig).await;
    let held = json!({ "come": "held" });
    let (status, answer) = rig.call(Method::POST, "/api/system/texts", Some(&rig.token), Some(held)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let ships = include_str!("../../../../blueprints/std/board.bp")
        .lines()
        .find_map(|line| line.strip_prefix("version: "))
        .expect("its version")
        .to_string();
    let before = (ships.parse::<u64>().unwrap() - 1).to_string();

    // The board as it was applied, with nothing said: the columns its text starts with.
    assert_eq!(applied(&rig, json!({}), false).await, before);
    assert_eq!(standing(&rig).await.0, words(&["backlog", "ready", "doing", "done"]));

    // Two columns added, as `mw board columns` and the board's page add them: one
    // between two of the board's own, and one before the last; a card in each column.
    let arranged = words(&["backlog", "ready", "doing", "reviewed", "staging", "done"]);
    let set = rig.send("/api/board/shop/columns", json!({ "columns": arranged })).await;
    assert_eq!(set["columns"], json!(arranged), "{set}");
    let mut cards = Vec::new();
    for column in &arranged {
        let title = format!("In {column}");
        rig.send("/api/board/shop/items", json!({ "title": title, "status": column })).await;
        cards.push((title, column.clone()));
    }
    cards.sort();
    assert_eq!(standing(&rig).await, (arranged.clone(), cards.clone()));

    // Applied again with its own columns said and one more, as it is held:
    // it declares the board again, the columns somebody added are as they
    // were, with their cards, and the board follows it for the one more.
    let more = json!({ "columns": ["backlog", "ready", "doing", "done", "archived"] });
    assert_eq!(applied(&rig, more, false).await, before);
    let declared = words(&["backlog", "ready", "doing", "reviewed", "staging", "done", "archived"]);
    assert_eq!(standing(&rig).await, (declared, cards.clone()));

    // The program brings another version, which starts with a column more;
    // applied again with nothing said and taking it, as the owner did
    // (`mw apply std/board --project shop --take`): the columns are as
    // they were and in their order, the new one after the one it follows
    // in the new text, and every card still in its column. What only a
    // declaration offered and the new one does not goes.
    std::fs::remove_dir_all(at.unwrap().join("before").join("std")).unwrap();
    assert_eq!(applied(&rig, json!({}), true).await, ships);
    let now = words(&["backlog", "ready", "doing", "review", "reviewed", "staging", "done"]);
    assert_eq!(standing(&rig).await, (now.clone(), cards.clone()));

    // And applied once more as it is: nothing moves.
    assert_eq!(applied(&rig, json!({}), false).await, ships);
    assert_eq!(standing(&rig).await, (now.clone(), cards.clone()));

    // A card is still moved to a column somebody added, as a stage passes one on.
    let board = rig.read("/api/board/shop").await;
    let doing = board["items"].as_array().unwrap().iter().find(|card| card["status"] == "doing").unwrap();
    let path = format!("/api/board/items/{}", doing["id"].as_str().unwrap());
    let (status, moved) =
        rig.call(Method::PATCH, &path, Some(&rig.token), Some(json!({ "status": "reviewed" }))).await;
    assert_eq!((status, &moved["status"]), (StatusCode::OK, &json!("reviewed")), "{moved}");
}
