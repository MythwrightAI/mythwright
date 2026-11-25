//! What a blueprint told somebody (`inbox.tell`), read from a terminal
//! with the real program: in the blueprint's own words, from the
//! factory's bot, by whoever it was told.

use crate::Rig;

/// A text that tells whoever applied it, about what it is handed.
const NIGHTLY: &str = r#"blueprint: nightly
permissions: [inbox]
params:
  about: { type: string, default: "" }
steps:
  tell:
    do: inbox.tell
    with: { to: applier, says: "The night's build is green.", why: built, about: "${params.about}" }
"#;

#[tokio::test(flavor = "multi_thread")]
async fn what_a_blueprint_told_somebody_is_in_their_inbox_from_a_terminal_in_its_own_words() {
    let rig = Rig::signed_in().await;
    let file = rig.dir.join(brand::blueprint_file("nightly"));
    std::fs::write(&file, NIGHTLY).unwrap();
    let file = file.to_str().unwrap();

    // Told about something, the line says what; and told again of the
    // same thing by another run, it is the one line, counted.
    for key in ["first", "second"] {
        rig.run(&["apply", file, "--key", key, "--param", "about=in_build"]).await.ok();
    }
    // About nothing, it says its words and no more.
    rig.run(&["apply", file, "--key", "third"]).await.ok();
    let mine = rig.run(&["inbox"]).await.ok();
    let lines: Vec<&str> = mine.lines().collect();
    assert_eq!(lines.len(), 3, "{mine}");
    assert!(lines[0].ends_with("  unread  [conductor] The night's build is green."), "{mine}");
    assert!(
        lines[1].ends_with("  unread  [conductor] The night's build is green.  (about in_build) (2 times)"),
        "{mine}"
    );
    assert_eq!(lines[2], "2 unread");
}
