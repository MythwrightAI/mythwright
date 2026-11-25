//! What an agent asked something in a channel is told of it
//! (`agents::docs`): what it says it is for, and the documents it keeps
//! to hand that it may read, each by what it is called and where it is,
//! ahead of what it is asked; and of one it may not read, nothing at all. And what an agent working a card
//! is told of what is attached to the card.

use serde_json::json;
use studio::docs::Doc;

use crate::{Rig, ada, own};

/// Two libraries of the project: guides anybody who reads records reads,
/// and days kept to whoever holds `private:journal`.
const LIBRARIES: &str = r#"
blueprint: libraries
permissions: [projects]
steps:
  guides:
    do: collection.ensure
    with: { name: guides, store: documents, at: guides, fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } } }
  days:
    do: collection.ensure
    with: { name: days, store: documents, at: days, reads: "private:journal", fields: { title: { type: text, required: true }, body: { type: text }, path: { type: text } } }
"#;

#[tokio::test]
async fn an_agent_asked_in_a_channel_is_told_the_documents_it_keeps_that_the_agent_may_read_and_nothing_of_the_rest()
 {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let ran = rig.run(LIBRARIES, json!({})).await;
    assert_eq!(ran.status, conductor::RunStatus::Ok, "{:?}", ran.error);
    let dir = rig.studio.projects.dir_of("shop").unwrap();
    for (path, text) in [
        ("guides/install.md", "# Installing the till\n\nThe quince goes first.\n"),
        ("days/monday.md", "# A Monday of medlars\n\nThe medlar was bletted.\n"),
        ("notes.md", "# Notes\n\nThe greengage.\n"),
    ] {
        std::fs::create_dir_all(dir.join(path).parent().unwrap()).unwrap();
        std::fs::write(dir.join(path), text).unwrap();
    }
    for doc in ["shop-guides:install", "shop-days:monday", "shop/notes.md"] {
        let doc = Doc::parse(doc).unwrap();
        assert!(rig.studio.channels.add_doc(&own(), "shop", &doc, &ada()).await.unwrap());
    }

    rig.say("hello").await;
    rig.turns_finished(1).await;
    let seen = rig.fake.seen.lock().unwrap()[0].clone();
    // Each it may read, by what it is called and where it is, with the command that reads it.
    assert!(seen.system.contains("keeps these documents to hand"), "{}", seen.system);
    assert!(
        seen.system.contains("- Installing the till: the document `shop-guides:install`"),
        "{}",
        seen.system
    );
    assert!(
        seen.system.contains(&format!("{} record show shop-guides:install", brand::CLI)),
        "{}",
        seen.system
    );
    assert!(seen.system.contains(&format!(
        "- notes.md: the file `notes.md` of `shop` (`{} file read shop notes.md`)",
        brand::CLI
    )));
    // Of the one kept from it, nothing: not where it is, not what it is called, and no word of it.
    for word in ["shop-days:monday", "Monday", "medlar", "days/"] {
        assert!(
            !seen.system.contains(word) && !seen.prompt.contains(word),
            "`{word}` was told: {}",
            seen.system
        );
    }
    // And no word of what is in the ones it may read: it reads those for itself.
    assert!(!seen.system.contains("quince") && !seen.system.contains("greengage"), "{}", seen.system);
}

#[tokio::test]
async fn an_agent_asked_in_a_channel_that_keeps_no_documents_is_told_of_none() {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("hello").await;
    rig.turns_finished(1).await;
    let seen = rig.fake.seen.lock().unwrap()[0].clone();
    assert!(!seen.system.contains("documents to hand"), "{}", seen.system);
}

#[tokio::test]
async fn an_agent_asked_in_a_channel_is_told_what_the_channel_says_it_is_for_and_nothing_where_it_says_nothing()
 {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    rig.say("hello").await;
    rig.turns_finished(1).await;
    let before = rig.fake.seen.lock().unwrap()[0].clone();
    assert!(!before.system.contains("says what it is for"), "{}", before.system);

    let said = "The till: what is sold, and what is still to be fixed.";
    rig.studio.channels.describe(&own(), "shop", said, &ada()).await.unwrap();
    rig.say("and now").await;
    rig.turns_finished(2).await;
    let seen = rig.fake.seen.lock().unwrap()[1].clone();
    assert!(
        seen.system.contains(&format!("The channel you are asked in says what it is for: {said}")),
        "{}",
        seen.system
    );
    assert!(!seen.system.contains("documents to hand"), "it keeps none: {}", seen.system);
}

#[tokio::test]
async fn an_agent_working_a_card_is_told_what_is_attached_to_it_by_name_and_size_with_the_command_and_never_what_is_in_it()
 {
    let rig = Rig::new().await;
    rig.agent("builder", "fake").await;
    let card = rig.studio.board.add("shop", "Fix the till", "", None, vec![], &ada()).await.unwrap();
    let shot = (&card.id as &str, ("till.png", Some("image/png")));
    let kept = rig.studio.records.attach(shot.0, shot.1, b"the quince".to_vec(), &ada(), None).await.unwrap();
    let run = rig
        .run(
            &format!(
                "blueprint: round\npermissions: [agents]\nsteps:\n  work: {{ do: agent.task, with: {{ slot: builder, thread: \"lane:{}\", prompt: Take the card. }} }}\n",
                card.id
            ),
            json!({}),
        )
        .await;
    assert_eq!(run.status, conductor::RunStatus::Ok, "{:?}", run.error);
    let seen = rig.fake.seen.lock().unwrap()[0].clone();
    let told = format!(
        "- till.png (image/png, 10 bytes), attached by Ada: `{} attachment {} --out <file>`",
        brand::CLI,
        kept.id
    );
    assert!(seen.system.contains(&told), "{}", seen.system);
    assert!(!seen.system.contains("quince") && !seen.prompt.contains("quince"), "{}", seen.system);
}
