//! The daemon as a process: starting it, stopping it, and keeping it running.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own, which the tests stop by a signal sent it, and what it hosts is a
//! line of that system's shell.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout};

use crate::{BINARY, run_in, scratch, soon};

pub(crate) struct Daemon {
    pub(crate) child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    pub(crate) url: String,
    pub(crate) dir: PathBuf,
    seen: Vec<String>,
    mode: String,
    /// Where it looks for the programs it runs, when that is not where
    /// these tests look.
    path: Option<String>,
    /// What its profile is called, for one that has one.
    profile: Option<String>,
    /// What else it was started with, and is started with again.
    extra: Vec<String>,
    /// The port it was told, which is this rig's until the rig is put
    /// away: the daemon has it only while it runs, and is started again
    /// there.
    _port: crate::ports::Port,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // The daemon first, so that it starts nothing more; then what it
        // started and left, which ending it does not end.
        let _ = self.child.start_kill();
        crate::ended_under(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Where what a daemon says on its error stream is kept, in its folder.
const SAID: &str = "said-on-stderr";

/// A daemon's error stream: added to the file in its folder, or nowhere
/// if that cannot be opened.
fn said_to(dir: &Path) -> Stdio {
    let _ = std::fs::create_dir_all(dir);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(SAID))
        .map_or_else(|_| Stdio::null(), Stdio::from)
}

pub(crate) fn signal(pid: u32, name: &str) {
    std::process::Command::new("kill").args([name, &pid.to_string()]).status().unwrap();
}

pub(crate) fn alive(pid: u32) -> bool {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => !stat.rsplit_once(')').is_some_and(|(_, rest)| rest.trim_start().starts_with(['Z', 'X'])),
        Err(_) => false,
    }
}

impl Daemon {
    /// Starts `serve` or `up` on a port that is this rig's and waits until
    /// it says it is running.
    pub(crate) async fn start(mode: &str) -> Daemon {
        Daemon::start_on(mode, None).await
    }

    /// The same, for a daemon that finds the programs it runs on `path`
    /// and nowhere else.
    pub(crate) async fn start_on(mode: &str, path: Option<&str>) -> Daemon {
        Daemon::start_as(mode, path, None).await
    }

    /// The same, for the factory of an profile.
    pub(crate) async fn start_as(mode: &str, path: Option<&str>, profile: Option<&str>) -> Daemon {
        Daemon::start_told(mode, path, profile, &[]).await
    }

    /// The same, told something more as it is started (`--handed`).
    pub(crate) async fn start_told(
        mode: &str,
        path: Option<&str>,
        profile: Option<&str>,
        extra: &[&str],
    ) -> Daemon {
        let extra: Vec<String> = extra.iter().map(|word| word.to_string()).collect();
        let dir = scratch(mode);
        // The daemon is told a port that is this rig's from here until
        // the rig is put away (`ports`), since it has to be the same one
        // more than once: a test starts its daemon again where it was
        // (`again`), and what keeps a daemon running starts it again as
        // it was started. A port the system gives is the system's to give
        // again once it is let go, whether this rig had it for a moment
        // (asked for, read and let go) or the daemon for as long as it
        // ran (told to take any, and then started again where it had
        // been): with suites and gates starting daemons side by side
        // something else was listening there by the time the daemon came
        // to, and it ended saying that it could not.
        let port = crate::ports::free();
        let url = format!("http://127.0.0.1:{}", port.number());
        let (child, lines) = Daemon::spawn(mode, &dir, &url, path, profile, &extra);
        let (path, profile) = (path.map(str::to_string), profile.map(str::to_string));
        let (seen, mode) = (Vec::new(), mode.to_string());
        let mut daemon = Daemon { child, lines, url, dir, seen, mode, path, profile, extra, _port: port };
        daemon.until("is running at").await;
        daemon.url = daemon.seen.last().unwrap().rsplit(' ').next().unwrap().to_string();
        daemon
    }

