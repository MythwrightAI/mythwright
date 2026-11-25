//! A channel kept to its members, from the terminal: made with who is
//! in it, an open one kept, somebody added and somebody leaving, whether
//! its agents are asked nowhere else, and opened again by the factory's
//! owner. Two people, each signed in from a home of their own.

use crate::{Rig, run_in, scratch};

#[tokio::test]
async fn a_channel_is_kept_to_its_members_added_to_left_and_opened_again_from_the_terminal() {
    let rig = Rig::signed_in().await;
    rig.run(&["tier", "premium"]).await.ok();
    // Nell is invited, and registers from a home of her own.
    let invited = rig.run(&["invite", "nell@example.com"]).await.ok();
    let code = invited.lines().next().unwrap().trim_start_matches("invitation code: ").to_string();
    let home = scratch("nell");
    let nell = async |args: &[&str]| run_in(&home, &rig.url, &[], args).await;
    let registered =
        ["register", "--email", "nell@example.com", "--name", "Nell", "--password", "correct horse"];
    let registered = nell(&[&registered[..], &["--invite", &code]].concat()).await.ok();
    assert!(registered.contains("Nell"), "{registered}");
    rig.run(&["agent", "add", "helper"]).await.ok();

    // Made kept, with what it is for: called what it was called, and
    // named by its id.
    let made = rig
        .run(&["channel", "create", "Quince review", "--kept", "--topic", "the greengage accounts"])
        .await
        .ok();
    assert!(made.starts_with("created `Quince review` (kept to its members: ch_"), "{made}");
    let id = made.trim_end().rsplit(' ').next().unwrap().trim_end_matches(')').to_string();
    let list = rig.run(&["channel", "list"]).await.ok();
    assert!(list.contains(&format!("`Quince review` (kept to its members: {id})")), "{list}");
    assert!(list.contains("the greengage accounts") && list.contains("Ada"), "{list}");
    rig.run(&["post", &id, "the medlar ledger"]).await.ok();
    // Somebody who is not in it is told of no such channel, by any of
    // these, and it is not in what they list.
    assert!(!nell(&["channel", "list"]).await.ok().contains("Quince"));
    for asked in [
        &["read", &id][..],
        &["channel", "keep", &id],
        &["channel", "open", &id],
        &["channel", "add", &id, "Nell"],
        &["channel", "leave", &id],
        &["channel", "leave", &id, "--agent", "helper"],
        &["channel", "alone", &id, "on"],
        &["agent", "try", "helper", "--channel", &id],
    ] {
        let refused = nell(asked).await;
        let nowhere: Vec<&str> =
            asked.iter().map(|word| if *word == id { "ch_nonesuch" } else { word }).collect();
        let nowhere = nell(&nowhere).await;
        assert!(
            refused.code != 0 && refused.err.contains("there is no channel"),
            "{asked:?}: {}",
            refused.err
        );
        assert_eq!(refused.err.replace(&id, "<it>"), nowhere.err.replace("ch_nonesuch", "<it>"), "{asked:?}");
    }

    // Added by somebody who is in it, by name or by what it is called;
    // she reads it from the first word, and says what its agents are.
    let added = rig.run(&["channel", "add", "quince review", "nell"]).await.ok();
    assert!(added.contains("is in `Quince review`") && added.starts_with("user:"), "{added}");
    assert!(nell(&["read", &id]).await.ok().contains("the medlar ledger"));
    let held = nell(&["channel", "alone", "Quince review", "on"]).await.ok();
    assert!(held.contains("the agents of `Quince review`") && held.contains("asked nowhere else"), "{held}");
    assert!(rig.run(&["channel", "list"]).await.ok().contains("its agents asked nowhere else"));
    let free = rig.run(&["channel", "alone", &id, "off"]).await.ok();
    assert!(free.contains("are asked anywhere again"), "{free}");
    let unsaid = rig.run(&["channel", "alone", &id]).await;
    assert!(unsaid.code != 0 && unsaid.err.contains("say `on` or `off`"), "{}", unsaid.err);
    // An agent of a factory that holds no walls is in no kept channel:
    // what the daemon says is what is printed.
    let agent = rig.run(&["channel", "add", &id, "helper"]).await;
    assert!(agent.code != 0 && agent.err.contains("is in no channel kept to its members"), "{}", agent.err);
    let made_with = rig
        .run(&["channel", "create", "Hiring", "--kept", "--alone", "--member", "Nell", "--member", "helper"])
        .await;
    assert!(made_with.code != 0 && made_with.err.contains("is in no channel kept"), "{}", made_with.err);
    assert!(!rig.run(&["channel", "list"]).await.ok().contains("Hiring"), "nothing was made");
    let hiring =
        rig.run(&["channel", "create", "Hiring", "--kept", "--alone", "--member", "Nell"]).await.ok();
    assert!(hiring.contains("its agents asked nowhere else"), "{hiring}");
    assert!(nell(&["channel", "list"]).await.ok().contains("`Hiring`"));

    // She leaves, and reads nothing of it; the last person in it does not.
    assert_eq!(nell(&["channel", "leave", &id]).await.ok().trim_end(), "you have left the channel");
    assert!(nell(&["read", &id]).await.err.contains("there is no channel"));
    let last = rig.run(&["channel", "leave", &id]).await;
    assert!(last.code != 0 && last.err.contains("nobody else is in it"), "{}", last.err);

    // An open channel is kept by somebody who is in it, and its name is
    // free from there; opened again, by the owner, it takes a name.
    rig.run(&["channel", "create", "general"]).await.ok();
    rig.run(&["channel", "add", "general", "helper"]).await.ok();
    assert!(nell(&["channel", "keep", "general"]).await.err.contains("is not in #general"));
    // Not with an agent in it that could be in no kept channel: the
    // daemon says which, and how it is taken out.
    let with = rig.run(&["channel", "keep", "general"]).await;
    assert!(with.code != 0 && with.err.contains("channel leave general --agent helper"), "{}", with.err);
    let out = rig.run(&["channel", "leave", "general", "--agent", "helper"]).await.ok();
    assert_eq!(out.trim_end(), "helper is out of the channel, with what it kept of it");
    let kept = rig.run(&["channel", "keep", "#general"]).await.ok();
    assert!(kept.starts_with("`general` (kept to its members: ch_"), "{kept}");
    assert!(kept.contains("nobody else from here"), "{kept}");
    assert!(nell(&["read", "general"]).await.err.contains("there is no channel `general`"));
    nell(&["channel", "create", "general"]).await.ok();
    let general = kept.split("ch_").nth(1).unwrap().split(')').next().unwrap();
    let general = &format!("ch_{general}");
    let taken = rig.run(&["channel", "open", general]).await;
    assert!(taken.code != 0 && taken.err.contains("already a channel called `general`"), "{}", taken.err);
    rig.run(&["channel", "add", general, "Nell"]).await.ok();
    // Two she reads are called the same: named by id, then.
    rig.run(&["channel", "create", "hiring", "--kept", "--member", "Nell"]).await.ok();
    let which = nell(&["channel", "leave", "Hiring"]).await;
    assert!(
        which.code != 0 && which.err.contains("2 channels you read are called `Hiring`"),
        "{}",
        which.err
    );
    let hers = nell(&["channel", "open", general]).await;
    assert!(hers.code != 0 && hers.err.contains("only the factory's owner opens"), "{}", hers.err);
    let opened = rig.run(&["channel", "open", general, "--name", "Old general"]).await.ok();
    assert_eq!(opened.trim_end(), "#old-general is open again: anybody who may read channels reads it");
    assert!(nell(&["channel", "list"]).await.ok().contains("#old-general"));
}
