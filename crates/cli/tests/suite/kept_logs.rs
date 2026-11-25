//! `logs --keep` and `logs --folder`: how long the detail of runs' steps
//! is kept and where it is written, said and changed from the terminal.

use crate::Rig;

#[tokio::test(flavor = "multi_thread")]
async fn the_terminal_says_how_long_logs_are_kept_and_where_and_changes_each() {
    let rig = Rig::signed_in().await;
    let usual = rig.run(&["logs"]).await.ok();
    assert!(usual.starts_with("What runs' steps printed is kept for ever."), "{usual}");
    assert!(usual.contains("It is written in the data folder, "), "{usual}");

    let kept = rig.run(&["logs", "--keep", "30"]).await.ok();
    assert!(
        kept.starts_with(
            "What runs' steps printed, and what was said of each as it went, is kept for 30 days"
        ),
        "{kept}"
    );
    assert!(
        kept.contains("Nothing was old enough to let go of now.") && kept.contains("\nsaid by "),
        "{kept}"
    );

    let folder = rig.dir.join("kept-logs");
    let moved = rig.run(&["logs", "--folder", folder.to_str().unwrap()]).await.ok();
    assert!(moved.contains("It is written in ") && moved.contains("kept-logs"), "{moved}");
    let back = rig.run(&["logs", "--folder", "usual", "--keep", "always"]).await.ok();
    assert!(back.starts_with("What runs' steps printed is kept for ever."), "{back}");
    assert!(
        back.contains("What was written before is read where it is: ") && back.contains("kept-logs"),
        "{back}"
    );

    let refused = rig.run(&["logs", "--keep", "often"]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(refused.err.contains("a number of days, or `always`"), "{}", refused.err);
    let refused = rig.run(&["logs", "--folder", "relative/logs"]).await;
    assert!(refused.err.contains("said whole"), "{}", refused.err);
    assert!(rig.run(&["help", "logs"]).await.ok().contains("`--keep 30`"));
}
