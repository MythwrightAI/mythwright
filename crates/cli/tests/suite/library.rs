//! A library (`std/docs`) from the terminal: a document is a record.

use crate::Rig;

#[tokio::test]
async fn a_librarys_documents_are_listed_a_line_each_written_and_spoken_on_from_the_terminal() {
    let rig = Rig::signed_in().await;
    let made = rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    assert!(made.contains("shop"), "{made}");
    let applied = rig.run(&["apply", "std/docs", "--project", "shop"]).await.ok();
    assert!(applied.contains(" ok in "), "{applied}");

    let long = "It begins here and goes on for a good deal longer than a line of a list has room for.";
    let body = format!("body={long}\n\nAnd there is more of it under that.");
    let added = rig
        .run(&["record", "add", "shop", "docs", "title=Release notes", "path=guides/releases", &body])
        .await;
    assert_eq!(added.ok().trim(), "added shop-docs:guides:releases to docs");

    // A line a document: of its text, how it begins.
    let listed = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    let line = listed.lines().find(|line| line.starts_with("shop-docs:guides:releases")).expect(&listed);
    assert!(line.contains("body It begins here and goes on for a good deal longer than a lin…"), "{line}");
    assert!(line.contains("path guides/releases.md") && line.contains("title Release notes"), "{line}");
    assert!(!listed.contains("And there is more"), "{listed}");
    // The whole of it is what `show` is for.
    let shown = rig.run(&["record", "show", "shop-docs:guides:releases"]).await.ok();
    assert!(shown.contains("And there is more of it under that."), "{shown}");

    let tagged = rig.run(&["record", "set", "shop-docs:guides:releases", "tags=news,first"]).await.ok();
    assert!(tagged.contains("shop-docs:guides:releases"), "{tagged}");
    let said =
        rig.run(&["record", "comment", "shop-docs:guides:releases", "Is", "this", "the", "place?"]).await;
    assert!(said.ok().contains("shop-docs:guides:releases"));
    let gone = rig.run(&["record", "remove", "shop-docs:guides:releases"]).await.ok();
    assert!(gone.contains("shop-docs:guides:releases"), "{gone}");
    let listed = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    assert!(!listed.contains("shop-docs:guides:releases"), "{listed}");
}