    fn spawn(
        mode: &str,
        dir: &Path,
        url: &str,
        path: Option<&str>,
        profile: Option<&str>,
        extra: &[String],
    ) -> (Child, Lines<BufReader<ChildStdout>>) {
        let own = std::env::var("PATH").unwrap_or_default();
        let named = profile.map(|name| ["--profile", name]);
        let mut child = tokio::process::Command::new(BINARY)
            .args([
                mode,
                "--data",
                dir.join("data").to_str().unwrap(),
                "--listen",
                url.trim_start_matches("http://"),
                "--adapter",
                "builtin",
            ])
            .args(named.iter().flatten())
            .args(extra)
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", path.unwrap_or(&own))
            .env("HOME", dir)
            // Where the command line keeps its session, as `run_in` says it: the
            // owner its first start makes is signed in there.
            .env("XDG_CONFIG_HOME", dir.join("config"))
            // How the journal gathers work is said to a suite by one
            // word, and the journal here is this daemon's.
            .envs(std::env::var("JOURNAL_GATHERS").map(|gathers| ("JOURNAL_GATHERS", gathers)))
            // And so is whether a project that is added has a journal
            // of its own (`journal::apart`).
            .envs(std::env::var("PROJECT_JOURNALS").map(|apart| ("PROJECT_JOURNALS", apart)))
            // A daemon started here is a test's, and waits for the disk
            // nowhere (`journal::disk`): no test takes the power away,
            // and what one that is killed had written is the system's
            // to keep. A daemon that is run is never told this.
            .env("JOURNAL_DISK", "unwaited")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            // Kept, for when it ends without a word of where it is
            // (`until`): each start adds to what the one before said.
            .stderr(said_to(dir))
            // A test that fails part-way must not leave its daemon running.
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        (child, lines)
    }

    /// Stops the daemon as its owner's system would and starts it again
    /// where it was, on what it had.
    pub(crate) async fn again(&mut self) {
        // One that has stopped already, having been asked to, has no
        // process to tell.
        if let Some(pid) = self.child.id() {
            signal(pid, "-TERM");
        }
        soon("the daemon to exit", self.child.wait()).await.unwrap();
        let (child, lines) = Daemon::spawn(
            &self.mode,
            &self.dir,
            &self.url,
            self.path.as_deref(),
            self.profile.as_deref(),
            &self.extra,
        );
        (self.child, self.lines) = (child, lines);
        self.until("is running at").await;
    }

    /// Stops the daemon, or what keeps it running and the daemon with
    /// it, as its owner's system would, and waits until it has ended.
    pub(crate) async fn stop(&mut self) {
        if let Some(pid) = self.child.id() {
            signal(pid, "-TERM");
        }
        soon("the daemon to exit", self.child.wait()).await.unwrap();
    }

    /// Starts `mode` in place of a daemon that was stopped, where it was
    /// and on what it had, and waits for nothing it says.
    pub(crate) fn begin(&mut self, mode: &str) {
        self.mode = mode.to_string();
        let (child, lines) = Daemon::spawn(
            mode,
            &self.dir,
            &self.url,
            self.path.as_deref(),
            self.profile.as_deref(),
            &self.extra,
        );
        (self.child, self.lines) = (child, lines);
    }

    /// Everything this rig's daemons have said on their error streams.
    pub(crate) fn said(&self) -> String {
        std::fs::read_to_string(self.dir.join(SAID)).unwrap_or_default()
    }

    /// Reads the daemon's output until a line contains `text`.
    pub(crate) async fn until(&mut self, text: &str) {
        soon(&format!("the daemon to print `{text}`"), async {
            loop {
                let line = self.lines.next_line().await.unwrap().unwrap_or_else(|| {
                    let said = std::fs::read_to_string(self.dir.join(SAID)).unwrap_or_default();
                    panic!("the daemon's output ended before it printed `{text}`, having said:\n{said}")
                });
                let found = line.contains(text);
                self.seen.push(line);
                if found {
                    return;
                }
            }
        })
        .await;
    }

