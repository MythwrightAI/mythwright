//! What came of a card's work after it was merged, counted under the way
//! of a comparison the card went: its work reverted by another card's
//! merge, its files changed again soon after by another's, and a later
//! card that says it fixes it. Each is said at the later card's merge
//! (`std/merge`, asking `place.follows`), of the card it is about, and the
//! usual counts of a board's cards count it under that card's way, once
//! for each card that did it. They are counts: nothing says which way did
//! better.

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{rig, shop};
use crate::lanes::{git, history, move_card, moved};

/// Two who work, each a way of the comparison, and one who reviews.
fn agents() -> Value {
    json!([
        { "title": "big", "role": "worker", "count": 1, "adapter": "writer" },
        { "title": "small", "role": "worker", "count": 1, "adapter": "writer" },
        { "title": "reviewer", "role": "reviewer", "count": 1, "adapter": "critic", "permission": "read" },
    ])
}

/// A card added with `fields`: its id.
async fn add(rig: &Rig, title: &str, fields: Value) -> String {
    let card = rig.send("/api/board/shop/items", json!({ "title": title, "fields": fields })).await;
    card["id"].as_str().unwrap().to_string()
}

/// The card worked by the fleet, reviewed, merged and done.
async fn done(rig: &Rig, id: &str) {
    move_card(rig, id, "ready", None).await;
    moved(rig, id, "done", 0).await;
}

/// The row of what came after a merge (`after <what>`) as `compare show`
/// lays it out, once the cell of `column` counts `count`.
async fn after(rig: &Rig, what: &str, column: usize, count: u64) -> Value {
    let what = format!("after {what}");
    let mut seen = Value::Null;
    crate::soon(&format!("`{what}` to count {count}"), async {
        loop {
            let shown = rig.read("/api/comparisons/models").await;
            let rows = shown["beside"]["rows"].as_array().cloned().unwrap_or_default();
            let row = rows.iter().find(|row| row["kind"] == "came" && row["what"] == what.as_str());
            if let Some(row) = row
                && row["by"][column]["count"] == count
            {
                return row.clone();
            }
            // What was counted meanwhile, for whoever reads why it never came to that.
            let came: Vec<&Value> = rows.iter().filter(|row| row["kind"] == "came").collect();
            if json!(came) != seen {
                seen = json!(came);
                println!("{what}: {seen}");
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

#[tokio::test]
async fn what_came_after_a_cards_work_was_merged_is_counted_under_its_way_once_for_each_card_that_did_it() {
    let (rig, _at) = rig().await;
    let dir = shop(&rig, json!({ "agents": agents() })).await;
    let asked = json!({
        "name": "models", "project": "shop", "of": "std/lane-round", "per": "item", "also": ["std/review"],
        "counts": "cards",
        "ways": [
            { "name": "big", "with": { "agent": "shop-big-1" } },
            { "name": "small", "with": { "agent": "shop-small-1" } },
        ],
    });
    let started = rig.send("/api/comparisons", asked).await;
    let afters: Vec<&Value> =
        started["counts"].as_array().unwrap().iter().filter(|count| count["name"] == "after").collect();
    assert_eq!(afters.len(), 3, "{started}");
    assert!(
        afters
            .iter()
            .all(|count| count["once"] == json!(["by"])
                && count["thing"].as_str().unwrap().starts_with("card")),
        "{started}"
    );

    // A card worked under a way and merged: its merge says which card it merged, in a last line of its own.
    let cart = add(&rig, "Add a cart", json!({})).await;
    done(&rig, &cart).await;
    let merge = git(&dir, &["log", "--first-parent", "--merges", "--format=%H", "-1"]);
    let said = git(&dir, &["log", "--no-walk", "--format=%B", &merge]);
    assert!(
        said.starts_with(&format!("{cart}: Add a cart")) && said.ends_with(&format!("Item: {cart}")),
        "{said}"
    );
    let way = if history(&rig, &cart, "conductor moved it from doing to review")
        .await
        .iter()
        .any(|line| line == "shop-big-1 worked on it")
    {
        "big"
    } else {
        "small"
    };
    let (column, other) = if way == "big" { (0, 1) } else { (1, 0) };

    // A card whose work is a revert of the first's, made by a person on its branch and put in review:
    // reviewed, merged, and the first said to have been reverted by it (and its files changed again).
    let undo = add(&rig, "Undo the cart", json!({})).await;
    let place = format!(".lanes/{undo}");
    git(&dir, &["worktree", "add", "--quiet", "-b", &format!("lane/{undo}"), &place]);
    git(&dir.join(&place), &["revert", "--no-edit", "-m", "1", &merge]);
    move_card(&rig, &undo, "review", None).await;
    moved(&rig, &undo, "done", 0).await;
    history(&rig, &cart, &format!("{undo} reverted its work")).await;

    // A card that says it fixes the first, worked by the fleet: changing the same file again, and
    // said to fix it.
    let fix = add(&rig, "Fix the cart", json!({ "fixes": [cart] })).await;
    done(&rig, &fix).await;
    history(&rig, &cart, &format!("{fix} says it fixes it")).await;

    // Its files changed again is said once at each merge, of every card whose files that is.
    let said = rig
        .event("card.changed_again", {
            let fix = fix.clone();
            move |event| event.data["by"] == fix.as_str()
        })
        .await;
    assert_eq!(said.data["cards"], json!([undo, cart]), "{:?}", said.data);

    // Counted under the first card's way, once for each card that did it: reverted once, its files
    // changed again by both, fixed once; and nothing of it under the other way.
    let reverted = after(&rig, "reverted", column, 1).await;
    let again = after(&rig, "changed_again", column, 2).await;
    let fixed = after(&rig, "fixed", column, 1).await;
    for row in [&reverted, &again, &fixed] {
        assert_eq!(row["by"][other]["count"].as_u64().unwrap_or(0), 0, "{row}");
    }
    let shown = rig.read("/api/comparisons/models").await;
    let laid = shown["beside"].to_string();
    assert!(!["better", "best", "score", "winner"].iter().any(|word| laid.contains(word)), "{laid}");
}
