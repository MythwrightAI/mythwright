//! What a sandbox leaves when it ends: nothing.
#![cfg(target_os = "linux")]

use serde_json::json;

use crate::Rig;

/// The names of this process's threads, as the system says them.
fn threads() -> Vec<String> {
    std::fs::read_dir("/proc/self/task")
        .unwrap()
        .flatten()
        .filter_map(|task| std::fs::read_to_string(task.path().join("comm")).ok())
        .map(|name| name.trim().to_string())
        .collect()
}

/// How many threads a sandbox has made: its own, its databases', and
/// the one that does what is asked off its own.
fn of_sandboxes() -> usize {
    let theirs = |name: &String| {
        name.starts_with("sandbox-") || name.starts_with("journal-") || name.starts_with("tokio-")
    };
    threads().iter().filter(|name| theirs(name)).count()
}

/// Waits until `count` is what the system says, without looking at the
/// clock: a thread that has ended is gone a moment after it says so.
fn until(count: usize, what: &str) {
    for _ in 0..2_000_000 {
        if of_sandboxes() == count {
            return;
        }
        std::thread::yield_now();
    }
    panic!(
        "{what}: {} threads of sandboxes are there, and {count} were meant: {:?}",
        of_sandboxes(),
        threads()
    );
}

#[tokio::test]
async fn a_sandbox_that_is_ended_or_let_go_of_leaves_no_thread_and_no_file() {
    // Alone: every other test's sandbox is over, and none begins meanwhile.
    let _alone = crate::held(true).await;
    until(0, "before any sandbox is made");
    let text = "blueprint: asks\nsteps:\n  ask: { do: outside.ask, with: { prompt: \"well?\" } }\n";

    for ended in [true, false] {
        let rig = Rig::alone(text).await;
        // Something is under way in it: a step waits for a stand-in.
        let did = rig.act(json!({ "apply": {} })).await;
        assert_eq!(did["waiting"].as_array().unwrap().len(), 1);
        assert!(rig.folder.join("projects").join("sandbox").is_dir());
        assert!(of_sandboxes() >= 3, "a sandbox is a thread and its databases': {:?}", threads());
        let folder = rig.folder.clone();

        match ended {
            true => {
                rig.end().await;
                // Ended is ended: its folder is gone by the time that is answered.
                assert!(!folder.exists(), "{} is still there", folder.display());
                let gone = crate::in_time("the sandbox to say it ended", &rig.seen, rig.sandbox.read())
                    .await
                    .unwrap_err();
                assert!(gone.message.contains("this sandbox has ended"), "{gone:?}");
            }
            false => drop(rig),
        }

        until(0, if ended { "once it was ended" } else { "once it was let go of" });
        assert!(!folder.exists(), "{} is still there", folder.display());
    }
}

#[tokio::test]
async fn a_sandbox_that_cannot_be_made_leaves_no_folder() {
    let _held = crate::beside().await;
    let folder = crate::folder();
    // Its folder cannot be made: a file is where it would be.
    std::fs::create_dir_all(folder.parent().unwrap()).unwrap();
    std::fs::write(&folder, "in the way").unwrap();
    let made = crate::made("blueprint: quiet\nsteps:\n  ready: { do: noop }\n", folder.clone());

    let seen = sandbox::Seen::default();
    let making = sandbox::Sandbox::make_seen(made, seen.clone());
    let refused =
        crate::in_time("a sandbox to be refused", &seen, making).await.err().expect("it is refused");

    assert!(!refused.message.is_empty());
    std::fs::remove_file(&folder).unwrap();
}

#[tokio::test]
async fn a_test_with_two_sandboxes_has_its_second_while_another_test_waits_to_have_them_all_to_itself() {
    let quiet = "blueprint: quiet\nsteps:\n  ready: { do: noop }\n";
    let first = Rig::new(quiet).await;
    // Another test asks to be alone, and waits for this one's sandbox.
    let alone = std::thread::spawn(|| drop(crate::ALONE.blocking_write()));
    // It waits once nobody more is let in beside those already in.
    let mut asked = 0u64;
    while crate::ALONE.try_read().is_ok() {
        asked += 1;
        assert!(asked < 50_000_000, "the other test never came to wait to be alone");
        std::thread::yield_now();
    }

    // This one's second sandbox is under what it holds already.
    let second = Rig::new(quiet).await;
    let did = second.apply().await;

    assert!(did["did"]["applied"].is_object(), "{did}");
    drop((first, second));
    alone.join().expect("the other test was alone once this one's sandboxes were over");
}
