//! Sign-ins from a terminal, against a daemon that is a process of its
//! own with an environment of its own: what a person types, where a value
//! may come from, and what an agent's program is then started with.
//!
//! The program is a stand-in that writes down the environment it was
//! started with. Nothing here is anybody's sign-in.
//!
//! Unix's alone: the daemon here is the real program as a process of its
//! own (`daemon::Daemon`), and the agent's program a file of a Unix shell
//! made a program by its mode.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;

use crate::{BINARY, Ran, command_in, run_in, scratch, soon};

/// What the daemon's own environment holds that an agent's program must
/// not be handed, or is handed only when its agent draws nothing.
const OWN_TOKEN: &str = "the-daemons-own-token";
const DEVICE_KEY: &str = "the-key-of-whoever-runs-the-daemon";

struct Factory {
    child: Child,
    url: String,
    dir: PathBuf,
}

impl Drop for Factory {
    fn drop(&mut self) {
        // The daemon, and what it started and left.
        let _ = self.child.start_kill();
        crate::ended_under(&self.dir);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl Factory {
    /// A daemon of its own, started where credentials of its own and a
    /// sign-in of the device's are in the environment; its owner signed
    /// in; and a package for a stand-in installed.
    async fn start() -> Factory {
        let dir = scratch("signins");
        let mut child = tokio::process::Command::new(BINARY)
            .args(["serve", "--data", dir.join("data").to_str().unwrap(), "--listen", "127.0.0.1:0"])
            .args(["--adapter", "builtin"])
            .env_clear()
            .envs(crate::kept_on_windows())
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", &dir)
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", dir.join("share"))
            .env(brand::env_var("TOKEN"), OWN_TOKEN)
            // What is handed on to a program in the open is what these
            // are about, and the stand-in writes beside itself; behind a
            // wall it inherits nothing and could not.
            .env(brand::env_var("WALLS"), "off")
            .env("STANDIN_KEY", DEVICE_KEY)
            .env("STANDIN_PROFILE", "work")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            // A test that fails part-way must not leave its daemon running.
            .kill_on_drop(true)
            .spawn()
            .unwrap();
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
        let factory = Factory { child, url, dir };
        factory
            .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
            .await
            .ok();
        let package = factory.dir.join("standin");
        std::fs::create_dir_all(&package).unwrap();
        let program = package.join("program");
        let script = "#!/bin/sh\nif [ \"$1\" = models ]; then env > \"HERE/env-models\"; printf '{\"models\":[{\"key\":\"quick\"}]}\\n'; exit 0; fi\ncat > /dev/null\nenv > \"HERE/env\"\nprintf '{\"kind\":\"end\",\"text\":\"done\"}\\n'\n";
        std::fs::write(&program, script.replace("HERE", &package.to_string_lossy())).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        let manifest = r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: PROGRAM
  input: ${ turn.prompt }
  output:
    - when: ${ event.kind == "end" }
      reply: ${ event.text }
  models:
    ask:
      args: ["models"]
      answer: ${ line.models != null }
      list: ${ line.models }
      id: ${ model.key }
  signin:
    key: STANDIN_KEY
    token: STANDIN_TOKEN
    without: [STANDIN_PROFILE]
"#;
        std::fs::write(
            package.join("extension.yaml"),
            manifest.replace("PROGRAM", &program.to_string_lossy()),
        )
        .unwrap();
        factory.run(&["extension", "add", package.to_str().unwrap()]).await.ok();
        factory
    }

    async fn run(&self, args: &[&str]) -> Ran {
        run_in(&self.dir, &self.url, &[], args).await
    }

    /// Runs a command with `input` on its standard input.
    async fn run_given(&self, input: &str, args: &[&str]) -> Ran {
        let mut command = command_in(&self.dir, &self.url, args);
        let mut child =
            command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        // A command that gives up before it reads leaves nobody to write
        // to. That is not the failure: what it said as it gave up is, and
        // the test that called this reads it.
        let _ = child.stdin.take().unwrap().write_all(input.as_bytes()).await;
        let output = soon(&format!("`{}`", args.join(" ")), child.wait_with_output()).await.unwrap();
        Ran {
            code: output.status.code().unwrap_or(-1),
            out: String::from_utf8_lossy(&output.stdout).into_owned(),
            err: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// Has an agent do a piece of work, and gives back the environment
    /// its program was started with.
    async fn works(&self, agent: &str) -> Vec<String> {
        let blueprint = self.dir.join("work.bp");
        let text = format!(
            "blueprint: work-{agent}\npermissions: [agents]\nsteps:\n  it:\n    do: agent.task\n    with: {{ slot: {agent}, prompt: build the cart }}\n"
        );
        std::fs::write(&blueprint, text).unwrap();
        self.run(&["apply", blueprint.to_str().unwrap()]).await.ok();
        let written = std::fs::read_to_string(self.dir.join("standin/env")).unwrap();
        written.lines().map(str::to_string).collect()
    }

    /// Stops the daemon, by its own process.
    async fn stop(mut self) {
        let pid = self.child.id().expect("the daemon is running");
        crate::daemon::signal(pid, "-TERM");
        let _ = soon("the daemon to stop", self.child.wait()).await;
    }
}

fn has(env: &[String], name: &str) -> Option<String> {
    env.iter().find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_string))
}

/// Every file under `dir` that holds `value`. A link is no file of the
/// folder and is not followed: what it names is looked at where it is,
/// where that is under the folder, and the one link a daemon makes that
/// leads out of its data is to its own program (`bin`), which is
/// nothing it keeps and was most of what this test took to read.
fn files_holding(dir: &Path, value: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            let held = |path: &Path| {
                let bytes = std::fs::read(path).unwrap_or_default();
                bytes.windows(value.len()).any(|window| window == value.as_bytes())
            };
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push(path),
                Ok(kind) if kind.is_file() && held(&path) => found.push(path),
                _ => {}
            }
        }
    }
    found
}

