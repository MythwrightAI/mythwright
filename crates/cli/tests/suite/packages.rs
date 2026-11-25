//! The packages kept beside the application that bring blueprints, each
//! held to its own examples.
//!
//! A package's blueprints run somebody's program, and nothing here may know
//! which. So what such a blueprint should do is written in the package: its
//! `examples/` holds a stand-in for the program (`stand-in.sh`), which
//! answers as the program does and writes down how it was called, and cases
//! (`<what is true>.case.json`), each a list of things done and what must
//! then be so. This runs every case against a daemon of its own that is the
//! real binary, with the stand-in under the program's name where that
//! daemon looks for programs, and nothing else there: a package's actions
//! are run by the application itself, so there is no shell for it to find
//! and no tool a script would need.
//!
//! A step of a case is one of:
//!
//! * `{ "given": { "<file>": "<text>" } }`: files beside the stand-in,
//!   which is how it is told what state to answer from; `null` removes one.
//! * `{ "apply": "<blueprint>", "params": {..}, "force": true }`, with what
//!   must come of it: `"gives": {..}` (its outputs), `"unchanged": true`,
//!   or `"fails": "<part of the sentence>"`. One that is applied warns
//!   of nothing, or the case fails. With `"project": "<project>"`
//!   it is applied to that project, and `"sets_off": "<handler>"` waits
//!   for the run of that handler which follows.
//! * `{ "run": ["<argument>", ..] }`: the command line, run with those
//!   arguments as the case's owner, which is how a case makes what a
//!   blueprint needs to be there (a project, a board, a card) and says
//!   what a blueprint hears. `"says": "<part of what it prints>"` is what
//!   it must then print, and `"sets_off": "<handler>"` waits for the run
//!   of that handler which follows, of whichever blueprint, or of the
//!   one `"in": "<blueprint>"` names where two have a handler called
//!   that. `"refused": "<part of the sentence>"`
//!   is a command that is to be refused, and what it is refused with.
//! * `{ "stop": "<blueprint>" }`: the instance that applying it made.
//! * `{ "restart": "<handler>" }`: the daemon is stopped and started, and
//!   the handler's run that follows is waited for; `"fails"` as above.
//! * `{ "call": "<caller>", "body": {..}, "signed": "<secret>", "header":
//!   "<name>", "prefix": "<text>" }`: a call to a caller of the case's
//!   daemon as a service outside makes one, with no key: what it sends
//!   signed with that secret as such services sign (HMAC-SHA256 over the
//!   body, in hex), the signature in that header after that text. Which
//!   header and what goes before the signature are the service's own, so
//!   the case says them. `"refused": true` is a call the daemon is not to
//!   take; `"sets_off"` as for `run`.
//! * `{ "key": "<name>", "scopes": ["<scope>", ..] }`: a key made by the
//!   case's owner that holds those scopes and no more. `{key:<name>}` in
//!   any text after it is that key, so a `run` that says `--token
//!   {key:<name>}` is the command line run as somebody who holds less.
//! * `{ "stopped_at": "<step>", "of": "<blueprint>", "by": {..}, "sets_off":
//!   "<handler>" }`: the daemon is told to stop as a step of that name of
//!   that blueprint of the package next begins (`down --at`), `by`, a
//!   `run` or a `call` as above, is done to bring that about, and when
//!   the daemon has stopped there it is started again and the run of the
//!   handler that follows is waited for. From the moment the step begins
//!   the daemon keeps nothing more, so what the step's own action did
//!   before the daemon was gone is not written down anywhere: it may
//!   have asked the program and may not have. A case that plays this
//!   says what is so afterwards either way. `"meanwhile": {..}` is files
//!   written beside the stand-in while the daemon is down, as `given`
//!   writes them: what the program had done in that moment that the
//!   daemon never heard of.
//! * `{ "stopped_inside": "<what was done>", "by": {..}, "sets_off":
//!   "<handler>" }`: the daemon is stopped while its program is in the
//!   middle of being asked something, having done it and not yet
//!   answered. A stand-in that finds a pipe called `held` beside it says
//!   down the pipe what it has just done, and waits there; the case makes
//!   the pipe, does `by`, hears what was done, which must be this, stops
//!   the daemon and starts it again.
//! * `{ "repository": "<folder>", "remote": "<address>" }`: a folder beside
//!   the stand-in made a git repository with one commit, and the address
//!   its remote `origin`, which is how a case has a project whose folder
//!   is a repository (`run` then adds it, `--path {here}/<folder>`). Git is
//!   then found beside the stand-in, as for a fleet, since the daemon asks
//!   git about the folder.
//! * `{ "ran": ["<arguments>", ..] }`: exactly how the program was called
//!   since the last time this was asked. `{any}` in one stands for
//!   letters with no space among them, where what is said is not the
//!   case's to know (the number the journal gave an event).
//! * `{ "holds": { "<folder>": { "<file>": "<text>" } } }`: exactly what a
//!   folder beside the stand-in holds now. `{any}` stands there for what it
//!   does in `ran`.
//!
//! A package whose program is asked something long may say it once, in
//! `examples/words.json`: a name each, and the text it stands for.
//! `{<name>}` in any text of its cases is then that text, so that a case
//! reads as what was asked and the asking is written, and held, in one
//! place.
//!
//! A case that says `"agent": "<script>"` beside its steps has a stand-in
//! for an agent's program as well: the script is what any agent of its
//! daemon runs on that is told `scripted` (`adapter` of a fleet's row),
//! handed what the agent is asked on its input and answering on its
//! output as an agent's program does. The daemon of such a case holds no
//! walls and can find git beside the stand-in, since a fleet works in a
//! project's folder.
//!
//! `{daemon}` in any text is the address of the case's daemon, and `{here}`
//! the folder the stand-in is in, where `given` puts its files. A case that
//! says `"profile": "<name>"` beside its steps is played against the
//! factory of that profile: a second factory on a device, as `dev1` is
//! beside `prod`.
//!
//! Unix's alone: each case has a daemon that is the real program as a
//! process of its own (`daemon::Daemon`), a stand-in for the package's
//! program that is a file of a Unix shell, and one held at a pipe with a
//! name.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::daemon::Daemon;
use crate::{scratch, soon};

