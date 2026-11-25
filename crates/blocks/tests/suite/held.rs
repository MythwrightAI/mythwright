//! The words a running tree links: the ones it reached when somebody
//! applied it, as the blueprints it names are, until somebody says to
//! take them as they now are. What holds a tree to its texts is the
//! runner's; `text.put` is what reads through it.

use conductor::{Apply, Moved};
use kernel::Source;
use serde_json::json;

use crate::handlers::{active, handled};
use crate::{Rig, user};

async fn moved(rig: &Rig, id: u64) {
    rig.journal.emit("board.item.moved", Source::default(), json!({ "id": id })).await.unwrap();
}

/// Says what the file it links says of each thing that is made ready.
const ASKS: &str = "\
blueprint: asks
on:
  ready:
    event: board.item.moved
    steps:
      words: { do: text.put, with: { parts: [{ from: ask.md, with: { item: \"${ event.data.id }\" } }] } }
      say: { do: emit, with: { topic: shop.told, text: \"${ steps.words.out.text }\" } }
";

/// The words as they shipped when it was applied, and as a later program
/// ships them: with a blank the text of before never hands.
const WORDS: &str = "Look at ${item}.\n";
const LATER_WORDS: &str = "Look hard at ${item} in ${place}.\n";

/// What it says of the thing `id`.
async fn told(rig: &Rig, id: u64) -> String {
    let before = rig.mark();
    moved(rig, id).await;
    let said = rig.event("shop.told", move |event| event.seq > before).await;
    said.data["text"].as_str().unwrap().to_string()
}

/// The words as they are said to have moved: the file's name, and what is said of it.
fn words(moved: &[Moved]) -> Vec<(&str, Vec<&str>)> {
    assert!(moved.iter().all(|one| one.was.is_none() && one.now.is_none()), "words say no version");
    moved.iter().map(|one| (one.name.as_str(), one.changed.iter().map(String::as_str).collect())).collect()
}

/// It applied while `WORDS` is what ships.
async fn asking(rig: &Rig) {
    rig.library.ship();
    rig.library.put("asks", ASKS);
    rig.library.put("prompts/ask.md", WORDS);
    let applied = rig.apply(ASKS, json!({})).await;
    active(rig, &applied.instance).await;
}

#[tokio::test]
async fn a_tree_goes_on_with_the_words_it_reached_and_they_are_taken_and_put_back_as_blueprints_are() {
    let mut rig = Rig::new().await;
    asking(&rig).await;
    assert_eq!(told(&rig, 1).await, "Look at 1.");

    // A new program: under the file's name are words the text was never written for. It goes on
    // with the ones it reached, across a restart too, and nothing fails for it.
    rig.library.put("prompts/ask.md", LATER_WORDS);
    assert_eq!(told(&rig, 2).await, "Look at 2.");
    // Stopped once its handler has run for both, or the second is heard again when it is back.
    handled(&rig, "on.ready", 2).await;
    rig.restart().await;
    assert_eq!(told(&rig, 3).await, "Look at 3.");
    assert!(rig.events("run.failed").await.is_empty());

    // Applied again as it is: nothing moves, and whoever asked is told that the words changed and
    // which blank is new. Words say no version, and a prompt that changed is no change to its blueprint.
    let again = rig.engine.apply(rig.request(ASKS, json!({}))).await.unwrap();
    let changed = vec![("prompts/ask.md", vec!["its words changed", "the blank `${place}` is new"])];
    assert!(!again.changed && again.reached.taken.is_empty());
    assert_eq!(words(&again.reached.left), changed);
    assert_eq!(told(&rig, 4).await, "Look at 4.");

    // Told to take the texts as they now are, it is held to them first: the text of before hands
    // nothing for the new blank, so it is refused saying so, and nothing has moved.
    let taking = |source: &str| Apply { take: true, ..rig.request(source, json!({})) };
    let refused = rig.engine.apply(taking(ASKS)).await.unwrap_err();
    assert!(refused.message.contains("its blank `${place}` is handed nothing"), "{refused}");
    assert_eq!(told(&rig, 5).await, "Look at 5.");

    // A text written for the words of now, applied taking them, moves, and says what to.
    let fitted =
        ASKS.replace("item: \"${ event.data.id }\" }", "item: \"${ event.data.id }\", place: shop }");
    assert_ne!(fitted, ASKS);
    let taken = rig.engine.apply(taking(&fitted)).await.unwrap();
    assert_eq!((words(&taken.reached.taken), taken.reached.left.len()), (changed, 0));
    assert_eq!(told(&rig, 6).await, "Look hard at 6 in shop.");
    let after = rig.engine.apply(rig.request(&fitted, json!({}))).await.unwrap();
    assert!(after.reached.is_empty(), "{:?}", after.reached);

    // Put back, it is on the words it had, though the program still ships the later ones. What
    // would be put back is said first, the way a move is.
    let gone = vec![("prompts/ask.md", vec!["its words changed", "the blank `${place}` is gone"])];
    let would = rig.engine.put_back(rig.request(&fitted, json!({})), true).await.unwrap();
    assert_eq!(words(&would.reached.taken), gone);
    assert_eq!(told(&rig, 7).await, "Look hard at 7 in shop.");
    let back = rig.engine.put_back(rig.request(&fitted, json!({})), false).await.unwrap();
    assert_eq!(words(&back.reached.taken), gone);
    assert_eq!(told(&rig, 8).await, "Look at 8.");
    assert!(rig.events("run.failed").await.is_empty());
}

#[tokio::test]
async fn words_a_tree_is_held_to_that_its_text_can_no_longer_fill_give_way_to_the_ones_there_now() {
    let rig = Rig::new().await;
    asking(&rig).await;
    assert_eq!(told(&rig, 1).await, "Look at 1.");
    // The text is written again to hand something else, with words to match, and applied as it
    // is: the words of before have a blank nothing fills now, so they are not gone on with.
    rig.library.put("prompts/ask.md", "See to ${thing}.\n");
    let other = ASKS.replace("item: \"${ event.data.id }\"", "thing: \"${ event.data.id }\"");
    assert_ne!(other, ASKS);
    rig.engine.apply(rig.request(&other, json!({}))).await.unwrap();
    assert_eq!(told(&rig, 2).await, "See to 2.");
    assert!(rig.events("run.failed").await.is_empty());
}

#[tokio::test]
async fn words_reached_only_as_a_step_ran_are_held_from_then_and_none_are_held_where_texts_are_followed() {
    let rig = Rig::new().await;
    rig.library.ship();
    // Which file is linked is worked out as the step runs: nothing was reached when it was applied.
    let by_name = ASKS.replace("from: ask.md", "from: \"${ event.data.which }.md\"");
    assert_ne!(by_name, ASKS);
    rig.library.put("prompts/ask.md", WORDS);
    let applied = rig.apply(&by_name, json!({})).await;
    active(&rig, &applied.instance).await;
    let told = async |id: u64| {
        let before = rig.mark();
        let made = json!({ "id": id, "which": "ask" });
        rig.journal.emit("board.item.moved", Source::default(), made).await.unwrap();
        let said = rig.event("shop.told", move |event| event.seq > before).await;
        said.data["text"].as_str().unwrap().to_string()
    };
    assert_eq!(told(1).await, "Look at 1.");
    rig.library.put("prompts/ask.md", "Look twice at ${item}.\n");
    assert_eq!(told(2).await, "Look at 2.", "held from when it was first read");

    // Where the owner has said texts are followed, words are read as they are, like every text.
    let follow = conductor::taking::Take { come: Some("follow".into()), land: None };
    rig.engine.set_taking(follow, &user()).await.unwrap();
    assert_eq!(told(3).await, "Look twice at 3.");
    rig.library.put("prompts/ask.md", "Look thrice at ${item}.\n");
    assert_eq!(told(4).await, "Look thrice at 4.");
}