#[tokio::test]
async fn a_sign_in_is_given_on_standard_input_or_in_a_file_and_is_never_shown_again() {
    let factory = Factory::start().await;
    let (token, key) = ("token-of-the-subscription", "key-that-is-paid-for");

    let added = factory
        .run_given(&format!("{token}\n"), &["signin", "add", "plan", "--for", "standin", "--kind", "token"])
        .await;
    assert!(added.ok().contains("`plan` is held: a token for `standin`, for 1 at once"));
    let file = factory.dir.join("the-key");
    std::fs::write(&file, format!("{key}\n")).unwrap();
    let args = ["signin", "add", "paid", "--for", "standin", "--kind", "key", "--at-once", "2", "--file"];
    factory.run(&[&args[..], &[file.to_str().unwrap()]].concat()).await.ok();
    std::fs::remove_file(&file).unwrap();

    // A value is not something to type after the command: with nothing on
    // standard input and no file, there is none.
    let typed = factory.run(&["signin", "add", "typed", "--for", "standin", "--kind", "key", key]).await;
    assert_eq!(typed.code, 1);
    assert!(typed.err.contains("no key was given"), "{}", typed.err);
    // And what a sign-in is for, and what it is, is said.
    let unsaid = factory.run_given(key, &["signin", "add", "paid", "--kind", "key"]).await;
    assert!(unsaid.err.contains("--for"), "{}", unsaid.err);
    let wrong =
        factory.run_given(key, &["signin", "add", "paid", "--for", "nonesuch", "--kind", "key"]).await;
    assert!(wrong.err.contains("no such package called `nonesuch` is installed"), "{}", wrong.err);

    for listing in
        [factory.run(&["signin", "list"]).await.ok(), factory.run(&["signin", "list", "--json"]).await.ok()]
    {
        assert!(listing.contains("plan") && listing.contains("paid"), "{listing}");
        assert!(!listing.contains(token) && !listing.contains(key), "a value is never shown: {listing}");
    }
    let listed = factory.run(&["signin", "list"]).await.ok();
    let paid = listed.lines().find(|line| line.starts_with("paid")).unwrap();
    assert!(paid.contains("key") && paid.contains("2 at once") && paid.contains("free"), "{paid}");

    // An agent told to draw is handed the subscription, and then says so.
    let told =
        ["agent", "add", "minion", "--adapter", "standin", "--signin", "subscription", "--signin", "key"];
    factory.run(&told).await.ok();
    let env = factory.works("minion").await;
    assert_eq!(has(&env, "STANDIN_TOKEN").as_deref(), Some(token));
    let listed = factory.run(&["signin", "list"]).await.ok();
    let plan = listed.lines().find(|line| line.starts_with("plan")).unwrap();
    assert!(plan.contains("free") && plan.contains("used ") && !plan.contains("never used"), "{plan}");

    // Each is kept in a file only its owner can read, sealed: the value
    // is in no file the factory wrote, that one included.
    for (name, value) in [("plan", token), ("paid", key)] {
        let kept = factory.dir.join("data/signins").join(name);
        let holding: Vec<PathBuf> = files_holding(&factory.dir, value)
            .into_iter()
            .filter(|path| !path.starts_with(factory.dir.join("standin")))
            .collect();
        assert_eq!(std::fs::metadata(&kept).unwrap().permissions().mode() & 0o777, 0o600);
        assert!(std::fs::read(&kept).unwrap().starts_with(b"sealed 1\n"), "{name}");
        assert_eq!(holding, Vec::<PathBuf>::new(), "{name}");
    }

    factory.run(&["signin", "remove", "plan"]).await.ok();
    assert!(!factory.run(&["signin", "list"]).await.ok().contains("plan"));
    assert!(factory.run(&["signin", "restore", "paid"]).await.ok().contains("`paid` is good again"));
    assert!(factory.run(&["help", "signin"]).await.ok().contains("never as an argument"));
    factory.stop().await;
}