/// The cases of every package here that has any, as (package, case).
fn cases() -> Vec<(PathBuf, PathBuf)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions");
    let mut found = Vec::new();
    for package in std::fs::read_dir(dir).unwrap().flatten().map(|entry| entry.path()) {
        for example in std::fs::read_dir(package.join("examples")).into_iter().flatten().flatten() {
            if example.file_name().to_string_lossy().ends_with(".case.json") {
                found.push((package.clone(), example.path()));
            }
        }
    }
    found.sort();
    found
}

/// What is asked in an address, each value written as an address holds it.
fn asked(query: &[(&str, &str)]) -> String {
    let written = |text: &str| -> String {
        text.bytes()
            .map(|byte| match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (byte as char).to_string()
                }
                _ => format!("%{byte:02X}"),
            })
            .collect()
    };
    query.iter().map(|(key, value)| format!("{key}={}", written(value))).collect::<Vec<_>>().join("&")
}

/// One case's daemon, and the folder its stand-in lives in.
struct Stage {
    daemon: Daemon,
    /// Where the stand-in is, and what it keeps.
    standin: PathBuf,
    /// The package's name, and the instances applying its blueprints made.
    package: String,
    instances: BTreeMap<String, String>,
    /// The keys the case made, by name.
    keys: BTreeMap<String, String>,
    /// What the package's cases say by a name, and the text each stands for.
    words: BTreeMap<String, String>,
    case: String,
    /// What reads what the daemon says, once a step has asked ([`Stage::reader`]).
    reader: tokio::sync::OnceCell<String>,
}

/// Whether `ran` is `expected`, where `{any}` in what is expected stands
/// for one or more letters with no space among them.
fn is(ran: &str, expected: &str) -> bool {
    let Some((before, after)) = expected.split_once("{any}") else { return ran == expected };
    let Some(rest) = ran.strip_prefix(before) else { return false };
    let free = rest.find(char::is_whitespace).unwrap_or(rest.len());
    (1..=free).filter(|at| rest.is_char_boundary(*at)).any(|at| is(&rest[at..], after))
}

impl Stage {
    fn text(&self, text: &str) -> String {
        let mut text = text.to_string();
        for (name, stands_for) in &self.words {
            text = text.replace(&format!("{{{name}}}"), stands_for);
        }
        let mut text =
            text.replace("{daemon}", &self.daemon.url).replace("{here}", &self.standin.to_string_lossy());
        for (name, key) in &self.keys {
            text = text.replace(&format!("{{key:{name}}}"), key);
        }
        text
    }

    /// A value of a case as the text it stands for.
    fn plain(&self, value: &Value) -> String {
        match value {
            Value::String(text) => self.text(text),
            other => other.to_string(),
        }
    }

    /// A value of a case with the daemon's address filled in throughout.
    fn filled(&self, value: &Value) -> Value {
        serde_json::from_str(&self.text(&value.to_string())).unwrap()
    }

