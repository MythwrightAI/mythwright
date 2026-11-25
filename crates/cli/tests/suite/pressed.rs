//! `board press`: what a card's column offers, pressed from the terminal
//! as whoever is signed in there, through the one door the client and
//! the factory's bot go through too.

use crate::Rig;

/// What can be pressed on a board's cards: a table that is an action's,
/// beside the board, as a fleet's `actions` is. Any blueprint may make one.
const ACTIONS: &str = r#"blueprint: signing
permissions: [projects, board, records]
steps:
  actions:
    do: collection.ensure
    with:
      name: actions
      model: action
      beside: board
      about: column
      fields:
        title: { type: text, required: true }
        name: { type: text }
        column: { type: text }
        who: { type: choice, of: [anyone, person] }
        scope: { type: text }
        holds: { type: flag }
        again: { type: flag }
        says: { type: text }
"#;

#[tokio::test]
async fn what_a_cards_column_offers_is_pressed_from_the_terminal_and_taken_back_while_it_is_where_it_was_pressed()
 {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    rig.run(&["apply", "std/board", "--project", "shop"]).await.ok();
    let file = rig.dir.join("signing.bp");
    std::fs::write(&file, ACTIONS).unwrap();
    assert!(rig.run(&["apply", file.to_str().unwrap(), "--project", "shop"]).await.ok().contains(" ok in "));
    rig.run(&["record", "add", "shop", "actions", "title=Signed off", "column=ready", "holds=true"])
        .await
        .ok();
    rig.run(&["board", "add", "shop", "Lay", "the", "floor", "--status", "ready"]).await.ok();

    // Nothing looks after the column here, so pressing it holds nobody up
    // and nobody takes the card: it is the press that is held to.
    let pressed = rig.run(&["board", "press", "shop-1", "signed-off"]).await.ok();
    assert_eq!(pressed.trim(), "pressed signed-off on shop-1: nothing holds it where it is now");
    let again = rig.run(&["board", "press", "shop-1", "Signed off"]).await.ok();
    assert_eq!(again.trim(), "Signed off stands pressed on shop-1 already");
    let board = rig.run(&["board", "shop"]).await.ok();
    assert!(board.contains("shop-1  Lay the floor  (pressed: signed-off)"), "{board}");
    let back = rig.run(&["board", "press", "shop-1", "signed-off", "--back"]).await.ok();
    assert_eq!(back.trim(), "took back signed-off on shop-1; it waits for Signed off");

    // What a column does not offer, and a card in another column, are said so.
    let none = rig.run(&["board", "press", "shop-1", "approved"]).await;
    assert!(none.code != 0 && none.err.contains("in ready it is offered Signed off"), "{}", none.err);
    rig.run(&["board", "move", "shop-1", "done"]).await.ok();
    let moved = rig.run(&["board", "press", "shop-1", "signed-off"]).await;
    assert!(moved.code != 0 && moved.err.contains("and shop-1 is in done"), "{}", moved.err);

    // A key that may only read the board presses nothing.
    let made = rig.run(&["key", "reads", "--scope", "board:read"]).await.ok();
    let token = made.lines().next().unwrap().to_string();
    rig.run(&["board", "move", "shop-1", "ready"]).await.ok();
    let refused = rig.run(&["board", "press", "shop-1", "signed-off", "--token", &token]).await;
    assert!(refused.code != 0 && refused.err.contains("board:write"), "{}", refused.err);
}
