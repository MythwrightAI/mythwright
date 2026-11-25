//! An agent whose program goes on without the daemon, with the real
//! program as the daemon: killed outright while the agent works, and
//! started again after the agent has finished.
//!
//! This is here and not with the agents' own tests because what stands
//! between such a program and its files, when it was handed a key, is this
//! program started under a word of its own, and in those tests "this
//! program" is the test.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), stopped by a signal; its agent is a line of
//! shell held at a pipe with a name, and is killed by a signal.
#![cfg(unix)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::daemon::{alive, signal};
use crate::{BINARY, command_in, run_in, scratch, soon};

/// What the agent is handed, which must be in nothing that is written down.
const KEY: &str = "tok-4f7a91c2d8e6";

struct Factory {
    child: tokio::process::Child,
    url: String,
    dir: PathBuf,
    /// What each daemon prints, held on to: a daemon whose output nobody
    /// holds any more ends the next time it has something to say.
    said: Vec<Said>,
}

type Said = tokio::io::Lines<BufReader<tokio::process::ChildStdout>>;

impl Factory {
    fn spawn(dir: &Path, listen: &str) -> tokio::process::Child {
        tokio::process::Command::new(BINARY)
            .args(["serve", "--data", dir.join("data").to_str().unwrap(), "--listen", listen])
            .args(["--adapter", "builtin", "--walls", "off"])
            .args(["--command", &format!("writer={}", dir.join("writer.sh").display())])
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", dir)
            // Where the command line keeps its session, as `run_in` says it: the
            // owner its first start makes is signed in there.
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap()
    }

    /// Reads what a daemon prints until it says where it is running: that,
    /// and the rest of what it prints, to be held on to.
    async fn running(child: &mut tokio::process::Child) -> (String, Said) {
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let url = soon("the daemon to say where it is", async {
            loop {
                let line = lines.next_line().await.unwrap().expect("the daemon ended before it was running");
                if line.contains("is running at") {
                    return line.rsplit(' ').next().unwrap().to_string();
                }
            }
        })
        .await;
        (url, lines)
    }

    async fn run(&self, args: &[&str]) -> String {
        run_in(&self.dir, &self.url, &[], args).await.ok()
    }

    /// The file of the one turn's program that is called this.
    fn kept(&self, what: &str) -> Option<PathBuf> {
        let run = self.dir.join("data").join("run");
        std::fs::read_dir(run).ok()?.flatten().map(|entry| entry.path()).find(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            name.starts_with("turn.") && name.ends_with(&format!(".{what}"))
        })
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_that_outlives_the_daemon_finishes_while_it_is_dead_and_what_it_was_handed_is_in_no_file() {
    let dir = scratch("outliving");
    let _ended = crate::Ended(&dir);
    // It says its key, says it has begun, waits to be let go, and answers
    // with its key again: twice more than any agent should.
    let at = dir.display();
    let does = format!(
        "#!/bin/sh\ncat >/dev/null\necho \"my key is $REMOTE_TOKEN\"\necho asked >> \"{at}/asked\"\n\
         echo begun > \"{at}/begun\"\nread go < \"{at}/hold\"\necho \"done with $REMOTE_TOKEN\"\n"
    );
    std::fs::write(dir.join("writer.sh"), does).unwrap();
    assert!(
        std::process::Command::new("chmod")
            .args(["+x", "writer.sh"])
            .current_dir(&dir)
            .status()
            .unwrap()
            .success()
    );
    for pipe in ["hold", "begun"] {
        assert!(std::process::Command::new("mkfifo").arg(dir.join(pipe)).status().unwrap().success());
    }
    let mut hold = std::fs::OpenOptions::new().read(true).write(true).open(dir.join("hold")).unwrap();

    // The daemon is started a second time where the first was, so where
    // that is, is the test's own until it is over (`crate::ports`).
    let (_own, listen) = crate::profiles::own_address();
    let mut child = Factory::spawn(&dir, &listen);
    let (url, said) = Factory::running(&mut child).await;
    assert_eq!(url, format!("http://{listen}"));
    let mut factory = Factory { child, url, dir: dir.clone(), said: vec![said] };
    factory
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await;
    factory.run(&["project", "create", "--bare", "Shop"]).await;
    factory.run(&["channel", "create", "shop", "--project", "shop"]).await;
    let mut set = command_in(&dir, &factory.url, &["secret", "set", "bot"]);
    let mut set = set.stdin(Stdio::piped()).stdout(Stdio::null()).spawn().unwrap();
    set.stdin.take().unwrap().write_all(KEY.as_bytes()).await.unwrap();
    assert!(soon("the secret to be kept", set.wait()).await.unwrap().success());
    let added = factory
        .run(&[
            "agent",
            "add",
            "builder",
            "--adapter",
            "writer",
            "--project",
            "shop",
            "--channel",
            "shop",
            "--secret",
            "REMOTE_TOKEN=bot",
            "--no-wall",
            "--outlives",
        ])
        .await;
    assert!(added.contains("its program goes on without the daemon"), "{added}");
    assert!(factory.run(&["agent", "list"]).await.contains("goes on without the daemon"));

    // Asked something, it is at work.
    factory.run(&["post", "shop", "builder: add a cart"]).await;
    soon("the agent's program to say it has begun", tokio::fs::read_to_string(dir.join("begun")))
        .await
        .unwrap();
    // What it has printed so far comes to be in a file, with its key taken
    // out. It is put there by what stands between the program and its
    // files, a moment after the program printed it: by the time the
    // program says it has begun the line may not have been written yet.
    let out = factory.kept("out").expect("it writes to a file");
    let said = soon("what it printed to be in its file", async {
        loop {
            let said = std::fs::read_to_string(&out).unwrap();
            if said.ends_with('\n') {
                return said;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert_eq!(said, "my key is [the secret `bot`]\n");
    let pid: u32 = std::fs::read_to_string(factory.kept("pid").unwrap()).unwrap().trim().parse().unwrap();

    // The daemon is killed outright. Its agent's program is not.
    signal(factory.child.id().unwrap(), "-KILL");
    soon("the daemon to be gone", factory.child.wait()).await.unwrap();
    assert!(alive(pid), "the agent's program goes on");
    // Let go, it finishes with nobody there, and says how it ended.
    hold.write_all(b"go\n").unwrap();
    soon("its program to end", async {
        while factory.kept("exit").is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await;

    // A daemon started on the same data finds the answer, and says it in
    // the channel as the agent, without the key.
    factory.child = Factory::spawn(&dir, &listen);
    let (_, said) = Factory::running(&mut factory.child).await;
    factory.said.push(said);
    let read = soon("the agent's answer to be in the channel", async {
        loop {
            let read = factory.run(&["read", "shop", "--limit", "10"]).await;
            if read.contains("done with") {
                return read;
            }
        }
    })
    .await;
    assert!(read.contains("done with [the secret `bot`]"), "{read}");
    assert!(!read.contains(KEY));
    // Asked once: the daemon that came back asked it nothing.
    assert_eq!(std::fs::read_to_string(dir.join("asked")).unwrap().lines().count(), 1);
    // And nothing under the data folder holds the key, in any file.
    let found =
        std::process::Command::new("grep").args(["-rl", KEY]).arg(dir.join("data").join("run")).output();
    assert_eq!(String::from_utf8_lossy(&found.unwrap().stdout).trim(), "", "the key is written in a file");

    factory.run(&["down"]).await;
    let _ = soon("the daemon to stop", factory.child.wait()).await;
    let _ = std::fs::remove_dir_all(&dir);
}