#[tokio::test]
async fn an_agents_program_is_started_without_the_daemons_own_credentials_whatever_it_draws() {
    let factory = Factory::start().await;
    factory
        .run_given(
            "token-of-the-subscription",
            &["signin", "add", "plan", "--for", "standin", "--kind", "token"],
        )
        .await
        .ok();

    // An agent that draws nothing is started as it always was: with the
    // device's own sign-in, and what it is deliberately given.
    factory.run(&["agent", "add", "plain", "--adapter", "standin"]).await.ok();
    let env = factory.works("plain").await;
    assert_eq!(has(&env, "STANDIN_KEY").as_deref(), Some(DEVICE_KEY));
    assert_eq!(has(&env, "STANDIN_PROFILE").as_deref(), Some("work"));
    assert_eq!(has(&env, "STANDIN_TOKEN"), None);
    assert_eq!(has(&env, &brand::env_var("SLOT")).as_deref(), Some("plain"));
    assert_eq!(has(&env, &brand::env_var("URL")).as_deref(), Some(factory.url.as_str()));
    let token_file = has(&env, &brand::env_var("TOKEN_FILE")).unwrap();
    assert!(token_file.ends_with("data/slots/plain/token"), "{token_file}");
    // Except that the daemon's own credentials are not its to have.
    assert_eq!(has(&env, &brand::env_var("TOKEN")), None, "the daemon's own token for the command line");
    assert!(!env.iter().any(|line| line.contains(OWN_TOKEN)));
    // And it is told what everything the daemon starts is told.
    assert_eq!(has(&env, "FACTORY_URL").as_deref(), Some(factory.url.as_str()));
    assert_eq!(has(&env, "FACTORY_PROFILE").as_deref(), Some(""), "a factory with no environment");
    assert!(has(&env, "FACTORY_CLI").is_some());

    // One that draws is handed what it drew and nothing else to sign in
    // with: not the device's key, and not what else its package names.
    factory.run(&["agent", "add", "minion", "--adapter", "standin", "--signin", "subscription"]).await.ok();
    let env = factory.works("minion").await;
    assert_eq!(has(&env, "STANDIN_TOKEN").as_deref(), Some("token-of-the-subscription"));
    assert_eq!(has(&env, "STANDIN_KEY"), None, "the device's own key is taken out");
    assert_eq!(has(&env, "STANDIN_PROFILE"), None, "and what else its package says it would sign in by");
    assert_eq!(has(&env, &brand::env_var("TOKEN")), None);
    assert_eq!(has(&env, &brand::env_var("SLOT")).as_deref(), Some("minion"));

    // Asked which models it offers, no agent is asking: it is started the
    // same way, as whoever runs the daemon.
    assert!(factory.run(&["extension", "models", "standin"]).await.ok().contains("quick"));
    let written = std::fs::read_to_string(factory.dir.join("standin/env-models")).unwrap();
    let env: Vec<String> = written.lines().map(str::to_string).collect();
    assert_eq!(has(&env, &brand::env_var("TOKEN")), None);
    assert_eq!(has(&env, "STANDIN_KEY").as_deref(), Some(DEVICE_KEY));
    assert_eq!(has(&env, "FACTORY_URL").as_deref(), Some(factory.url.as_str()));

    // Told to draw none again, it is as the first.
    factory.run(&["agent", "add", "minion", "--no-signin"]).await.ok();
    let env = factory.works("minion").await;
    assert_eq!(has(&env, "STANDIN_KEY").as_deref(), Some(DEVICE_KEY));
    assert_eq!(has(&env, "STANDIN_TOKEN"), None);
    factory.stop().await;
}
