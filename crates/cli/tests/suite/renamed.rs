//! A channel called something else from the terminal: an open one by
//! its name, one kept to its members by what it is called, and a name
//! another has refused in the daemon's sentence; what one says it is
//! for, and the last of its managers staying.

use crate::Rig;

#[tokio::test]
async fn a_channel_is_renamed_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "general"]).await.ok();
    rig.run(&["channel", "create", "elsewhere"]).await.ok();
    rig.run(&["post", "general", "the quince ledger"]).await.ok();

    let renamed = rig.run(&["channel", "rename", "#general", "The Lobby"]).await.ok();
    assert_eq!(renamed.trim_end(), "it is called #the-lobby now");
    assert!(rig.run(&["read", "the-lobby"]).await.ok().contains("the quince ledger"));
    let refused = rig.run(&["channel", "rename", "the-lobby", "elsewhere"]).await;
    assert!(
        refused.code != 0 && refused.err.contains("there is already a channel called `elsewhere`"),
        "{}",
        refused.err
    );

    rig.run(&["channel", "create", "Pay review", "--kept"]).await.ok();
    let kept = rig.run(&["channel", "rename", "Pay review", "Salary review"]).await.ok();
    assert!(kept.starts_with("it is called `Salary review` (kept to its members: ch_"), "{kept}");
    // What it says it is for, and who manages it: whoever made it, here.
    let described = rig.run(&["channel", "describe", "the-lobby", "where", "the", "fruit", "is"]).await.ok();
    assert_eq!(described.trim_end(), "#the-lobby says it is for: where the fruit is");
    let emptied = rig.run(&["channel", "describe", "the-lobby"]).await.ok();
    assert_eq!(emptied.trim_end(), "#the-lobby says nothing of what it is for now");
    let last = rig.run(&["channel", "manager", "the-lobby", "Ada", "--not"]).await;
    assert!(last.code != 0 && last.err.contains("last manager stays"), "{}", last.err);
    let told = rig.run(&["help", "channel"]).await.ok();
    assert!(told.contains("channel rename <channel> <name>"), "{told}");
    assert!(told.contains("channel manager <channel> <who> [--not]"), "{told}");
}
