//! A conversation from a terminal, with the real program: a person
//! writes one with `to`, an agent the factory does not start finds it
//! in its box asked as with `--as`, reads it and answers by its id, and
//! one of them opens it as a channel.

use crate::external::{handed, invitation};
use crate::{Rig, run_in};

/// The id of the conversation a listing of a box names in the line
/// that says `words`.
fn conversation_of(listed: &str, words: &str) -> String {
    let line = listed.lines().find(|line| line.contains(words));
    let line = line.unwrap_or_else(|| panic!("no line says `{words}`:\n{listed}"));
    let from = line.find("(ch_").unwrap_or_else(|| panic!("the line names no conversation: {line}"));
    line[from + 1..].split(';').next().unwrap().to_string()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_person_writes_to_an_agent_from_outside_which_reads_its_box_and_the_conversation_and_answers() {
    let rig = Rig::signed_in().await;
    let cli = brand::CLI;
    rig.run(&["channel", "create", "general"]).await.ok();
    let invited = invitation(&rig.run(&["agent", "invite", "walls"]).await.ok());
    handed(&rig.dir, &rig.url, &["agent", "join"], &invited).await.ok();
    let as_walls =
        async |args: &[&str]| run_in(&rig.dir, &rig.url, &[], &[&["--as", "walls"], args].concat()).await;

    // Written to the agent by its handle: nothing is started for it and
    // nobody answers; whoever wrote it is told how it is read.
    let wrote = rig.run(&["to", "@walls", "--subject", "The gate", "who", "hangs", "it?"]).await.ok();
    assert!(wrote.starts_with("wrote `The gate` to Ada, walls"), "{wrote}");
    assert!(wrote.contains(&format!("{cli} read ch_")), "{wrote}");
    // It is in the agent's box, with what it is about, who is in it and
    // the id it is read and answered by; and in no list of channels.
    let theirs = as_walls(&["inbox"]).await.ok();
    assert!(theirs.contains("unread  conversation `The gate` (ch_"), "{theirs}");
    assert!(theirs.contains("; with Ada, walls)  [Ada] who hangs it?"), "{theirs}");
    assert!(theirs.ends_with("1 unread\n"), "{theirs}");
    let id = conversation_of(&theirs, "who hangs it?");
    for listed in [rig.run(&["channel", "list"]).await.ok(), as_walls(&["channel", "list"]).await.ok()] {
        assert!(
            listed.contains("#general") && !listed.contains("The gate") && !listed.contains(&id),
            "{listed}"
        );
    }
    assert!(rig.run(&["inbox"]).await.ok().starts_with("nothing waits in your inbox"));

    // It reads the conversation and answers there, as itself, by its id.
    let read = as_walls(&["read", &id]).await.ok();
    assert_eq!(read.trim(), "[Ada] who hangs it?");
    as_walls(&["post", &id, "I", "do,", "on", "Tuesday"]).await.ok();
    let mine = rig.run(&["inbox"]).await.ok();
    assert!(
        mine.contains(&format!(
            "unread  conversation `The gate` ({id}; with Ada, walls)  [walls] I do, on Tuesday"
        )),
        "{mine}"
    );
    assert_eq!(rig.run(&["read", &id]).await.ok().trim(), "[Ada] who hangs it?\n[walls] I do, on Tuesday");
    // And writes one itself, with no subject: called by its first words.
    let back = as_walls(&["to", "ada", "the", "latch", "is", "on", "too"]).await.ok();
    assert!(back.starts_with("wrote `the latch is on too` to walls, Ada"), "{back}");
    assert!(rig.run(&["inbox"]).await.ok().ends_with("2 unread\n"));

    // What is asked wrongly says what to ask, and nothing is written.
    let nobody = rig.run(&["to", "nobody", "hello"]).await;
    assert!(nobody.code != 0 && nobody.err.contains("nobody here is called `@nobody`"), "{}", nobody.err);
    let nothing = rig.run(&["to", "walls"]).await;
    assert!(nothing.code != 0 && nothing.err.contains("missing what is said"), "{}", nothing.err);
    assert!(as_walls(&["inbox"]).await.ok().ends_with("1 unread\n"), "nothing more is in its box");

    // Opened as a channel by one of them: listed among the channels of
    // whoever is in it from then, by what it is called and its id.
    let opened = rig.run(&["channel", "from", &id, "--name", "Gate works"]).await.ok();
    assert!(
        opened.contains("`Gate works` (kept to its members: ") && opened.contains("is a channel from here"),
        "{opened}"
    );
    for listed in [rig.run(&["channel", "list"]).await.ok(), as_walls(&["channel", "list"]).await.ok()] {
        assert!(listed.contains(&format!("`Gate works` (kept to its members: {id})")), "{listed}");
    }
    let again = rig.run(&["channel", "from", &id]).await;
    assert!(again.code != 0 && again.err.contains("is no conversation"), "{}", again.err);

    // And the program says what it is.
    let help = rig.run(&["help", "to"]).await.ok();
    assert!(help.contains(&format!("{cli} to <who>[,<who>...] [--subject S] <words...>")), "{help}");
    assert!(help.contains(&format!("`{cli} read <id>` and `{cli} post <id> <words...>`")), "{help}");
    let channels = rig.run(&["help", "channel"]).await.ok();
    assert!(
        channels.contains("channel from <conversation>") && channels.contains("by\nits id"),
        "{channels}"
    );
    assert!(rig.run(&["help"]).await.ok().contains("| to <who>[,<who>...] [--subject S] <words...>"));
}