    pub(crate) async fn run(&self, args: &[&str]) -> crate::Ran {
        run_in(&self.dir, &self.url, &[], args).await
    }

    pub(crate) fn file(&self, name: &str, content: &str) -> String {
        let path: &Path = &self.dir.join(name);
        std::fs::write(path, content).unwrap();
        path.to_string_lossy().into_owned()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_daemon_starts_answers_and_stops_when_asked() {
    let mut daemon = Daemon::start("serve").await;
    assert!(
        daemon.seen[0].starts_with(&format!("{} is running at {}", brand::DISPLAY, daemon.url)),
        "{:?}",
        daemon.seen
    );
    assert!(
        daemon.run(&["status"]).await.ok().contains(" CPUs, "),
        "the owner its first start made is signed in here"
    );

    signal(daemon.child.id().unwrap(), "-TERM");
    let status = soon("the daemon to exit", daemon.child.wait()).await.unwrap();
    assert!(status.success(), "asked to stop, it exits cleanly: {status:?}");
    daemon.until("what is running is left running").await;
    assert_eq!(daemon.run(&["status"]).await.code, 1, "and nothing is listening any more");
}

/// A daemon has stopped when its address is free, and not when it has
/// closed what it listened with: a program it was starting as it stopped
/// has a copy of everything it had open until that program has become
/// itself, and lets the folder's lock go before the listener. Started
/// again in that moment, the daemon found its folder free and its address
/// taken, and was refused it at once as though something that is not this
/// factory had it (once in some fifty crowded playings of a package's
/// cases). The test is that program: it takes a copy of the listener as
/// the system hands one to a program being started, and lets it go only
/// once the daemon says it waits for it, so the order is the one that
/// failed on any machine.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_that_stops_waits_for_its_address_to_be_let_go_of_by_a_program_it_was_starting() {
    let mut daemon = Daemon::start("serve").await;
    let pid = daemon.child.id().unwrap();
    let Some(copy) = listened_with(pid, daemon._port.number()) else {
        // Taking another program's descriptor is refused where the system
        // lets nobody look into a program they started.
        println!("this system lets the test take nothing of its daemon's: the order is not made here");
        return;
    };

    signal(pid, "-TERM");
    daemon.until("to be let go of").await;
    drop(copy);
    let status = soon("the daemon to exit", daemon.child.wait()).await.unwrap();
    assert!(status.success(), "{status:?}");

    // Started again where it was, it has its address.
    daemon.again().await;
    assert!(
        daemon.run(&["status"]).await.ok().contains(" CPUs, "),
        "the owner its first start made is signed in here"
    );
}

/// A daemon that is killed waits for nothing, and leaves where it was
/// served (`serving`), which is how one started on its folder next knows
/// that the address it is refused was its own a moment ago, and waits for
/// it as a program the killed one was starting lets it go. The test holds
/// the copy for as long as the start could wait, so what is said is what
/// is said after waiting, where a start that did not wait said at once
/// that something that is not this factory had the address.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_started_where_one_was_killed_waits_for_the_address_a_program_it_was_starting_has() {
    let mut daemon = Daemon::start("serve").await;
    let pid = daemon.child.id().unwrap();
    let Some(copy) = listened_with(pid, daemon._port.number()) else {
        println!("this system lets the test take nothing of its daemon's: the order is not made here");
        return;
    };
    signal(pid, "-KILL");
    soon("the daemon to end", daemon.child.wait()).await.unwrap();

    daemon.begin("serve");
    let refused = soon("the start to be refused", daemon.child.wait()).await.unwrap();
    assert_eq!(refused.code(), Some(3), "an address something else has is so however often it is asked");
    let said = daemon.said();
    assert!(
        said.contains("was served there and ended without stopping, and the address was waited for"),
        "{said}"
    );
    assert!(!said.contains("Something that is not this factory has the address"), "{said}");

