//! Factories with profiles, as processes on one device: each started
//! by name, found by name, and signed in to by itself.
//!
//! Every command here says which factory it means, by `--env` or by its
//! address: a command that said neither would go to wherever a factory with
//! no profile usually listens, which on a developer's machine is their
//! own.
//!
//! Unix's alone: its factories are the real program as processes of their
//! own, each stopped by a signal sent it (`daemon::signal`).
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStdout};

use crate::daemon::signal;
use crate::{BINARY, Ran, scratch, soon};

/// One user's home: where the factories they start keep their data unless
/// told otherwise, and where the list of them is.
pub(crate) struct Home(pub(crate) PathBuf);

impl Drop for Home {
    fn drop(&mut self) {
        // The daemons started in this home, and what any of them left.
        crate::ended_under(&self.0);
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// An address on this device at a port that is the test's own for as
/// long as what comes with it is kept (`crate::ports`): for a factory
/// that a test stops and starts again where it was. A port the factory
/// took for itself is the system's to give again while the factory is
/// stopped, and something else was now and then listening there by the
/// time it came back.
pub(crate) fn own_address() -> (crate::ports::Port, String) {
    let port = crate::ports::free();
    let address = format!("127.0.0.1:{}", port.number());
    (port, address)
}

/// A daemon started in a home, and what it has printed.
pub(crate) struct Started {
    pub(crate) child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    pub(crate) seen: Vec<String>,
    pub(crate) url: String,
}

impl Home {
    pub(crate) fn new(name: &str) -> Home {
        Home(scratch(name))
    }

    pub(crate) fn command(&self, args: &[&str]) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(BINARY);
        command
            .args(args)
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &self.0)
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("XDG_DATA_HOME", self.0.join("share"))
            .stdin(Stdio::null());
        command
    }

    /// Runs a command as this user, with whatever else is in its environment.
    async fn run_with(&self, env: &[(String, &str)], args: &[&str]) -> Ran {
        assert!(
            args.iter().any(|arg| ["--profile", "--env", "--url"].contains(arg))
                || !env.is_empty()
                || ["profiles", "env"].contains(&args[0]),
            "a command here must say which factory it means: {args:?}"
        );
        let mut command = self.command(args);
        for (name, value) in env {
            command.env(name, value);
        }
        let output = soon(&format!("`{}`", args.join(" ")), command.output()).await.unwrap();
        Ran {
            code: output.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    pub(crate) async fn run(&self, args: &[&str]) -> Ran {
        self.run_with(&[], args).await
    }

    /// Starts a daemon with `args` and waits until it says it is running.
    pub(crate) async fn start(&self, args: &[&str]) -> Started {
        self.start_as("serve", args).await
    }

    /// The same, as the daemon itself (`serve`) or as what keeps one
    /// running (`up`).
    pub(crate) async fn start_as(&self, mode: &str, args: &[&str]) -> Started {
        let mut child = self
            .command(&[&[mode, "--adapter", "builtin"], args].concat())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            // A test that fails part-way must not leave its daemon running.
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let lines = BufReader::new(child.stdout.take().unwrap()).lines();
        let mut started = Started { child, lines, seen: Vec::new(), url: String::new() };
        started.until("  client  ").await;
        started.url = started.seen[0].rsplit(' ').next().unwrap().to_string();
        started
    }

    /// Where a factory called `name` keeps its data when it is told nowhere.
    pub(crate) fn data_of(&self, name: &str) -> PathBuf {
        self.0.join("share").join(format!("{}-{name}", brand::ID))
    }

    fn config(&self) -> PathBuf {
        self.0.join("config").join(brand::ID)
    }

    pub(crate) async fn register(&self, env: &str, name: &str) {
        let email = format!("{}@example.com", name.to_lowercase());
        self.run(&[
            "--profile",
            env,
            "register",
            "--email",
            &email,
            "--name",
            name,
            "--password",
            "correct horse",
        ])
        .await
        .ok();
    }
}

impl Started {
    pub(crate) async fn until(&mut self, text: &str) {
        soon(&format!("the daemon to print `{text}`"), async {
            loop {
                let line = self.lines.next_line().await.unwrap().expect("the daemon's output ended");
                let found = line.contains(text);
                self.seen.push(line);
                if found {
                    return;
                }
            }
        })
        .await;
    }

    pub(crate) async fn stop(&mut self) {
        signal(self.child.id().unwrap(), "-TERM");
        soon("the daemon to exit", self.child.wait()).await.unwrap();
    }
}

fn lines_of<'a>(listed: &'a str, name: &str) -> Vec<&'a str> {
    listed.lines().filter(|line| line.split_whitespace().next() == Some(name)).collect()
}

