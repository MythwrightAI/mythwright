//! A forum's topics from a terminal (`topic`): made, listed, replied to
//! under one another, answered, moved and followed, each named by its
//! title whatever its capitals.

use crate::Rig;

#[tokio::test(flavor = "multi_thread")]
async fn a_topic_is_made_replied_to_under_a_reply_answered_and_moved_from_a_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let made = rig
        .run(&[
            "topic",
            "make",
            "Why",
            "the",
            "build",
            "is",
            "slow",
            "--body",
            "It takes minutes.",
            "--status",
            "open",
        ])
        .await
        .ok();
    assert!(
        made.contains("made @-topics-1") && made.contains("open") && made.contains("Why the build is slow"),
        "{made}"
    );

    let first = rig
        .run(&["topic", "reply", "why the BUILD is slow", "Linking", "is", "most", "of", "it."])
        .await
        .ok();
    let seq = first.trim().strip_prefix("said at ").expect(&first).to_string();
    rig.run(&["topic", "reply", "@-topics-1", "Which", "linker?", "--to", &seq]).await.ok();
    let shown = rig.run(&["topic", "show", "Why the build is slow"]).await.ok();
    assert!(shown.contains("It takes minutes."), "{shown}");
    assert!(shown.contains(&format!("{seq}  [")) && shown.contains("] Linking is most of it."), "{shown}");
    assert!(
        shown.contains("\n  ") && shown.contains("] Which linker?"),
        "the second is under the first: {shown}"
    );

    let answered = rig.run(&["topic", "answer", "Why the build is slow", &seq]).await.ok();
    assert!(answered.contains("answered"), "{answered}");
    let moved = rig.run(&["topic", "move", "Why the build is slow", "--project", "shop"]).await.ok();
    assert!(moved.contains("(project shop"), "{moved}");
    let followed = rig.run(&["topic", "follow", "Why the build is slow"]).await.ok();
    assert!(followed.contains("you follow it"), "{followed}");
    let listed = rig.run(&["topic", "list"]).await.ok();
    assert!(listed.contains("answered") && listed.contains("2 replies"), "{listed}");

    // What a topic says is changed as any record is; where it is, not so.
    rig.run(&["record", "set", "@-topics-1", "summary=Mostly linking."]).await.ok();
    let refused = rig.run(&["record", "set", "@-topics-1", "project=elsewhere"]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("`topic move`"), "{}", refused.err);
    let none = rig.run(&["topic", "show", "Nothing like it"]).await;
    assert!(none.err.contains("there is no topic `Nothing like it`"), "{}", none.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_is_made_under_another_moved_under_one_to_the_top_and_shown_where_it_is_from_a_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["topic", "make", "Handbook"]).await.ok();
    let made = rig.run(&["topic", "make", "Deploying", "--under", "handbook"]).await.ok();
    assert!(made.contains("made @-topics-2") && made.contains("Deploying"), "{made}");
    rig.run(&["topic", "make", "Getting", "started", "--under", "Handbook"]).await.ok();
    rig.run(&["topic", "make", "Rolling", "back", "--under", "Deploying"]).await.ok();

    let shown = rig.run(&["topic", "show", "Rolling back"]).await.ok();
    assert!(shown.starts_with("in Handbook › Deploying\n"), "{shown}");
    let held = rig.run(&["topic", "show", "Handbook"]).await.ok();
    assert!(held.contains("pages under it (2):"), "{held}");
    assert!(held.find("Deploying").unwrap() < held.find("Getting started").unwrap(), "{held}");

    let moved = rig
        .run(&["topic", "move", "Getting started", "--under", "Handbook", "--before", "Deploying"])
        .await
        .ok();
    assert!(moved.starts_with("under @-topics-1: "), "{moved}");
    let held = rig.run(&["topic", "show", "Handbook"]).await.ok();
    assert!(held.find("Getting started").unwrap() < held.find("Deploying").unwrap(), "{held}");
    let top = rig.run(&["topic", "move", "Deploying", "--top"]).await.ok();
    assert!(top.starts_with("a wiki of its own: "), "{top}");
    let shown = rig.run(&["topic", "show", "Rolling back"]).await.ok();
    assert!(shown.starts_with("in Deploying\n"), "its page went with it: {shown}");

    let itself = rig.run(&["topic", "move", "Deploying", "--under", "Rolling back"]).await;
    assert_ne!(itself.code, 0);
    assert!(itself.err.contains("cannot go under itself or a page that is under it"), "{}", itself.err);
    let both = rig.run(&["topic", "move", "Deploying", "--under", "Handbook", "--top"]).await;
    assert!(both.err.contains("not both"), "{}", both.err);
    rig.run(&["project", "create", "--bare", "Shop"]).await.ok();
    let page = rig.run(&["topic", "move", "Rolling back", "--project", "shop"]).await;
    assert!(page.err.contains("`Rolling back` is a page of the wiki `Deploying`"), "{}", page.err);
    let wiki = rig.run(&["topic", "move", "Deploying", "--project", "shop"]).await.ok();
    assert!(wiki.contains("(project shop"), "a wiki is put there with its pages: {wiki}");
    let shown = rig.run(&["topic", "show", "Rolling back"]).await.ok();
    assert!(shown.contains("(project shop"), "{shown}");
    let refused = rig.run(&["record", "set", "@-topics-4", "parent=@-topics-1"]).await;
    assert!(refused.err.contains("`topic make --under`"), "{}", refused.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_taken_away_from_a_terminal_takes_the_pages_under_it_into_the_trash_and_back() {
    let rig = Rig::signed_in().await;
    rig.run(&["topic", "make", "Handbook"]).await.ok();
    rig.run(&["topic", "make", "Deploying", "--under", "Handbook"]).await.ok();
    rig.run(&["topic", "make", "Rolling", "back", "--under", "Deploying"]).await.ok();
    let gone = rig.run(&["record", "remove", "@-topics-2"]).await.ok();
    assert!(gone.contains("took @-topics-2 away, with the record under it"), "{gone}");
    let held = rig.run(&["topic", "show", "Handbook"]).await.ok();
    assert!(!held.contains("pages under it"), "{held}");
    let none = rig.run(&["topic", "show", "Rolling back"]).await;
    assert_ne!(none.code, 0, "it went with its page: {}", none.out);
    let listed = rig.run(&["trash"]).await.ok();
    let entry = listed.split_whitespace().next().unwrap().to_string();
    assert!(listed.contains("`Deploying` of @, with the record under it"), "{listed}");
    let back = rig.run(&["trash", "restore", &entry]).await.ok();
    assert!(back.contains("with the record that went with it"), "{back}");
    let shown = rig.run(&["topic", "show", "Rolling back"]).await.ok();
    assert!(shown.starts_with("in Handbook › Deploying\n"), "{shown}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_topic_made_from_a_terminal_has_no_status_until_one_is_said_and_loses_it_when_cleared() {
    let rig = Rig::signed_in().await;
    let made = rig.run(&["topic", "make", "Handbook"]).await.ok();
    // Its status is a blank column: the title is where it is on every line.
    assert!(
        made.contains("made @-topics-1  ") && made.contains(&format!("{} Handbook", " ".repeat(9))),
        "{made}"
    );
    assert!(!made.contains("open"), "{made}");
    let shown = rig.run(&["topic", "show", "Handbook"]).await.ok();
    assert!(shown.contains(&format!("@-topics-1  {} Handbook", " ".repeat(9))), "{shown}");
    let said = rig.run(&["topic", "reply", "Handbook", "Start", "here."]).await.ok();
    let seq = said.trim().strip_prefix("said at ").expect(&said).to_string();
    let refused = rig.run(&["topic", "answer", "Handbook", &seq]).await;
    assert_ne!(refused.code, 0);
    assert!(refused.err.contains("not tracked as a question"), "{}", refused.err);

    let asked = rig.run(&["topic", "make", "Which", "store", "--status", "open"]).await.ok();
    assert!(asked.contains("open      Which store"), "{asked}");
    let standing = rig.run(&["topic", "make", "Naming", "branches", "--status", "standing"]).await.ok();
    assert!(standing.contains("standing  Naming branches"), "{standing}");
    let wrong = rig.run(&["topic", "make", "Nothing", "--status", "answered"]).await;
    assert!(wrong.err.contains("`open` or `standing`"), "{}", wrong.err);

    rig.run(&["record", "set", "@-topics-2", "status="]).await.ok();
    let listed = rig.run(&["topic", "list"]).await.ok();
    let line = listed.lines().find(|line| line.contains("Which store")).unwrap();
    assert!(line.starts_with(&format!("@-topics-2  {} Which store", " ".repeat(9))), "cleared: {listed}");
    assert!(listed.contains("standing  Naming branches"), "{listed}");
}
