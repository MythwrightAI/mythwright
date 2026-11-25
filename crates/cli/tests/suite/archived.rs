//! A channel archived and given back, from the terminal: read as before,
//! said nothing in, listed as archived, and back as it was.

use crate::Rig;

#[tokio::test]
async fn a_channel_is_archived_and_given_back_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "general"]).await.ok();
    rig.run(&["post", "general", "the quince ledger"]).await.ok();

    let archived = rig.run(&["channel", "archive", "#general"]).await.ok();
    assert!(archived.starts_with("#general is archived: whoever read it reads it"), "{archived}");
    assert!(archived.contains("channel unarchive general"), "{archived}");
    assert!(rig.run(&["channel", "list"]).await.ok().contains("#general (archived)"));
    assert!(rig.run(&["read", "general"]).await.ok().contains("the quince ledger"));
    let refused = rig.run(&["post", "general", "the medlar ledger"]).await;
    assert!(refused.code != 0 && refused.err.contains("#general is archived"), "{}", refused.err);
    assert!(refused.err.contains("channel unarchive general"), "{}", refused.err);
    // Archived again, it is as it was.
    assert!(rig.run(&["channel", "archive", "general"]).await.ok().contains("is archived"));

    let back = rig.run(&["channel", "unarchive", "general"]).await.ok();
    assert_eq!(back.trim_end(), "#general is back as it was");
    assert!(!rig.run(&["channel", "list"]).await.ok().contains("(archived)"));
    rig.run(&["post", "general", "the medlar ledger"]).await.ok();
    assert!(rig.run(&["read", "general"]).await.ok().contains("the medlar ledger"));
    let told = rig.run(&["help", "channel"]).await.ok();
    assert!(told.contains("channel archive <channel> | channel unarchive <channel>"), "{told}");
}