pub(crate) fn path(dir: &Path) -> &str {
    dir.to_str().unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn two_factories_started_by_name_have_their_own_data_address_and_sign_in_and_are_found_by_name() {
    let home = Home::new("env-two");
    let cli = brand::CLI;
    let prod = home.start(&["--profile", "prod"]).await;
    let mut dev = home.start(&["--profile", "dev1"]).await;

    // Each says what it is, has data of its own beside where a factory with
    // no profile would keep its, and an address of its own on this device.
    assert_eq!(prod.seen[1], "  profile  prod", "{:?}", prod.seen);
    assert!(prod.seen.last().unwrap().contains(&format!("`{cli} --profile prod status`")), "{:?}", prod.seen);
    assert!(
        home.data_of("prod").join("journal.db").is_file()
            && home.data_of("dev1").join("journal.db").is_file()
    );
    assert!(!home.0.join("share").join(brand::ID).exists(), "neither landed on the usual folder");
    assert_ne!(prod.url, dev.url);
    assert!(prod.url.starts_with("http://127.0.0.1:") && !prod.url.ends_with(":7700"), "{}", prod.url);

    // The list has both, by name and address, and says each is there.
    let listed = home.run(&["profiles"]).await.ok();
    assert_eq!(listed.lines().count(), 2, "{listed}");
    for (name, started) in [("prod", &prod), ("dev1", &dev)] {
        let line = lines_of(&listed, name)[0];
        assert!(line.contains(&started.url) && line.contains("answering"), "{listed}");
        assert!(line.ends_with(&format!("data in {}", home.data_of(name).display())), "{listed}");
    }

    // Signing in is to one of them. Each made its owner as it first
    // started and signed this terminal in to it, with a session kept for
    // each, so that the one a daemon found by its address would use is
    // not there to be changed.
    let kept = |name: &str| {
        let path = home.config().join(format!("sessions/{name}.json"));
        let session: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        session["token"].as_str().unwrap().to_string()
    };
    assert_ne!(kept("prod"), kept("dev1"), "a session of one is not a session of the other");
    assert!(!home.config().join("session.json").exists());
    let fresh = home.run(&["--profile", "prod", "status"]).await.ok();
    assert!(fresh.contains("(prod) on "), "{fresh}");
    home.register("prod", "Ada").await;
    assert!(home.run(&["--profile", "prod", "whoami"]).await.ok().starts_with("Ada"));
    // An account in one is not an account in the other.
    assert!(!home.run(&["--profile", "dev1", "whoami"]).await.ok().starts_with("Ada"));
    home.register("dev1", "Grace").await;
    assert!(home.run(&["--profile", "dev1", "whoami"]).await.ok().starts_with("Grace"));
    assert!(home.run(&["--profile", "prod", "whoami"]).await.ok().starts_with("Ada"));
    // The name can be in the environment, for a terminal that is all one factory's.
    let variable = brand::env_var("PROFILE");
    assert!(home.run_with(&[(variable.clone(), "dev1")], &["whoami"]).await.ok().starts_with("Grace"));
    // And by the old spelling of before 0.1.0, flag and variable, which say so once.
    let old = home.run_with(&[(brand::env_var("ENV"), "dev1")], &["whoami"]).await;
    assert_eq!(old.err.matches("is the old spelling of").count(), 1, "{}", old.err);
    assert!(old.ok().starts_with("Grace"));
    let old = home.run(&["--env", "prod", "whoami"]).await;
    assert!(old.err.contains("`--env` is the old spelling of `--profile`"), "{}", old.err);
    assert!(old.ok().starts_with("Ada"));
    // The list holds nobody's credential.
    let written = std::fs::read_to_string(home.config().join("environments.json")).unwrap();
    let session: Value =
        serde_json::from_str(&std::fs::read_to_string(home.config().join("sessions/prod.json")).unwrap())
            .unwrap();
    assert!(!written.contains(session["token"].as_str().unwrap()), "{written}");

    // What is made in one is not in the other.
    home.run(&["--profile", "prod", "project", "create", "--bare", "Riverside Shop"]).await.ok();
    home.run(&["--profile", "prod", "apply", "std/board", "--project", "riverside-shop"]).await.ok();
    home.run(&["--profile", "prod", "board", "add", "riverside-shop", "Ship", "it"]).await.ok();
    assert!(home.run(&["--profile", "prod", "board", "riverside-shop"]).await.ok().contains("Ship it"));
    assert!(home.run(&["--profile", "dev1", "project", "list"]).await.ok().starts_with("no projects yet"));
    assert_ne!(home.run(&["--profile", "dev1", "board", "riverside-shop"]).await.code, 0);
    assert!(home.data_of("prod").join("projects/riverside-shop").is_dir());
    assert!(!home.data_of("dev1").join("projects/riverside-shop").exists());

    // A directory that is prod's is refused to dev1, and nothing is made of it there.
    let shop = home.0.join("shop");
    std::fs::create_dir_all(&shop).unwrap();
    std::fs::write(shop.join("README.md"), "# Shop\n").unwrap();
    home.run(&["--profile", "prod", "project", "create", "--bare", "Shop", "--path", path(&shop)]).await.ok();
    let refused =
        home.run(&["--profile", "dev1", "project", "create", "--bare", "Shop", "--path", path(&shop)]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(refused.err.contains("is a project of a `prod` factory"), "{}", refused.err);
    assert!(refused.err.contains("this is a `dev1` factory"), "{}", refused.err);
    assert!(home.run(&["--profile", "dev1", "project", "list"]).await.ok().starts_with("no projects yet"));

    // A blueprint's command is told which one it is running in.
    let asks = home.0.join(brand::blueprint_file("asks"));
    let source = "blueprint: asks\npermissions: [shell]\nsteps:\n  said: { do: shell, with: { run: 'echo \"in $FACTORY_PROFILE\"' } }\noutputs:\n  said: \"${ trim(steps.said.out.stdout) } and ${ profile.name }\"\n";
    std::fs::write(&asks, source).unwrap();
    for name in ["prod", "dev1"] {
        let applied = home.run(&["--profile", name, "apply", path(&asks)]).await.ok();
        assert!(applied.contains(&format!("in {name} and {name}")), "{applied}");
    }

    // Stopped, it is still listed, as not running, and asking for it says how to start it.
    dev.stop().await;
    let listed = home.run(&["profiles"]).await.ok();
    assert!(lines_of(&listed, "dev1")[0].contains("not running"), "{listed}");
    assert!(lines_of(&listed, "prod")[0].contains("answering"), "{listed}");
    let gone = home.run(&["--profile", "dev1", "status"]).await;
    assert_eq!(gone.code, 1);
    assert!(gone.err.contains(&format!("nothing is answering at {}", dev.url)), "{}", gone.err);
    assert!(gone.err.contains(&format!("start it with `{cli} up --profile dev1`")), "{}", gone.err);

    // Started again by name alone, it is where it was, with what it had.
    let again = home.start(&["--profile", "dev1"]).await;
    assert_eq!(again.url, dev.url);
    assert!(home.run(&["--profile", "dev1", "whoami"]).await.ok().starts_with("Grace"));

    // A name nothing was started under, and a name and an address together.
    let unknown = home.run(&["--profile", "staging", "status"]).await;
    assert_eq!(unknown.code, 1);
    assert!(unknown.err.contains("there is no `staging` factory on this device"), "{}", unknown.err);
    assert!(unknown.err.contains(&format!("`{cli} up --profile staging` starts one")), "{}", unknown.err);
    let both = home.run(&["--profile", "prod", "--url", &dev.url, "status"]).await;
    assert!(both.err.contains("give one"), "{}", both.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn two_factories_called_the_same_are_not_reached_by_the_name() {
    let home = Home::new("env-same");
    let one = home.start(&["--profile", "dev1"]).await;
    let other_data = home.0.join("another");
    let other = home.start(&["--profile", "dev1", "--data", path(&other_data)]).await;
    assert_ne!(one.url, other.url);

    let refused = home.run(&["--profile", "dev1", "status"]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(refused.err.contains("there are 2 `dev1` factories on this device"), "{}", refused.err);
    for (started, data) in [(&one, home.data_of("dev1")), (&other, other_data.clone())] {
        let line = refused.err.lines().find(|line| line.contains(&started.url)).expect("each is listed");
        assert!(line.ends_with(&format!("data in {}", data.display())), "{}", refused.err);
    }
    assert!(refused.err.contains(&format!("`{} --url <address> ...`", brand::CLI)), "{}", refused.err);

    // By its address, each is itself: and a session saved with one, by its
    // address, is not handed to the other for being asked at another.
    // Each first start kept its owner's session under the name, and the
    // second, finding the first's there, left it: by its address, this
    // terminal is signed in as the owner of the one whose folder it reads.
    let first = home.data_of("dev1");
    let here = ["login", "--here", "--data", path(&first)];
    home.run(&[&["--url", &one.url], &here[..]].concat()).await.ok();
    let register = ["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"];
    home.run(&[&["--url", &one.url], &register[..]].concat()).await.ok();
    assert!(home.run(&["--url", &one.url, "whoami"]).await.ok().starts_with("Ada"));
    let elsewhere = home.run(&["--url", &other.url, "whoami"]).await;
    assert_eq!(elsewhere.code, 1, "{}", elsewhere.out);
    let sent = format!("the session kept on this machine is with {} and not {}", one.url, other.url);
    assert!(elsewhere.err.contains(&sent), "nothing was sent with the request: {}", elsewhere.err);
    assert!(home.run(&["--url", &other.url, "status"]).await.ok().contains("(dev1) is running"));
    assert!(home.run(&["--url", &one.url, "status"]).await.ok().contains("(dev1) on "));
    assert_eq!(lines_of(&home.run(&["profiles"]).await.ok(), "dev1").len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_data_folder_is_one_profiles_for_good_however_it_is_started() {
    let home = Home::new("env-stamp");
    let data = home.0.join("data");
    let (_own, listen) = own_address();
    let mut prod = home.start(&["--profile", "prod", "--data", path(&data), "--listen", &listen]).await;
    prod.stop().await;

    // As another, it is refused, and whatever keeps a daemon running does not go on trying.
    for mode in ["serve", "up"] {
        let refused = home.command(&[mode, "--profile", "dev1", "--data", path(&data)]).output();
        let refused = soon("the daemon to refuse", refused).await.unwrap();
        assert_eq!(refused.status.code(), Some(2), "{mode}");
        let said = String::from_utf8_lossy(&refused.stderr);
        assert!(said.contains(&format!("{} is the data of a `prod` factory", data.display())), "{said}");
        assert!(said.contains("belongs to one profile for good"), "{said}");
    }
    let unnamed = home.command(&["serve", "--profile", "Prod One"]).output();
    let said = soon("the daemon to refuse", unnamed).await.unwrap();
    assert_eq!(said.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&said.stderr).contains("cannot be a profile's name"));

    // With no name it is what it was, where it was: this is how a factory
    // that was given a name once is started ever after.
    let again = home.start(&["--data", path(&data)]).await;
    assert!(again.seen[1].starts_with("  profile  prod"), "{:?}", again.seen);
    assert_eq!(again.url, prod.url);
    // And it is the factory the owner its first start made is signed in to.
    assert!(home.run(&["--profile", "prod", "status"]).await.ok().contains("(prod) on "));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_factory_with_no_profile_is_given_one_by_starting_it_once_with_the_name() {
    let home = Home::new("env-name-it");
    let data = home.0.join("data");
    // As every factory was before any had a name.
    let (_own, listen) = own_address();
    let mut unnamed = home.start(&["--data", path(&data), "--listen", &listen]).await;
    let url = unnamed.url.clone();
    assert_eq!(url, format!("http://{listen}"));
    assert!(!unnamed.seen.iter().any(|line| line.contains("profile")), "{:?}", unnamed.seen);
    let register = ["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"];
    home.run(&[&["--url", &url], &register[..]].concat()).await.ok();
    let status = home.run(&["--url", &url, "status"]).await.ok();
    assert!(status.starts_with(&format!("{} {} on ", brand::DISPLAY, env!("CARGO_PKG_VERSION"))), "{status}");
    assert!(home.run(&["profiles"]).await.ok().starts_with("no factory on this device has a profile yet"));
    assert!(!data.join("factory.json").exists());
    // And nothing of it is written outside its own data, whatever it makes there.
    home.run(&["--url", &url, "project", "create", "--bare", "Riverside Shop"]).await.ok();
    assert!(!home.config().join("environments.json").exists());
    unnamed.stop().await;

    // Named, on the data and at the address it has.
    let mut named = home.start(&["--profile", "prod", "--data", path(&data), "--listen", &listen]).await;
    assert_eq!(named.url, url);
    // Whoever was signed in to it by its address still is, and it has what it had.
    assert!(home.run(&["--url", &url, "whoami"]).await.ok().starts_with("Ada"));
    assert!(home.run(&["--url", &url, "status"]).await.ok().contains("(prod) on "));
    named.stop().await;
    // From then on it needs telling neither its name nor its address.
    let again = home.start(&["--data", path(&data)]).await;
    assert_eq!(again.url, url);
    assert!(lines_of(&home.run(&["profiles"]).await.ok(), "prod")[0].contains(&url));
}

#[tokio::test(flavor = "multi_thread")]
async fn what_has_taken_a_stopped_factorys_address_is_not_sent_its_session() {
    let home = Home::new("env-taken");
    let (_own, address) = own_address();
    let mut dev = home.start(&["--profile", "dev1", "--listen", &address]).await;
    home.register("dev1", "Grace").await;
    dev.stop().await;

    // Something else is listening where dev1 was: here, another factory.
    let other = home.start(&["--data", path(&home.0.join("other")), "--listen", &address]).await;
    assert_eq!(other.url, dev.url);

    let refused = home.run(&["--profile", "dev1", "whoami"]).await;
    assert_eq!(refused.code, 1, "{}", refused.out);
    assert!(
        refused.err.contains("what answers there now is a factory with no profile. Nothing was sent to it"),
        "{}",
        refused.err
    );
    let listed = home.run(&["profiles"]).await.ok();
    assert!(lines_of(&listed, "dev1")[0].contains("not running (something else answers there)"), "{listed}");
}
