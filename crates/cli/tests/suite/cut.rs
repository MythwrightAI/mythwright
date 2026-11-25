//! A list the daemon cuts at so many says so where it is printed: what
//! is alive, and what happened. Read as all there is, a cut list was taken
//! for the whole of something (`events --limit 5000` printed
//! a thousand, and `instances` two hundred, with nothing said).

use kernel::Source;
use serde_json::{Value, json};

use crate::Rig;

const REST: &str = "blueprint: rest
steps:
  done: { do: emit, with: { topic: shop.rested } }
";

#[tokio::test]
async fn what_ran_listed_up_to_so_many_says_that_there_are_more() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let file = rig.dir.join(brand::blueprint_file("rest"));
    std::fs::write(&file, REST).unwrap();
    for key in ["one", "two", "three"] {
        rig.run(&["apply", file.to_str().unwrap(), "--key", key, "--project", "shop"]).await.ok();
    }

    let cut = rig.run(&["instances", "--all", "--project", "shop", "--limit", "2"]).await;
    assert_eq!(cut.code, 0, "{}", cut.err);
    assert_eq!(cut.out.lines().filter(|line| line.contains("rest")).count(), 2, "{}", cut.out);
    assert!(cut.err.contains("There are more than these 2, the newest"), "{}", cut.err);
    // As JSON what is printed is the list alone, saying there are more.
    let json: Value = serde_json::from_str(
        &rig.run(&["instances", "--all", "--project", "shop", "--limit", "2", "--json"]).await.ok(),
    )
    .unwrap();
    assert_eq!(
        (json["instances"].as_array().map(Vec::len), &json["more"]),
        (Some(2), &json!(true)),
        "{json}"
    );

    let whole = rig.run(&["instances", "--all", "--project", "shop", "--limit", "3"]).await;
    assert_eq!(whole.out.lines().filter(|line| line.contains("rest")).count(), 3, "{}", whole.out);
    assert!(whole.err.is_empty(), "nothing is said of a list that is whole: {}", whole.err);
}

#[tokio::test]
async fn what_happened_asked_for_beyond_what_is_handed_at_once_says_so() {
    let rig = Rig::signed_in().await;
    let journal = &rig.node.as_ref().unwrap().app.journal;
    for n in 0..1001 {
        journal.emit("shop.said", Source::default(), json!({ "n": n })).await.unwrap();
    }

    let cut = rig.run(&["events", "--topics", "shop.**", "--limit", "5000"]).await;
    assert_eq!(cut.code, 0, "{}", cut.err);
    assert_eq!(cut.out.lines().count(), 1000, "the daemon hands a thousand at once");
    assert!(cut.err.contains("The daemon hands at most 1000 events at once"), "{}", cut.err);

    // Asked for fewer than there are, the list is as long as was asked and says nothing more.
    let asked = rig.run(&["events", "--topics", "shop.**", "--limit", "20"]).await;
    assert_eq!(asked.out.lines().count(), 20, "{}", asked.out);
    assert!(asked.err.is_empty(), "{}", asked.err);
}
