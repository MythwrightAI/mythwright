//! A command a row of `segments` runs for a card that goes on past the
//! time its row says (`warn_after` of `std/run`): whoever the
//! project's `tells` names for `slow` is told once, in their inbox, the
//! command is not stopped, and a daemon stopped while it runs, after the
//! telling, tells nobody twice when the command is run again.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use kernel::ManualClock;
use serde_json::json;

use crate::Rig;
use crate::fleets::{add, add_row, card, shop};
use crate::lanes::{move_card, moved};
use crate::tells::{heard, lines};

/// A node on a clock the test moves, whose fleet's stand-ins work any
/// card and approve it; and the folder the command's holds are in.
async fn ticking() -> (Rig, ManualClock, PathBuf) {
    let clock = ManualClock::new();
    let moved = clock.clone();
    let mut at = None;
    let rig = Rig::with("api-slowed", |config, dir| {
        config.clock = Some(Arc::new(moved));
        let writes = ["swallow", "write", "CHANGES-%:pid%.md", "changed", "say", "I changed a file."];
        let looks = ["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, clock, at.unwrap())
}

#[tokio::test]
async fn a_command_that_runs_past_its_time_is_told_once_goes_on_and_is_told_to_nobody_twice_after_a_stop() {
    let (mut rig, clock, at) = ticking().await;
    let hold = at.join("hold-check");
    crate::hold(&hold);
    let mut began = crate::Listening::at(&at.join("began-check"));
    let mut begun = async || {
        crate::soon("the command to have begun", async { began.next_line().await.unwrap() }).await;
    };
    shop(&rig, json!({})).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    // The command says it began, where the test listens, and waits to be
    // let go. Twenty minutes, and whoever applied the fleet is told.
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row = json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": command,
        "warn_after": "20m",
    });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;

    // 1: its command is under way, and nobody has been told anything of
    // it. Twenty-one minutes on, whoever applied the fleet is told once,
    // in words that say which card, which command and how long so far.
    begun().await;
    assert_eq!(lines(&rig, &rig.token, "slow").await, []);
    let before = rig.mark();
    clock.advance(Duration::from_secs(21 * 60));
    let slow = rig.event("card.slow", move |event| event.seq > before).await;
    assert_eq!((&slow.data["card"], &slow.data["after"]), (&json!(id), &json!("20m")), "{:?}", slow.data);
    heard(&rig, "on.slow", slow.seq).await;
    // A command longer than a sentence says is cut there.
    let shown = match command.chars().count() > 120 {
        true => format!("{}…", command.chars().take(120).collect::<String>()),
        false => command.clone(),
    };
    let sentence = format!(
        "{id}: `{shown}` has been running for 21m, past the 20m it may run before somebody is told, and goes on."
    );
    assert_eq!(lines(&rig, &rig.token, "slow").await, [(sentence.clone(), 1)]);
    // It is not stopped: the card is still where it is looked at, and an
    // hour more says nothing more of it.
    assert_eq!(card(&rig, &id).await["status"], "review");
    // Its history says so in a line of its own, and not the command's
    // whole words.
    rig.event("collection.history", |event| event.data["topic"] == "card.slow").await;
    let history = rig.read(&format!("/api/records/{id}/history")).await.to_string();
    assert!(history.contains("its command was still running after 20m"), "{history}");
    clock.advance(Duration::from_secs(3600));
    assert_eq!(rig.count_of("card.slow").await, 1, "`after` is once");

    // 2: the daemon is stopped while the command runs and started again.
    // The command is run again from its start, and runs past its time
    // again: what it says of the card is told to nobody a second time.
    rig.restart().await;
    begun().await;
    let before = rig.mark();
    clock.advance(Duration::from_secs(21 * 60));
    let again = rig.event("card.slow", move |event| event.seq > before).await;
    heard(&rig, "on.slow", again.seq).await;
    assert_eq!(lines(&rig, &rig.token, "slow").await, [(sentence, 1)]);
    assert_eq!(rig.count_of("inbox.told").await, 1, "told once");

    // 3: let go, it passes, and the card goes on as it would have.
    let before = rig.mark();
    crate::pass(&hold, 8);
    moved(&rig, &id, "done", before).await;
    assert_eq!(lines(&rig, &rig.token, "slow").await.len(), 1);
}

#[tokio::test]
async fn a_command_whose_row_says_no_time_is_told_of_to_nobody_however_long_it_runs() {
    let (rig, clock, at) = ticking().await;
    let hold = at.join("hold-check");
    crate::hold(&hold);
    let mut began = crate::Listening::at(&at.join("began-check"));
    shop(&rig, json!({})).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row =
        json!({ "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": command });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    crate::soon("the command to have begun", async { began.next_line().await.unwrap() }).await;
    clock.advance(Duration::from_secs(24 * 3600));
    let before = rig.mark();
    crate::pass(&hold, 8);
    moved(&rig, &id, "done", before).await;
    assert_eq!(rig.count_of("card.slow").await, 0);
    assert_eq!(lines(&rig, &rig.token, "slow").await, []);
}

#[tokio::test]
async fn a_command_that_runs_past_its_time_again_on_a_later_coming_of_its_card_raises_the_line_there_is() {
    let (rig, clock, at) = ticking().await;
    let hold = at.join("hold-check");
    crate::hold(&hold);
    let mut began = crate::Listening::at(&at.join("began-check"));
    let mut begun = async || {
        crate::soon("the command to have begun", async { began.next_line().await.unwrap() }).await;
    };
    shop(&rig, json!({})).await;
    rig.event("record.created", |event| event.data["collection"] == "tells").await;
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (begins, waits) = (file("began-check"), file("hold-check"));
    let command = crate::standin::line(&["tell", &begins, "check", "held", &waits, "check"]);
    let row = json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": command,
        "warn_after": "20m",
    });
    add_row(&rig, "segments", row).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;

    // 1: its first coming runs long and is told; let go, the card is done.
    begun().await;
    let before = rig.mark();
    clock.advance(Duration::from_secs(21 * 60));
    let slow = rig.event("card.slow", move |event| event.seq > before).await;
    heard(&rig, "on.slow", slow.seq).await;
    let told = lines(&rig, &rig.token, "slow").await;
    assert_eq!(told.iter().map(|(_, count)| *count).collect::<Vec<_>>(), [1], "{told:?}");
    let before = rig.mark();
    crate::pass(&hold, 1);
    moved(&rig, &id, "done", before).await;
    // Its place tidied away, which is the last thing done for a card.
    rig.event("run.finished", move |event| {
        event.seq > before && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.tidied"
    })
    .await;

    // 2: made ready again, it is worked again and comes to the command
    // again, which runs long again: that is another coming, and whoever
    // was told is told again, by the one line they have, raised.
    move_card(&rig, &id, "ready", None).await;
    begun().await;
    let before = rig.mark();
    clock.advance(Duration::from_secs(21 * 60));
    let again = rig.event("card.slow", move |event| event.seq > before).await;
    assert_ne!(again.data["asking"], slow.data["asking"], "another coming");
    heard(&rig, "on.slow", again.seq).await;
    let told = lines(&rig, &rig.token, "slow").await;
    assert_eq!(told.iter().map(|(_, count)| *count).collect::<Vec<_>>(), [2], "{told:?}");
    assert_eq!(rig.count_of("inbox.told").await, 2);
    let before = rig.mark();
    crate::pass(&hold, 1);
    moved(&rig, &id, "done", before).await;
}