    drop(copy);
    daemon.again().await;
    assert!(
        daemon.run(&["status"]).await.ok().contains(" CPUs, "),
        "the owner its first start made is signed in here"
    );
}

/// A copy of what the daemon `pid` listens with at `port`, or nothing
/// where the system will not hand one over.
#[cfg(target_os = "linux")]
fn listened_with(pid: u32, port: u16) -> Option<std::os::fd::OwnedFd> {
    use rustix::process::{Pid, PidfdFlags, PidfdGetfdFlags, pidfd_getfd, pidfd_open};
    let at = format!(":{port:04X} ");
    let tcp = std::fs::read_to_string("/proc/net/tcp").unwrap();
    let inode = tcp
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .find(|cols| cols.len() > 9 && format!("{} ", cols[1]).ends_with(&at) && cols[3] == "0A")
        .map(|cols| format!("socket:[{}]", cols[9]))
        .expect("the daemon listens at its port");
    let fd = std::fs::read_dir(format!("/proc/{pid}/fd"))
        .unwrap()
        .flatten()
        .find(|fd| std::fs::read_link(fd.path()).is_ok_and(|link| link.to_string_lossy() == inode))
        .and_then(|fd| fd.file_name().to_str()?.parse::<i32>().ok())
        .expect("the daemon holds what it listens with");
    let pidfd = pidfd_open(Pid::from_raw(pid as i32)?, PidfdFlags::empty()).ok()?;
    pidfd_getfd(&pidfd, fd, PidfdGetfdFlags::empty()).ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_that_dies_is_started_again_and_finds_its_fleet_as_it_left_it() {
    let mut daemon = Daemon::start("up").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let blueprint = daemon.file(
        "idle.bp",
        "blueprint: idle\npermissions: [fleet]\nsteps:\n  start: { do: service.start, with: { name: idle, run: \"i=0; while [ $i -lt 3000 ]; do i=$((i+1)); sleep 0.2; done\" } }\n",
    );
    daemon.run(&["apply", &blueprint]).await.ok();
    let before = daemon.run(&["services"]).await.ok();
    assert!(before.starts_with("idle  healthy  port "), "{before}");
    let service_pid: u32 =
        before.split("pid ").nth(1).unwrap().split_whitespace().next().unwrap().parse().unwrap();

    // Kill the daemon outright, as a crash would.
    let supervisor = daemon.child.id().unwrap();
    let children =
        std::process::Command::new("pgrep").args(["-P", &supervisor.to_string()]).output().unwrap();
    let daemon_pid: u32 = String::from_utf8_lossy(&children.stdout)
        .trim()
        .parse()
        .expect("the supervisor has one child: the daemon");
    signal(daemon_pid, "-KILL");
    daemon.until("is running at").await;

    // The service never noticed, and the new daemon knows it.
    let after = daemon.run(&["services"]).await.ok();
    assert_eq!(after, before, "same service, same process, still healthy");
    assert!(alive(service_pid));
    assert!(daemon.run(&["whoami"]).await.ok().starts_with("Ada"), "and the session is still good");
    let events =
        daemon.run(&["events", "--topics", "system.started,service.started", "--limit", "10"]).await.ok();
    let count = |topic: &str| events.lines().filter(|line| line.contains(topic)).count();
    assert_eq!(
        (count("system.started"), count("service.started")),
        (2, 1),
        "started twice; the service once:\n{events}"
    );
    let instances = daemon.run(&["instances"]).await.ok();
    assert!(instances.contains("idle") && instances.contains("active"), "{instances}");

    // Stopping the supervisor stops the daemon and leaves the fleet running.
    signal(supervisor, "-TERM");
    assert!(soon("the supervisor to exit", daemon.child.wait()).await.unwrap().success());
    assert!(alive(service_pid), "the service outlives the daemon, to be found by the next one");
    std::process::Command::new("kill").args(["-KILL", "--", &format!("-{service_pid}")]).status().unwrap();
}

/// A port a daemon gives a service is kept from everything else on the
/// machine by the daemon, a process of its own here as it is for anybody
/// (`hosting::ports`): this test is another process, as another factory
/// would be, and what refuses it is the system. The service never
/// listens, which is the moment between being given a port and taking
/// it, for as long as the test goes on: a port given by asking the system
/// for a free one was the system's to give again in that moment, and a
/// daemon that is run gave its services theirs that way.
#[tokio::test(flavor = "multi_thread")]
async fn a_port_a_daemon_gave_a_service_is_kept_from_other_processes_by_it_and_by_the_daemon_that_comes_back()
{
    let mut daemon = Daemon::start("serve").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let text = format!(
        "blueprint: held\npermissions: [fleet]\nsteps:\n  start:\n    do: service.start\n    with: {{ name: held, restart: never, cmd: {} }}\n",
        crate::standin::cmd(&["idle"])
    );
    let file = daemon.file(&brand::blueprint_file("held"), &text);
    let applied = daemon.run(&["apply", &file, "--json"]).await.ok();
    let applied: serde_json::Value = serde_json::from_str(&applied).unwrap();
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    // Which process it is (its wrapper) and the port it was given.
    async fn hosted(daemon: &Daemon) -> (u32, u16) {
        let said = daemon.run(&["services", "--json"]).await.ok();
        let said: serde_json::Value = serde_json::from_str(&said).unwrap();
        let held = &said["services"][0];
        assert_eq!(held["name"], "held", "{said}");
        (held["pid"].as_u64().unwrap() as u32, held["port"].as_u64().unwrap() as u16)
    }
    let (pid, port) = hosted(&daemon).await;

    // None of those the system gives by itself to whoever asks for any,
    // and nobody else's to speak for while the daemon looks after it.
    assert!(hosting::ports::GIVEN.contains(&port), "it was given {port}, which the system gives again");
    assert!(
        hosting::ports::speak_for(port).is_none(),
        "the daemon does not keep {port} from another process"
    );

    // The daemon goes and another is started on what it left: the
    // service is the same process on the same port, found and not
    // started, and its port is kept by the daemon there is now.
    daemon.again().await;
    assert_eq!(hosted(&daemon).await, (pid, port));
    assert!(alive(pid));
    let started = daemon.run(&["events", "--topics", "service.started", "--limit", "10"]).await.ok();
    assert_eq!(started.lines().filter(|line| line.contains("service.started")).count(), 1, "{started}");
    assert!(
        hosting::ports::speak_for(port).is_none(),
        "the daemon that came back does not keep {port} from another process"
    );

    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    soon("the service's process to be gone", async {
        while alive(pid) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_put_away_with_its_test_takes_with_it_what_it_hosted_and_what_kept_it_running() {
    // What a test that fails part-way leaves: a daemon that is kept
    // running, hosting a service that stays, with nothing stopped.
    let daemon = Daemon::start("up").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let said = daemon.dir.join("pid");
    let text = format!(
        "blueprint: held\npermissions: [fleet]\nsteps:\n  start:\n    do: service.start\n    with:\n      name: held\n      restart: always\n      cmd: [/bin/sh, -c, 'echo $$ >> {}; exec /bin/sleep 300']\n",
        said.display()
    );
    let file = daemon.file(&brand::blueprint_file("held"), &text);
    daemon.run(&["apply", &file]).await.ok();
    let service: u32 = soon("the service to say which process it is", async {
        loop {
            match tokio::fs::read_to_string(&said).await {
                Ok(pid) if pid.ends_with('\n') => return pid.trim().parse().unwrap(),
                _ => tokio::task::yield_now().await,
            }
        }
    })
    .await;
    let supervisor = daemon.child.id().unwrap();
    let children =
        std::process::Command::new("pgrep").args(["-P", &supervisor.to_string()]).output().unwrap();
    let kept: u32 =
        String::from_utf8_lossy(&children.stdout).trim().parse().expect("the daemon it keeps running");
    assert!(alive(service) && alive(kept));

    // The test is over. Ending what the test holds ends neither the
    // daemon under it nor the service, which a daemon leaves on purpose;
    // the rig ends both. Each was ended by the time the rig was put
    // away, and is gone as soon as the system has taken it off its list.
    drop(daemon);
    soon("the daemon that was kept running, and the service it hosted, to be gone", async {
        while alive(kept) || alive(service) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_just_started_is_stopped_by_the_terminal_its_first_start_signed_in() {
    // Started to be looked at, with nothing registered: its first start
    // made its owner and signed this terminal in, which puts it away.
    let mut daemon = Daemon::start("up").await;
    let said = daemon.run(&["down"]).await.ok();
    assert!(said.starts_with("the daemon is stopping"), "{said}");
    assert!(soon("the supervisor to exit", daemon.child.wait()).await.unwrap().success());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_daemon_is_stopped_by_its_owner_from_the_command_line_and_is_not_started_again() {
    let mut daemon = Daemon::start("up").await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    // Once it is somebody's, not by anyone who merely can reach it.
    daemon.run(&["logout"]).await.ok();
    let refused = daemon.run(&["down"]).await;
    assert_ne!(refused.code, 0, "{}", refused.out);
    daemon.run(&["login", "--email", "ada@example.com", "--password", "correct horse"]).await.ok();

    let said = daemon.run(&["down"]).await.ok();
    assert!(said.starts_with("the daemon is stopping"), "{said}");
    // The supervisor takes a daemon that was asked to stop at its word.
    assert!(soon("the supervisor to exit", daemon.child.wait()).await.unwrap().success());
    daemon.until("what is running is left running").await;
    assert_eq!(daemon.run(&["status"]).await.code, 1, "nothing is listening any more");
    // And asking again is not an error: it is already what was wanted.
    assert!(daemon.run(&["down"]).await.ok().starts_with("nothing is running at"));
}

/// What the daemon runs for itself, to start a process and to stop one, it
/// finds where a system keeps it. Whoever starts a daemon may have nothing
/// of the kind on their path (a desktop's launcher, a service manager):
/// found along the path, it could then start nothing; or could start what
/// it could not stop, and say nothing of it.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_with_nothing_on_its_path_starts_a_service_and_stops_it_when_asked() {
    let nothing = scratch("nothing-on-the-path");
    let daemon = Daemon::start_on("serve", Some(nothing.to_str().unwrap())).await;
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    // A service that says which process it is and then stays.
    let said = daemon.dir.join("pid");
    let text = format!(
        "blueprint: held\npermissions: [fleet]\nsteps:\n  start:\n    do: service.start\n    with:\n      name: held\n      restart: never\n      cmd: [/bin/sh, -c, 'echo $$ > {}; exec /bin/sleep 300']\n",
        said.display()
    );
    let file = daemon.file(&brand::blueprint_file("held"), &text);
    let applied = daemon.run(&["apply", &file, "--json"]).await.ok();
    let applied: serde_json::Value = serde_json::from_str(&applied).unwrap();
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let pid = soon("the service to say which process it is", async {
        loop {
            match tokio::fs::read_to_string(&said).await {
                Ok(pid) if pid.ends_with('\n') => return pid.trim().to_string(),
                _ => tokio::task::yield_now().await,
            }
        }
    })
    .await;
    let running = std::path::PathBuf::from(format!("/proc/{pid}"));
    assert!(running.exists(), "it is running, as process {pid}");

    // Stopped, it is gone: asked, and not left behind with the wait for it run out.
    daemon.run(&["stop", applied["instance"].as_str().unwrap()]).await.ok();
    soon("the service's process to be gone", async {
        while running.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let _ = std::fs::remove_dir_all(nothing);
}
