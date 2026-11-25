//! One's own inbox followed from a terminal as it is told
//! (`inbox --follow`), with the real program: an agent the factory does
//! not start, joined from a home of its own, told each line once as it
//! comes and a line told again once more; a person following theirs in
//! words; nothing of anybody else's; a key that holds no box refused;
//! and, with the daemon a process of its own, what was told while it was
//! stopped and started again printed once it is back, and nothing twice.

use std::path::Path;
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout};
use tokio::sync::mpsc;

use crate::external::{handed, invitation};
use crate::fleets::{event_after, now};
use crate::{Rig, command_in, run_in, scratch, soon};

/// The program following an inbox, its lines read as they come and what
/// it says on its error stream kept apart.
pub(crate) struct Following {
    pub(crate) child: Child,
    out: Lines<BufReader<ChildStdout>>,
    /// Read only by the test of a restart, which is Unix's.
    #[cfg(unix)]
    err: mpsc::UnboundedReceiver<String>,
}

impl Following {
    pub(crate) fn start(dir: &Path, url: &str, args: &[&str]) -> Following {
        let mut command = command_in(dir, url, args);
        command.stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
        let mut child = command.spawn().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap()).lines();
        // Read by a task of its own, so that what it says there never
        // stops it for want of a reader.
        let (said, heard) = mpsc::unbounded_channel::<String>();
        let mut lines = BufReader::new(child.stderr.take().unwrap()).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = said.send(line);
            }
        });
        #[cfg(not(unix))]
        drop(heard);
        Following {
            child,
            out,
            #[cfg(unix)]
            err: heard,
        }
    }

    /// The next line it prints.
    pub(crate) async fn next(&mut self) -> String {
        let line = soon("the next line of the inbox followed", self.out.next_line()).await.unwrap();
        line.unwrap_or_else(|| panic!("the inbox followed printed nothing more"))
    }

    /// The next line it prints, as the object `--json` prints.
    pub(crate) async fn next_json(&mut self) -> Value {
        let line = self.next().await;
        serde_json::from_str(&line).unwrap_or_else(|_| panic!("not one object: {line}"))
    }

    /// Waits until it says something with `text` on its error stream.
    #[cfg(unix)]
    pub(crate) async fn says(&mut self, text: &str) -> String {
        soon(&format!("the inbox followed to say `{text}`"), async {
            loop {
                let line =
                    self.err.recv().await.unwrap_or_else(|| panic!("it ended before it said `{text}`"));
                if line.contains(text) {
                    return line;
                }
            }
        })
        .await
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_inbox_followed_prints_each_line_as_it_is_told_once_and_nothing_of_anybody_elses() {
    let rig = Rig::signed_in().await;
    rig.run(&["channel", "create", "general"]).await.ok();
    // An agent this factory does not start, joined from a home of its own.
    let home = scratch("walls");
    let invited = invitation(&rig.run(&["agent", "invite", "walls"]).await.ok());
    handed(&home, &rig.url, &["agent", "join"], &invited).await.ok();

    // What waits before it is followed is printed first.
    let before = now(&rig);
    rig.run(&["post", "general", "@walls", "hang", "the", "gate"]).await.ok();
    event_after(&rig, before, "inbox.told", |event| event.data["owner"] == "agent:walls").await;
    let mut walls = Following::start(&home, &rig.url, &["--as", "walls", "inbox", "--follow", "--json"]);
    let first = walls.next_json().await;
    assert_eq!(
        (&first["kind"], &first["who"], &first["where"], &first["words"], &first["count"]),
        (&"named".into(), &"Ada".into(), &"general".into(), &"@walls hang the gate".into(), &1.into()),
        "{first}"
    );
    assert!(first["message"].as_str().is_some_and(|id| !id.is_empty()), "{first}");
    assert!(first["from"].as_str().is_some_and(|from| from.starts_with("user:")), "{first}");

    // Each mention as it comes.
    rig.run(&["post", "general", "@walls", "oil", "the", "hinge"]).await.ok();
    let second = walls.next_json().await;
    assert_eq!(second["words"], "@walls oil the hinge", "{second}");
    assert_ne!(second["id"], first["id"]);

    // A conversation is one line, printed again each time it is told.
    rig.run(&["to", "walls", "--subject", "plans", "the", "fence", "next"]).await.ok();
    let written = walls.next_json().await;
    assert_eq!(
        (&written["kind"], &written["where"], &written["words"], &written["count"], &written["conversation"]),
        (&"message".into(), &"plans".into(), &"the fence next".into(), &1.into(), &true.into()),
        "{written}"
    );
    let conversation = written["place"].as_str().unwrap().to_string();
    rig.run(&["post", &conversation, "and", "the", "post"]).await.ok();
    let again = walls.next_json().await;
    assert_eq!(
        (&again["id"], &again["count"], &again["words"]),
        (&written["id"], &2.into(), &"and the post".into())
    );

    // What is addressed to somebody else never arrives: the next line is
    // the next one for this agent.
    let as_walls =
        async |args: &[&str]| run_in(&home, &rig.url, &[], &[&["--as", "walls"], args].concat()).await;
    let before = now(&rig);
    as_walls(&["post", "general", "@ada", "the", "gate", "is", "hung"]).await.ok();
    event_after(&rig, before, "inbox.told", |event| event.data["owner"] != "agent:walls").await;
    rig.run(&["post", "general", "nobody", "is", "named", "here"]).await.ok();
    rig.run(&["post", "general", "@walls", "paint", "it"]).await.ok();
    assert_eq!(walls.next_json().await["words"], "@walls paint it");

    // A person follows theirs the same way, in words.
    let mut ada = Following::start(&rig.dir, &rig.url, &["inbox", "--follow"]);
    let theirs = ada.next().await;
    assert!(theirs.contains("unread  #general  [walls] @ada the gate is hung"), "{theirs}");
    as_walls(&["post", "general", "@ada", "and", "painted"]).await.ok();
    let next = ada.next().await;
    assert!(next.contains("#general  [walls] @ada and painted"), "{next}");

    // A key that holds no box is refused in a sentence, and prints nothing.
    let key = rig.run(&["key", "ci", "--scope", "channels"]).await.ok();
    let key = key.split_whitespace().find(|word| word.starts_with("key_")).unwrap().to_string();
    let refused = rig.run_with(&[(brand::env_var("TOKEN").as_str(), key)], &["inbox", "--follow"]).await;
    assert!(refused.code != 0 && refused.err.contains("needs `inbox:read`"), "{}", refused.err);
    assert!(refused.out.is_empty(), "{}", refused.out);
    // And what is not taken beside it is refused as anywhere.
    let unread = rig.run(&["inbox", "--follow", "--all"]).await;
    assert!(unread.code != 0 && unread.err.contains("--all"), "{}", unread.err);

    // Asked to stop, it ends well.
    #[cfg(unix)]
    for following in [&mut walls, &mut ada] {
        crate::daemon::signal(following.child.id().unwrap(), "-TERM");
        let ended = soon("the inbox followed to end", following.child.wait()).await.unwrap();
        assert_eq!(ended.code(), Some(0));
    }
    // Elsewhere there is no asking: each is ended.
    #[cfg(not(unix))]
    for following in [&mut walls, &mut ada] {
        following.child.kill().await.unwrap();
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// Unix's: the daemon is a process of its own here, stopped by a signal
/// and started again where it was (`daemon.rs`).
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn an_inbox_followed_across_a_restart_prints_what_was_told_meanwhile_once_and_nothing_twice() {
    use crate::daemon::{Daemon, signal};

    let mut daemon = Daemon::start("serve").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    daemon.run(&["channel", "create", "general"]).await.ok();
    let home = scratch("walls-again");
    let invited = invitation(&daemon.run(&["agent", "invite", "walls"]).await.ok());
    handed(&home, &daemon.url, &["agent", "join"], &invited).await.ok();
    let mut walls = Following::start(&home, &daemon.url, &["--as", "walls", "inbox", "--follow", "--json"]);
    daemon.run(&["post", "general", "@walls", "before", "it", "stopped"]).await.ok();
    assert_eq!(walls.next_json().await["words"], "@walls before it stopped");

    // Lost, it says so, and is back once the daemon is.
    daemon.stop().await;
    walls.says("lost the daemon").await;
    daemon.begin("serve");
    daemon.until("is running at").await;
    daemon.run(&["post", "general", "@walls", "while", "it", "was", "away"]).await.ok();
    let meanwhile = walls.next_json().await;
    assert_eq!(meanwhile["words"], "@walls while it was away", "{meanwhile}");
    walls.says("is back").await;
    // And nothing it printed before is printed again: the next line is
    // the next one told.
    daemon.run(&["post", "general", "@walls", "after", "it", "was", "back"]).await.ok();
    assert_eq!(walls.next_json().await["words"], "@walls after it was back");

    // A follower that never reached the daemon ends saying so; one that
    // had, and is waiting for it, ends well when asked to stop.
    daemon.stop().await;
    walls.says("lost the daemon").await;
    let never = run_in(&home, &daemon.url, &[], &["--as", "walls", "inbox", "--follow"]).await;
    assert!(never.code != 0 && never.err.contains("nothing is answering"), "{}", never.err);
    signal(walls.child.id().unwrap(), "-TERM");
    let ended = soon("the inbox followed to end", walls.child.wait()).await.unwrap();
    assert_eq!(ended.code(), Some(0));
    let _ = std::fs::remove_dir_all(&home);
}
