//! What is in the Trash, from the terminal: who deleted each thing named
//! as the page names them, with who that is only where two who deleted
//! something share a name.

use crate::{Rig, run_in, scratch};

#[tokio::test]
async fn the_trash_names_who_deleted_each_thing_and_says_who_that_is_only_where_two_share_a_name() {
    let rig = Rig::signed_in().await;
    rig.run(&["project", "create", "--bare", "Plain"]).await.ok();
    rig.run(&["project", "create", "--bare", "Other"]).await.ok();
    rig.run(&["project", "forget", "plain"]).await.ok();
    let listed = rig.run(&["trash"]).await.ok();
    assert!(listed.contains("project `Plain` (plain)  by Ada  goes "), "{listed}");

    // Another Ada, from a home of her own, deletes something too.
    rig.run(&["tier", "premium"]).await.ok();
    let invited = rig.run(&["invite", "ada.two@example.com"]).await.ok();
    let code = invited.lines().next().unwrap().trim_start_matches("invitation code: ").to_string();
    let home = scratch("another-ada");
    let other = async |args: &[&str]| run_in(&home, &rig.url, &[], args).await;
    let register =
        ["register", "--email", "ada.two@example.com", "--name", "Ada", "--password", "correct horse"];
    other(&[&register[..], &["--invite", &code]].concat()).await.ok();
    other(&["project", "forget", "other"]).await.ok();
    let listed = rig.run(&["trash"]).await.ok();
    let lines: Vec<&str> = listed.lines().filter(|line| line.contains("project `")).collect();
    assert_eq!(lines.len(), 2, "{listed}");
    assert!(lines.iter().all(|line| line.contains("  by Ada (user:")), "each said with who it is: {listed}");
}