    async fn apply(&mut self, step: &Value) {
        let blueprint = step["apply"].as_str().unwrap();
        let name = format!("{}/{blueprint}", self.package);
        let mut args = vec!["apply".to_string(), name, "--json".to_string()];
        for (key, value) in step["params"].as_object().into_iter().flatten() {
            args.extend(["--param".to_string(), format!("{key}={}", self.plain(value))]);
        }
        if step["force"] == true {
            args.push("--force".to_string());
        }
        if let Some(project) = step["project"].as_str() {
            args.extend(["--project".to_string(), project.to_string()]);
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let place = self.before(step).await;
        let ran = self.daemon.run(&args).await;
        let case = &self.case;
        if let Some(sentence) = step["fails"].as_str() {
            assert_ne!(ran.code, 0, "{case}: applying `{blueprint}` should have been refused:\n{}", ran.out);
            let sentence = self.text(sentence);
            assert!(ran.err.contains(&sentence), "{case}: it was refused, but with:\n{}", ran.err);
            return;
        }
        assert_eq!(ran.code, 0, "{case}: applying `{blueprint}` failed:\n{}\n{}", ran.out, ran.err);
        let applied: Value = serde_json::from_str(&ran.out).unwrap();
        // A blueprint a package brings is applied with nothing to warn
        // of: what its handlers hear is said somewhere that can be seen,
        // or said to come from outside.
        let warned = applied["warnings"].as_array().is_some_and(|warnings| !warnings.is_empty());
        assert!(!warned, "{case}: applying `{blueprint}` warns: {}", applied["warnings"]);
        self.instances.insert(blueprint.to_string(), applied["instance"].as_str().unwrap().to_string());
        if step["unchanged"] == true {
            assert_eq!(applied["changed"], false, "{case}: nothing was to be done again:\n{applied}");
        }
        if !step["gives"].is_null() {
            assert_eq!(
                applied["finished"]["outputs"],
                self.filled(&step["gives"]),
                "{case}: what it gave out"
            );
        }
        if let (Some(handler), Some(place)) = (step["sets_off"].as_str(), place) {
            self.run_of(handler, &place, &Value::Null).await;
        }
    }

    /// A token of the case's owner's, for reading what the daemon says
    /// with nothing of the command line between: a key that holds what
    /// they hold, made the first time a step waits for what it sets off.
    async fn reader(&self) -> &str {
        self.reader
            .get_or_init(|| async {
                let made = self.daemon.run(&["key", "the-cases-reader", "--json"]).await.ok();
                serde_json::from_str::<Value>(&made).unwrap()["token"].as_str().unwrap().to_string()
            })
            .await
    }

    /// The daemon read as the case's owner: what it answers, as JSON.
    async fn read(&self, path: &str, query: &[(&str, &str)]) -> Value {
        let asked = reqwest::Client::new()
            .get(format!("{}{path}?{}", self.daemon.url, asked(query)))
            .bearer_auth(self.reader().await)
            .send()
            .await
            .unwrap();
        asked.json().await.unwrap()
    }

    /// Where every journal of the daemon has got to: a place, handed out
    /// by the daemon and handed back as it was. What is said from here on
    /// is after it, whichever journal keeps it. Never an event's number:
    /// a project that is added has a journal of its own, and of two
    /// events of two journals the later may have the lesser number. A
    /// daemon started again with a run of a project's to take up draws
    /// that project's numbers before its own, and what the factory says
    /// from then on (a message in a channel, its index) has the greater
    /// numbers: a case that went by the newest number waited for ever
    /// for a project's handler whose events all had lesser ones.
    async fn now(&self) -> String {
        self.read("/api/events/recent", &[("limit", "1")]).await["cursor"].as_str().unwrap().to_string()
    }

    /// Where the daemon's journals have got to before a step that waits
    /// for what it sets off, which is found by having been said after
    /// that. A step that waits for nothing does not ask: three in four
    /// do not.
    async fn before(&self, step: &Value) -> Option<String> {
        match step["sets_off"].is_null() {
            true => None,
            false => Some(self.now().await),
        }
    }

    /// Whether the run `run` was set off by something said after `place`.
    /// A run nothing said set off (one on a timer), or one the daemon
    /// cannot say anything of, is taken to have been. An event's number is
    /// one event's in the whole factory, so it is looked for among what
    /// was said after the place, and never held against another number.
    async fn set_off_after(&self, run: &str, place: &str) -> bool {
        let shown = self.daemon.run(&["run", run, "--json"]).await;
        let by = serde_json::from_str::<Value>(shown.out.trim()).unwrap_or_default()["event"].clone();
        let (Some(seq), Some(topic)) = (by["seq"].as_u64(), by["topic"].as_str()) else { return true };
        let after =
            self.read("/api/events/recent", &[("after", place), ("topics", topic), ("limit", "1000")]).await;
        after["events"].as_array().unwrap().iter().any(|event| event["seq"].as_u64() == Some(seq))
    }

    /// Waits for the run of `handler` that the step set off to finish:
    /// one set off by something said after `place`, which is where the
    /// daemon's journals had got to before the step. And holds it to
    /// `fails` if the step says that.
    ///
    /// Finishing after `place` is not enough to be that run. A run of the
    /// same handler that an earlier step set off may still be ending as
    /// this step begins (one handler's run says what sets off another's,
    /// and the case goes on when the first has ended): taken for the
    /// step's own, the case looked at what the program had been asked
    /// before the handler it was waiting for had asked it.
    async fn run_of(&self, handler: &str, place: &str, step: &Value) {
        let handler = format!("on.{handler}");
        let mut stream = reqwest::Client::new()
            .get(format!(
                "{}/api/events?{}",
                self.daemon.url,
                asked(&[("since", place), ("topics", "run.finished")])
            ))
            .bearer_auth(self.reader().await)
            .send()
            .await
            .unwrap();
        let finished = soon(&format!("{}: the run of `{handler}`", self.case), async {
            // That a run finished is passed on and not kept, where nothing
            // hears it: one that finished before the stream was there is
            // found among the runs, as they are written.
            if let Some(ended) = self.ended_already(&handler, place, step).await {
                return ended;
            }
            let mut buffer = String::new();
            loop {
                let Some(end) = buffer.find("\n\n") else {
                    let chunk = stream.chunk().await.unwrap().expect("the events ended");
                    buffer.push_str(&String::from_utf8_lossy(&chunk));
                    continue;
                };
                let frame: String = buffer.drain(..end + 2).collect();
                let Some(data) = frame.lines().find_map(|line| line.strip_prefix("data:")) else { continue };
                let Ok(event) = serde_json::from_str::<Value>(data.trim()) else { continue };
                let of = step["in"].as_str().is_none_or(|of| event["data"]["blueprint"] == of);
                if event["data"]["flow"] == handler.as_str() && of {
                    let run = event["source"]["run"].as_str().unwrap_or_default();
                    if self.set_off_after(run, place).await {
                        return event["data"].clone();
                    }
                }
            }
        })
        .await;
        let case = &self.case;
        match step["fails"].as_str() {
            Some(sentence) => {
                let said = finished["error"].as_str().unwrap_or_default();
                assert!(
                    said.contains(&self.text(sentence)),
                    "{case}: `{handler}` should have failed: {finished}"
                );
            }
            None => assert_eq!(finished["status"], "ok", "{case}: `{handler}`: {finished}"),
        }
    }

    /// A run of `handler` (`on.<name>`) that has ended already and was set
    /// off by something said after `place`: how it ended, as the event
    /// that says so carries it.
    async fn ended_already(&self, handler: &str, place: &str, step: &Value) -> Option<Value> {
        let runs = self.read("/api/runs", &[("limit", "500")]).await;
        for run in runs["runs"].as_array().into_iter().flatten() {
            if run["flow"] != handler || run["status"] == "running" {
                continue;
            }
            if let Some(of) = step["in"].as_str() {
                let instance = run["instance"].as_str().unwrap_or_default();
                let shown = self.read(&format!("/api/instances/{instance}"), &[]).await;
                if shown["instance"]["blueprint"] != of {
                    continue;
                }
            }
            if self.set_off_after(run["id"].as_str().unwrap_or_default(), place).await {
                return Some(json!({ "status": run["status"], "error": run["error"] }));
            }
        }
        None
    }

    /// Stops the daemon, starts it, and waits for the run of `handler`
    /// that the starting sets off.
    async fn restart(&mut self, step: &Value) {
        let place = self.now().await;
        self.daemon.again().await;
        self.run_of(step["restart"].as_str().unwrap(), &place, step).await;
    }

    /// Stops the daemon as a step begins, starts it again, and waits for
    /// the run of the handler that follows.
    async fn stopped_at(&mut self, step: &Value) {
        let (case, at) = (self.case.clone(), step["stopped_at"].as_str().unwrap());
        let of = format!("{}/{}", self.package, step["of"].as_str().expect("the step's blueprint, as `of`"));
        let place = self.now().await;
        self.daemon.run(&["down", "--at", at, "--of", &of]).await.ok();
        // Whoever brings the step about may get no answer: the daemon is
        // cut off at the step, and answers nobody after it.
        self.bring_about(&step["by"]).await;
        let waiting = format!("{case}: the daemon to stop as `{at}` begins");
        let stopped = soon(&waiting, self.daemon.child.wait()).await.unwrap();
        assert!(stopped.success(), "{case}: the daemon stopped as `{at}` began");
        if !step["meanwhile"].is_null() {
            self.given(&step["meanwhile"]);
        }
        self.daemon.again().await;
        if let Some(handler) = step["sets_off"].as_str() {
            self.run_of(handler, &place, step).await;
        }
    }

    /// Does what brings a step about, for a daemon that may be gone
    /// before it has answered.
    async fn bring_about(&self, by: &Value) {
        if !by["run"].is_null() {
            let args: Vec<String> = by["run"].as_array().unwrap().iter().map(|arg| self.plain(arg)).collect();
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let _ = self.daemon.run(&args).await;
        } else {
            let mut by = by.clone();
            by["unanswered"] = Value::Bool(true);
            self.call(&by).await;
        }
    }

    /// Stops the daemon while its program is held in the middle of being
    /// asked something, having done it, and starts it again.
    async fn stopped_inside(&mut self, step: &Value) {
        let (case, done) = (self.case.clone(), step["stopped_inside"].as_str().unwrap());
        let pipe = self.standin.join("held");
        let made = std::process::Command::new("mkfifo").arg(&pipe).status().unwrap();
        assert!(made.success(), "{case}: a pipe to hold the stand-in by");
        let place = self.now().await;
        self.bring_about(&step["by"]).await;
        // The stand-in says what it has done, and waits at the pipe.
        let said = soon(&format!("{case}: the stand-in to say it has done `{done}`"), async {
            let pipe = tokio::fs::File::open(&pipe).await.unwrap();
            BufReader::new(pipe).lines().next_line().await.unwrap().unwrap_or_default()
        })
        .await;
        assert_eq!(said, done, "{case}: what the program had done when the daemon was stopped");
        // Gone before the daemon is back: what is asked next is not held.
        std::fs::remove_file(&pipe).unwrap();
        self.daemon.again().await;
        if let Some(handler) = step["sets_off"].as_str() {
            self.run_of(handler, &place, step).await;
        }
    }

    /// Runs the command line as the case's owner, and waits for the run of
    /// the handler it sets off, if the step names one.
    async fn run(&self, step: &Value) {
        let args: Vec<String> = step["run"].as_array().unwrap().iter().map(|arg| self.plain(arg)).collect();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let place = self.before(step).await;
        let ran = self.daemon.run(&args).await;
        let case = &self.case;
        if let Some(sentence) = step["refused"].as_str() {
            let sentence = self.text(sentence);
            assert_ne!(ran.code, 0, "{case}: `{}` should have been refused:\n{}", args.join(" "), ran.out);
            assert!(ran.err.contains(&sentence), "{case}: it was refused, but with:\n{}", ran.err);
            return;
        }
        assert_eq!(ran.code, 0, "{case}: `{}` failed:\n{}\n{}", args.join(" "), ran.out, ran.err);
        if let Some(says) = step["says"].as_str() {
            let says = self.text(says);
            assert!(
                ran.out.contains(&says),
                "{case}: `{}` should say `{says}`:\n{}",
                args.join(" "),
                ran.out
            );
        }
        if let (Some(handler), Some(place)) = (step["sets_off"].as_str(), place) {
            self.run_of(handler, &place, step).await;
        }
    }

    /// Makes a key that holds what the step says and no more.
    async fn key(&mut self, step: &Value) {
        let name = step["key"].as_str().unwrap();
        let mut args = vec!["key".to_string(), name.to_string(), "--json".to_string()];
        for scope in step["scopes"].as_array().expect("a key says its `scopes`") {
            args.extend(["--scope".to_string(), scope.as_str().unwrap().to_string()]);
        }
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let made: Value = serde_json::from_str(&self.daemon.run(&args).await.ok()).unwrap();
        self.keys.insert(name.to_string(), made["token"].as_str().unwrap().to_string());
    }

    /// Calls a caller of the daemon as a service outside does: what it
    /// sends, signed with the secret the case says, and nothing else to
    /// say who it is.
    async fn call(&self, step: &Value) {
        use hmac::{Hmac, KeyInit, Mac};
        let (case, name) = (&self.case, step["call"].as_str().unwrap());
        let body = self.filled(&step["body"]).to_string();
        let mut signing =
            Hmac::<sha2::Sha256>::new_from_slice(self.plain(&step["signed"]).as_bytes()).unwrap();
        signing.update(body.as_bytes());
        let signature: String =
            signing.finalize().into_bytes().iter().map(|byte| format!("{byte:02x}")).collect();
        let prefix = step["prefix"].as_str().unwrap_or_default();
        let place = self.before(step).await;
        let asked = reqwest::Client::new()
            .post(format!("{}/in/{name}", self.daemon.url))
            .header(
                step["header"].as_str().expect("a call says which header its signature is in"),
                format!("{prefix}{signature}"),
            )
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await;
        // A daemon that is stopping as the call lands may answer nobody.
        let answer = match asked {
            Err(_) if step["unanswered"] == true => return,
            asked => asked.unwrap(),
        };
        let status = answer.status();
        let said = answer.text().await.unwrap_or_default();
        if step["refused"] == true {
            assert!(!status.is_success(), "{case}: the call to `{name}` should have been refused: {said}");
            return;
        }
        assert!(status.is_success(), "{case}: the call to `{name}` was refused: {status} {said}");
        if let (Some(handler), Some(place)) = (step["sets_off"].as_str(), place) {
            self.run_of(handler, &place, step).await;
        }
    }

    fn given(&self, files: &Value) {
        for (file, text) in files.as_object().unwrap() {
            let path = self.standin.join(file);
            if text.is_null() {
                let _ = std::fs::remove_file(path);
            } else {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, self.plain(text)).unwrap();
            }
        }
    }

    /// How the program was called since this was last asked.
    fn repository(&self, folder: &str, remote: &str) {
        // The daemon asks git about the folder, so git is found beside the stand-in.
        let git = ["/usr/bin/git", "/bin/git", "/usr/local/bin/git"]
            .into_iter()
            .find(|git| Path::new(git).exists());
        let git = git.expect("git is on this system");
        let beside = self.standin.join("git");
        if !beside.exists() {
            std::os::unix::fs::symlink(git, &beside).unwrap();
        }
        let dir = self.standin.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "# A project\n").unwrap();
        let asked: &[&[&str]] = &[
            &["init", "--quiet", "-b", "main", "."],
            &["config", "user.name", "Ada"],
            &["config", "user.email", "ada@example.com"],
            &["add", "-A"],
            &["commit", "--quiet", "-m", "The first"],
            &["remote", "add", "origin", remote],
        ];
        for args in asked {
            let ran = crate::keyed::asking(&dir).args(*args).output().unwrap();
            assert!(
                ran.status.success(),
                "{}: git {args:?}: {}",
                self.case,
                String::from_utf8_lossy(&ran.stderr)
            );
        }
    }

    fn ran(&self, expected: &Value) {
        let calls = self.standin.join("calls");
        let ran: Vec<String> =
            std::fs::read_to_string(&calls).unwrap_or_default().lines().map(str::to_string).collect();
        let _ = std::fs::remove_file(calls);
        let expected: Vec<String> =
            expected.as_array().unwrap().iter().map(|call| self.plain(call)).collect();
        let same =
            ran.len() == expected.len() && ran.iter().zip(&expected).all(|(ran, expected)| is(ran, expected));
        if !same {
            assert_eq!(ran, expected, "{}: how the program was called", self.case);
        }
    }

    fn holds(&self, folders: &Value) {
        for (folder, expected) in folders.as_object().unwrap() {
            let mut held = BTreeMap::new();
            for entry in std::fs::read_dir(self.standin.join(folder)).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                held.insert(name, std::fs::read_to_string(entry.path()).unwrap());
            }
            let expected: BTreeMap<String, String> = expected
                .as_object()
                .unwrap()
                .iter()
                .map(|(name, text)| (name.clone(), self.plain(text)))
                .collect();
            let same = held.len() == expected.len()
                && held
                    .iter()
                    .zip(&expected)
                    .all(|(held, expected)| held.0 == expected.0 && is(held.1, expected.1));
            if !same {
                assert_eq!(held, expected, "{}: what `{folder}` holds", self.case);
            }
        }
    }
}

async fn play(package: &Path, example: &Path) {
    let case: Value = serde_json::from_str(&std::fs::read_to_string(example).unwrap())
        .unwrap_or_else(|err| panic!("{}: {err}", example.display()));
    let said = example.file_name().unwrap().to_string_lossy().into_owned();

    // Where the daemon looks for programs: the stand-in's folder, and
    // nowhere else. Whatever these tests' own path holds, the real program
    // among it, cannot be run by a case; and a blueprint that reached for a
    // shell, or for any tool but the package's program, would find none.
    let bin = scratch("standin");
    let standin = bin.join("only");
    std::fs::create_dir_all(&standin).unwrap();
    let path = standin.to_str().unwrap();
    let daemon = match case["agent"].as_str() {
        None => Daemon::start_as("serve", Some(path), case["profile"].as_str()).await,
        // A stand-in for an agent's program too, which a fleet's agents
        // run on where their row says `scripted`. A fleet asks git about
        // the project's folder, so git is found beside the stand-in, and
        // nothing else of the system's is.
        Some(script) => {
            let agent = bin.join("agent.sh");
            std::fs::write(&agent, script.replace("{here}", path)).unwrap();
            std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();
            let git = ["/usr/bin/git", "/bin/git", "/usr/local/bin/git"]
                .into_iter()
                .find(|git| Path::new(git).exists());
            std::os::unix::fs::symlink(git.expect("git is on this system"), standin.join("git")).unwrap();
            let runs = format!("scripted={}", agent.display());
            let told = ["--command", runs.as_str(), "--walls", "off"];
            Daemon::start_told("serve", Some(path), case["profile"].as_str(), &told).await
        }
    };
    if let Some(name) = case["profile"].as_str() {
        let status = daemon.run(&["status"]).await.ok();
        assert!(
            status.contains(&format!("({name})")),
            "{said}: the case's daemon is the `{name}` factory: {status}"
        );
    }
    daemon
        .run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    let added = daemon.run(&["extension", "add", package.to_str().unwrap(), "--json"]).await.ok();
    let added: Value = serde_json::from_str(&added).unwrap();
    let program = added["program"].as_str().unwrap();
    assert!(!program.is_empty(), "{said}: the package names the program its blueprints run");
    // A case may be about the program not being there at all.
    if case["program"] != false {
        let placed = standin.join(program);
        std::fs::copy(package.join("examples/stand-in.sh"), &placed).unwrap();
        std::fs::set_permissions(&placed, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut stage = Stage {
        daemon,
        standin,
        package: added["name"].as_str().unwrap().to_string(),
        instances: BTreeMap::new(),
        keys: BTreeMap::new(),
        words: std::fs::read_to_string(package.join("examples/words.json"))
            .map(|words| serde_json::from_str(&words).expect("a package's words.json is names and texts"))
            .unwrap_or_default(),
        case: said,
        reader: tokio::sync::OnceCell::new(),
    };
    for step in case["steps"].as_array().expect("a case has `steps`") {
        if !step["given"].is_null() {
            stage.given(&step["given"]);
        } else if !step["apply"].is_null() {
            stage.apply(step).await;
        } else if let Some(blueprint) = step["stop"].as_str() {
            let instance = stage.instances[blueprint].clone();
            stage.daemon.run(&["stop", &instance]).await.ok();
        } else if !step["stopped_at"].is_null() {
            stage.stopped_at(step).await;
        } else if !step["stopped_inside"].is_null() {
            stage.stopped_inside(step).await;
        } else if !step["restart"].is_null() {
            stage.restart(step).await;
        } else if !step["run"].is_null() {
            stage.run(step).await;
        } else if !step["key"].is_null() {
            stage.key(step).await;
        } else if !step["call"].is_null() {
            stage.call(step).await;
        } else if let Some(folder) = step["repository"].as_str() {
            stage.repository(folder, step["remote"].as_str().unwrap_or_default());
        } else if !step["ran"].is_null() {
            stage.ran(&step["ran"]);
        } else if !step["holds"].is_null() {
            stage.holds(&step["holds"]);
        } else {
            panic!(
                "{}: a step that is none of given, apply, stop, stopped_at, stopped_inside, restart, run, key, call, repository, ran, holds: {step}",
                stage.case
            );
        }
    }
    let _ = std::fs::remove_dir_all(bin);
}

#[tokio::test(flavor = "multi_thread")]
async fn every_package_here_that_brings_blueprints_does_what_its_cases_say() {
    let cases = cases();
    assert!(!cases.is_empty(), "there are packages whose blueprints are held to their word");
    // Each has a daemon of its own, so they do not wait for one another,
    // but only as many are played at once as there are processors: every
    // one of them started together, forty daemons on four processors, a
    // command of one was not answered in the time a command is given.
    let room = std::thread::available_parallelism().map_or(2, |processors| processors.get().max(2));
    let at_once = std::sync::Arc::new(tokio::sync::Semaphore::new(room));
    let playing: Vec<_> = cases
        .into_iter()
        .map(|(package, example)| {
            let at_once = at_once.clone();
            tokio::spawn(async move {
                let _turn = at_once.acquire_owned().await.expect("nothing closes it");
                play(&package, &example).await
            })
        })
        .collect();
    let mut failed = Vec::new();
    for case in playing {
        if let Err(err) = case.await {
            failed.push(err.to_string());
        }
    }
    assert!(failed.is_empty(), "{} case(s) failed; each says which above", failed.len());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_package_that_brings_blueprints_is_added_listed_and_applied_from_the_command_line() {
    // A package for a product there is not, which runs no program.
    let rig = crate::Rig::signed_in().await;
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let manifest = "\
extension: reach
title: Reach
blueprints:
  open: |
    blueprint: reach/open
    params:
      door: 9
    steps:
      knock: { do: noop }
    outputs:
      door: ${ params.door }
";
    std::fs::write(folder.join("extension.yaml"), manifest).unwrap();
    let cli = brand::CLI;

    let added = rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    assert_eq!(added, format!("`reach` is installed\nit brings `reach/open`: `{cli} apply reach/open`\n"));
    let listed = rig.run(&["extension", "list"]).await.ok();
    assert_eq!(
        listed, "reach  Reach  ready  reach/open\n",
        "what it brings, and that nothing it needs is missing"
    );

    let applied = rig.run(&["apply", "reach/open", "--param", "door=3"]).await.ok();
    assert!(applied.ends_with("outputs: {\"door\":3}\n"), "{applied}");

    let removed = rig.run(&["extension", "remove", "reach"]).await.ok();
    assert!(removed.starts_with("`reach` is removed"), "{removed}");
    let gone = rig.run(&["apply", "reach/open", "--key", "again"]).await;
    assert_eq!(gone.code, 1, "{}", gone.out);
    assert!(gone.err.contains("there is no blueprint called `reach/open`"), "{}", gone.err);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_package_that_brings_actions_says_which_and_a_blueprint_does_them_from_the_command_line() {
    // A package for a product there is not, whose program is a script.
    let rig = crate::Rig::signed_in().await;
    let folder = rig.dir.join("reach");
    std::fs::create_dir_all(&folder).unwrap();
    let program = folder.join("reach-program");
    std::fs::write(&program, "#!/bin/sh\necho \"knocked $2 times on door $1\"\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let manifest = format!(
        "\
extension: reach
title: Reach
program: {}
actions:
  knock:
    description: Knock on a door.
    with:
      door: {{ type: int, required: true, description: \"Which door.\" }}
      times: {{ type: int, default: 3, description: \"How often.\" }}
    args: [\"${{ with.door }}\", \"${{ with.times }}\"]
    out:
      heard: ${{ trim(answer) }}
blueprints:
  open: |
    blueprint: reach/open
    permissions: [extensions:reach]
    steps:
      knock: {{ do: reach.knock, with: {{ door: 9 }} }}
    outputs:
      heard: ${{ steps.knock.out.heard }}
",
        program.display()
    );
    std::fs::write(folder.join("extension.yaml"), manifest).unwrap();
    let cli = brand::CLI;

    let added = rig.run(&["extension", "add", folder.to_str().unwrap()]).await.ok();
    assert_eq!(
        added,
        format!(
            "`reach` is installed\nit brings actions for a blueprint to `do`: reach.knock (`{cli} actions \
             reach.knock`)\nit brings `reach/open`: `{cli} apply reach/open`\n"
        )
    );
    let listed = rig.run(&["extension", "list"]).await.ok();
    assert_eq!(
        listed,
        format!(
            "reach  Reach  ready  reach/open\nactions they bring, for a blueprint to `do` (`{cli} actions \
             <name>` says more):\n  reach.knock  door, [times] -> heard  Knock on a door.\n"
        ),
        "each with what it takes, what may be left out in brackets, and what it gives"
    );

    // Among the device's actions, described as they are.
    let all = rig.run(&["actions"]).await.ok();
    assert!(all.lines().any(|line| line.starts_with("reach.knock") && line.ends_with("Knock on a door.")));
    let one = rig.run(&["actions", "reach.knock"]).await.ok();
    assert!(
        one.starts_with("reach.knock: Knock on a door.\nneeds: extensions:reach\nwith:\n")
            && one.contains("  door   int  required  Which door.")
            && one.contains("  times  int            How often."),
        "{one}"
    );

    let applied = rig.run(&["apply", "reach/open"]).await.ok();
    assert!(applied.ends_with("outputs: {\"heard\":\"knocked 3 times on door 9\"}\n"), "{applied}");

    // Removed, a blueprint that does one is told what is missing.
    rig.run(&["extension", "remove", "reach"]).await.ok();
    let source = rig.dir.join("knock.bp");
    std::fs::write(&source, "blueprint: knock\nsteps:\n  it: { do: reach.knock, with: { door: 1 } }\n")
        .unwrap();
    let gone = rig.run(&["apply", source.to_str().unwrap()]).await;
    assert_eq!(gone.code, 1, "{}", gone.out);
    assert!(gone.err.contains("there is no action called `reach.knock`"), "{}", gone.err);
}
