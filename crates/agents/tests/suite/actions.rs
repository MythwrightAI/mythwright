//! The actions a package declares: its program run by the application,
//! with no shell, and read with the package's own expressions.
//!
//! The package here is made up, for a product there is not, and its
//! program is a script written here. The real packages kept beside the
//! application are held to their own examples elsewhere.
//!
//! Unix's alone: the package's program is a file of a Unix shell that
//! takes commands of its own, made a program by its mode.
#![cfg(unix)]

use std::path::Path;
use std::time::Duration;

use agents::Extension;
use conductor::{Apply, RunInfo, RunStatus, StepInfo, StepStatus};
use kernel::{Principal, PrincipalKind, Source};
use serde_json::json;

use crate::{Rig, ada, alive, soon};

/// A program for a product there is not: it writes down how it was called
/// and answers as such a program would, each way of being called showing
/// one thing a program can do.
fn program_in(dir: &Path) -> String {
    let here = dir.display();
    let (url, cli, environment) = (hosting::ADDRESS_ENV, hosting::PROGRAM_ENV, hosting::ENVIRONMENT_ENV);
    let script = format!(
        r#"#!/bin/sh
echo "$*" >> "{here}/calls"
case "$1" in
  status) printf '{{\n  "state": "%s",\n  "self": {{ "name": "box.example." }}\n}}\n' "$(cat "{here}/state" 2>/dev/null || echo up)" ;;
  told) printf '%s\n' "${url}" "${{{cli}:+its own program}}" "${{{environment}-unset}}" "$SAID_BY_THE_STEP" ;;
  words) echo "not json at all" ;;
  serve) echo "serving $3 on $2" ;;
  stop) echo "stopped" ;;
  break) echo "warming up" >&2; echo "it broke" >&2; exit 3 ;;
  locked) echo "run it with --operator first" >&2; exit 1 ;;
  hang) sleep 300 & echo $! > "{here}/child"; echo $$ >> "{here}/child"; wait ;;
  flood) echo $$ > "{here}/child"; while :; do echo 0123456789012345678901234567890123456789012345678901234567890123456789; done ;;
esac
"#
    );
    let program = dir.join("example-program");
    crate::program(&program, &script);
    program.display().to_string()
}

/// A package that declares an action for each of them.
fn package(program: &str) -> String {
    format!(
        r#"
extension: example
title: Example
description: Not a real product.
program: {program}
install: "It is to be had from nobody."
actions:
  status:
    description: Say whether it is up, and what this device is called.
    args: ["status", "--json"]
    answer: json
    fails:
      - when: ${{ answer.state != "up" }}
        says: "It is ${{ answer.state }}, not up. Start it and do this again."
    out:
      name: ${{ replace(answer.self.name, ".example.", "") }}
    repeatable: true
  serve:
    description: Serve something on a port.
    makes: {{ what: something served on the port, named: port }}
    with:
      port: {{ type: int, required: true, description: "The port." }}
      to: {{ type: string, default: "${{ daemon.url }}", description: "What is served; the daemon, unless said." }}
      loud: {{ type: bool, default: false, description: "Say so loudly." }}
    refuse:
      - when: ${{ with.port < 1024 }}
        says: "Port ${{ with.port }} is not one this may serve on; choose one from 1024 up."
      - when: ${{ !starts_with(with.to, "http://") }}
        says: "`to` must be a full address; it is ${{ with.to }}."
    args:
      - ["serve", "--port=${{ with.port }}", "${{ with.to }}"]
      - when: ${{ with.loud }}
        args: ["--loud"]
    out:
      to: ${{ with.to }}
      said: ${{ trim(answer) }}
  stop:
    description: Stop serving, but only on the device it was started on.
    with:
      on: {{ type: string, required: true, description: "What that device is called." }}
    first: status
    refuse:
      - when: ${{ first.name != with.on }}
        says: "This device is ${{ first.name }}, not ${{ with.on }}, so nothing was stopped."
    args: ["stop"]
  told:
    description: Say what the program is told about where it runs.
    args: ["told"]
    out:
      lines: ${{ split(trim(answer), "\n") }}
  words:
    description: Ask for JSON of a program that prints something else.
    args: ["words"]
    answer: json
    fails:
      - when: ${{ answer == null }}
        says: "It did not answer in a way this understands. It said: ${{ trim(stdout) }}"
  break:
    description: Run it in a way that breaks.
    args: ["break"]
  locked:
    description: Run it in a way it will not take from just anyone.
    args: ["locked"]
    fails:
      - when: ${{ code != 0 && contains(stderr, "--operator") }}
        says: "It takes this from its operator only. Become that, and do this again."
  hang:
    description: Run it in a way that never ends.
    args: ["hang"]
  flood:
    description: Run it in a way that never stops talking.
    args: ["flood"]
"#
    )
}

/// A rig with the package installed, and where its program keeps its notes.
async fn installed() -> Rig {
    let rig = Rig::new().await;
    rig.agents.add_extension(&package(&program_in(&rig.dir)), &ada()).await.unwrap();
    rig
}

/// How the program was called, a line a time.
fn calls(rig: &Rig) -> Vec<String> {
    std::fs::read_to_string(rig.dir.join("calls")).unwrap_or_default().lines().map(str::to_string).collect()
}

/// A blueprint that does one action of the package and gives out what it gave.
fn doing(name: &str, action: &str, with: &str) -> String {
    format!(
        "blueprint: {name}\npermissions: [extensions:example]\nsteps:\n  it: {{ do: example.{action}, with: {with} }}\noutputs:\n  gave: ${{ steps.it.out }}\n"
    )
}

async fn applied_by(rig: &Rig, who: Principal, source: &str) -> Result<RunInfo, String> {
    let name = blueprint::check(source, &blueprint::NoCatalog).blueprint.map(|b| b.name).unwrap_or_default();
    let request = Apply { source: Some(source.to_string()), ..Apply::new(name, who) };
    let applied = rig.engine.apply(request).await.map_err(|err| err.message)?;
    Ok(soon("the run to finish", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap())
}

async fn run(rig: &Rig, source: &str) -> RunInfo {
    applied_by(rig, ada(), source).await.unwrap_or_else(|err| panic!("apply failed: {err}"))
}

fn step<'a>(run: &'a RunInfo, id: &str) -> &'a StepInfo {
    run.steps.iter().find(|step| step.step == id).unwrap()
}

/// The sentence a run of one action failed with.
fn failure(run: &RunInfo) -> String {
    assert_eq!(run.status, RunStatus::Failed, "{:?}", run.outputs);
    step(run, "it").error.clone().unwrap()
}

/// The processes the program wrote down, once it has written `count` of them.
async fn started(rig: &Rig, count: usize) -> Vec<String> {
    let file = rig.dir.join("child");
    soon("the program to start", async {
        loop {
            let pids: Vec<String> =
                std::fs::read_to_string(&file).unwrap_or_default().lines().map(str::to_string).collect();
            if pids.len() >= count {
                return pids;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

async fn all_dead(pids: &[String]) {
    soon("its processes to die", async {
        while pids.iter().any(|pid| alive(pid)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_blueprint_does_a_packages_action_and_the_program_is_run_as_the_package_says() {
    let rig = installed().await;
    let source = "\
blueprint: reach
permissions: [extensions:example]
steps:
  up: { do: example.status }
  serve: { do: example.serve, with: { port: 8443, loud: true } }
  elsewhere: { do: example.serve, with: { port: 9000, to: \"http://127.0.0.1:3000\" } }
outputs:
  name: ${ steps.up.out.name }
  first: ${ steps.serve.out }
  second: ${ steps.elsewhere.out.to }
";
    let run = run(&rig, source).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        run.outputs,
        json!({
            "name": "box",
            "first": { "to": "http://127.0.0.1:7700", "said": "serving http://127.0.0.1:7700 on --port=8443" },
            "second": "http://127.0.0.1:3000",
        }),
        "what was left out is the daemon's own address, and what it printed is read"
    );
    assert_eq!(
        calls(&rig),
        [
            "status --json",
            "serve --port=8443 http://127.0.0.1:7700 --loud",
            "serve --port=9000 http://127.0.0.1:3000"
        ],
        "each argument as the package wrote it, and those with a condition only when it holds"
    );

    // It is a step like any other: what it did is under the action's name.
    let begun = rig.events("step.started").await;
    let targets: Vec<&str> = begun.iter().filter_map(|event| event.data["target"].as_str()).collect();
    assert_eq!(targets, ["example.status", "example.serve", "example.serve"]);
    let spec = rig.engine.registry().get("example.status").unwrap().spec().clone();
    assert!(spec.idempotent, "one the package calls `repeatable` may be run again after an interruption");
    let serve = rig.engine.registry().get("example.serve").unwrap().spec().clone();
    assert!(!serve.idempotent);
    // What a package says its action makes is on the action, for whoever
    // says in words what a blueprint that asks it does; one that says
    // nothing makes nothing that outlasts it.
    let made = serve.makes.expect("it says what it makes");
    assert_eq!((made.what.as_str(), made.named.as_str()), ("something served on the port", "port"));
    assert_eq!(spec.makes, None);
}

#[tokio::test]
async fn what_a_step_gives_is_refused_in_the_packages_words_before_the_program_is_run() {
    let rig = installed().await;
    let low = run(&rig, &doing("low", "serve", "{ port: 80 }")).await;
    assert_eq!(failure(&low), "Port 80 is not one this may serve on; choose one from 1024 up.");
    let bare = run(&rig, &doing("bare", "serve", "{ port: 8443, to: \"127.0.0.1:3000\" }")).await;
    assert_eq!(failure(&bare), "`to` must be a full address; it is 127.0.0.1:3000.");
    assert_eq!(calls(&rig), Vec::<String>::new(), "the program was never started");

    // Its arguments are checked as any action's are.
    let wrong = applied_by(&rig, ada(), &doing("wrong", "serve", "{ port: many }")).await.unwrap_err();
    assert!(wrong.contains("`example.serve`: `port` must be int"), "{wrong}");
    let missing =
        applied_by(&rig, ada(), &doing("missing", "serve", "{ to: \"http://x\" }")).await.unwrap_err();
    assert!(missing.contains("`example.serve` needs `port` (The port.)"), "{missing}");
}

#[tokio::test]
async fn what_the_program_did_is_a_failure_in_the_packages_words_or_else_in_the_programs_own() {
    let rig = installed().await;
    // The package says what this means.
    std::fs::write(rig.dir.join("state"), "down").unwrap();
    let down = run(&rig, &doing("down", "status", "{}")).await;
    assert_eq!(failure(&down), "It is down, not up. Start it and do this again.");
    let locked = run(&rig, &doing("locked", "locked", "{}")).await;
    assert_eq!(failure(&locked), "It takes this from its operator only. Become that, and do this again.");
    assert_eq!(step(&locked, "it").output["code"], 1, "what the program did is kept with the failure");
    assert_eq!(step(&locked, "it").output["stderr"], "run it with --operator first");

    // The package has nothing to say: the program's last words are it.
    let broke = run(&rig, &doing("broke", "break", "{}")).await;
    let program = rig.dir.join("example-program");
    assert_eq!(failure(&broke), format!("`{}` exited with code 3: it broke", program.display()));

    // Asked for JSON, what came was not: the answer is nothing, and the
    // package says what that means.
    let words = run(&rig, &doing("words", "words", "{}")).await;
    assert_eq!(failure(&words), "It did not answer in a way this understands. It said: not json at all");
}

#[tokio::test]
async fn an_action_can_be_done_after_another_and_decide_by_what_that_gave_out() {
    let rig = installed().await;
    let here = run(&rig, &doing("here", "stop", "{ on: box }")).await;
    assert_eq!(here.status, RunStatus::Ok, "{:?}", here.error);
    assert_eq!(calls(&rig), ["status --json", "stop"]);

    let elsewhere = run(&rig, &doing("elsewhere", "stop", "{ on: shed }")).await;
    assert_eq!(failure(&elsewhere), "This device is box, not shed, so nothing was stopped.");
    assert_eq!(calls(&rig).len(), 3, "it was asked again, and not stopped");

    // The one done first failing is this one failing, in its words.
    std::fs::write(rig.dir.join("state"), "down").unwrap();
    let down = run(&rig, &doing("down", "stop", "{ on: box }")).await;
    assert_eq!(failure(&down), "It is down, not up. Start it and do this again.");
}

#[tokio::test]
async fn the_program_is_told_what_a_command_a_blueprint_runs_is_told() {
    let rig = installed().await;
    let source = "\
blueprint: told
permissions: [extensions:example]
env: { SAID_BY_THE_STEP: \"port ${ 8000 + 443 }\" }
steps:
  it: { do: example.told }
outputs:
  lines: ${ steps.it.out.lines }
";
    let run = run(&rig, source).await;
    assert_eq!(
        run.outputs["lines"],
        json!(["http://127.0.0.1:7700", "its own program", "unset", "port 8443"]),
        "where the daemon is, where the product's own program is, and what the blueprint's `env` says"
    );
}

/// Whether any file under `dir` holds `text`.
fn held_in(dir: &Path, text: &str) -> bool {
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|entry| match entry.path() {
        path if path.is_dir() => held_in(&path, text),
        path => std::fs::read(path)
            .is_ok_and(|bytes| bytes.windows(text.len()).any(|part| part == text.as_bytes())),
    })
}

#[tokio::test]
async fn a_packages_program_that_prints_the_secret_it_was_handed_gives_its_name_and_never_its_value() {
    let rig = installed().await;
    let value = "tok-4f7a91c2d8e6b3";
    rig.hosting.secrets().unwrap().set("example-token", value).unwrap();
    let source = "\
blueprint: told
permissions: [extensions:example, shell, secrets]
env: { SAID_BY_THE_STEP: { secret: example-token } }
steps:
  it: { do: example.told }
outputs:
  lines: ${ steps.it.out.lines }
";
    let run = run(&rig, source).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    // What it printed is read by the daemon itself, a line at a time, and
    // is without the value before anything is made of it.
    assert_eq!(run.outputs["lines"][3], "[the secret `example-token`]");
    let said = serde_json::to_string(&rig.events("**").await).unwrap();
    assert!(!said.contains(value), "{said}");
    assert!(!held_in(&rig.dir, value), "a file under {} holds the value", rig.dir.display());
}

#[tokio::test]
async fn a_program_that_is_not_on_the_device_is_said_so_with_what_to_do() {
    let rig = Rig::new().await;
    rig.agents.add_extension(&package("no-such-program-anywhere"), &ada()).await.unwrap();
    let run = run(&rig, &doing("absent", "status", "{}")).await;
    assert_eq!(
        failure(&run),
        "The program `no-such-program-anywhere` is not installed on this device, or not where the daemon \
         looks for programs. Install it and do this again: it is what the extension package `example` \
         runs. It is to be had from nobody."
    );
}

#[tokio::test]
async fn doing_a_packages_action_takes_leave_to_use_that_package_and_not_a_shell() {
    let rig = installed().await;
    let with = |scopes: &[&str]| Principal {
        kind: PrincipalKind::Agent,
        id: "builder".into(),
        name: "builder".into(),
        scopes: scopes.iter().map(|scope| scope.to_string()).collect(),
        key: None,
    };
    let source = doing("asked", "status", "{}");

    // Leave to run any command is not leave to use a package.
    let shell = applied_by(&rig, with(&["blueprints", "shell"]), &source).await.unwrap();
    assert_eq!(failure(&shell), "builder may not use `extensions:example`");
    assert_eq!(calls(&rig), Vec::<String>::new());

    // Leave for this package is enough, with no shell; so is leave for all
    // of them; leave for another package is not.
    let one = applied_by(&rig, with(&["blueprints", "extensions:example"]), &source).await.unwrap();
    assert_eq!(one.status, RunStatus::Ok, "{:?}", one.error);
    let all = applied_by(&rig, with(&["blueprints", "extensions"]), &source).await.unwrap();
    assert_eq!(all.status, RunStatus::Ok, "{:?}", all.error);
    let other = applied_by(&rig, with(&["blueprints", "extensions:another"]), &source).await.unwrap();
    assert_eq!(failure(&other), "builder may not use `extensions:example`");

    // A blueprint that lists what it may use has to list this.
    let unlisted = source.replace("[extensions:example]", "[shell]");
    let said = applied_by(&rig, ada(), &unlisted).await.unwrap_err();
    assert!(
        said.contains("this uses `extensions:example`, which is not listed under `permissions`"),
        "{said}"
    );
    assert!(kernel::principal::MEMBER_SCOPES.contains(&"extensions"), "a person may use them by default");
    assert!(
        !kernel::principal::AGENT_SCOPES.iter().any(|scope| scope.starts_with("extensions")),
        "an agent may not, unless it is given leave"
    );
}

#[tokio::test]
async fn a_packages_actions_are_there_once_it_is_installed_and_gone_when_it_is_removed() {
    let rig = Rig::new().await;
    let listed =
        |rig: &Rig| rig.engine.registry().all_specs().iter().any(|spec| spec.name == "example.status");
    let source = "\
blueprint: watch
permissions: [extensions:example]
steps:
  it: { do: example.status }
on:
  asked: { event: asked.again, do: example.status }
";
    assert!(!listed(&rig));
    let unknown = applied_by(&rig, ada(), source).await.unwrap_err();
    assert!(unknown.contains("there is no action called `example.status`"), "{unknown}");

    // Installed while the conductor runs, it is there at once: listed,
    // checked against, and done.
    let manifest = package(&program_in(&rig.dir));
    rig.agents.add_extension(&manifest, &ada()).await.unwrap();
    assert!(listed(&rig));
    let described = rig.engine.registry().all_specs().into_iter().find(|spec| spec.name == "example.serve");
    let described = serde_json::to_value(described.unwrap()).unwrap();
    assert_eq!(
        (&described["summary"], &described["capability"], &described["args"][1], &described["outputs"]),
        (
            &json!("Serve something on a port."),
            &json!("extensions:example"),
            &json!({ "name": "port", "type": "int", "required": true, "help": "The port." }),
            &json!([["said", ""], ["to", ""]]),
        ),
        "described as every action is"
    );
    let misspelt =
        applied_by(&rig, ada(), &source.replace("example.status }\non", "example.statu }\non")).await;
    assert!(
        misspelt.unwrap_err().contains(
            "the extension package `example` has no action called `statu`; it has: break, flood, hang"
        ),
        "an action the package does not have is said to be the package's to have"
    );
    let first = run(&rig, source).await;
    assert_eq!(first.status, RunStatus::Ok, "{:?}", first.error);

    // Removed, it is gone: not listed, and a step of what is still running
    // says why it can no longer be done.
    rig.agents.remove_extension("example", &ada()).await.unwrap();
    assert!(!listed(&rig));
    rig.journal.emit("asked.again", Source::actor("user:ada"), json!({})).await.unwrap();
    let handled = rig.event("run.finished", |event| event.data["flow"] == "on.asked").await;
    let said = handled.data["error"].as_str().unwrap();
    assert!(
        said.contains(
            "`example.status` came with the extension package `example`, which is not installed on this \
             device any more."
        ),
        "{said}"
    );

    // Installed again, the same instance does it again.
    rig.agents.add_extension(&manifest, &ada()).await.unwrap();
    rig.journal.emit("asked.again", Source::actor("user:ada"), json!({})).await.unwrap();
    let again =
        rig.event("run.finished", |event| event.data["flow"] == "on.asked" && event.seq > handled.seq).await;
    assert_eq!(again.data["status"], "ok", "{}", again.data);
}

#[tokio::test]
async fn an_action_of_a_package_that_is_only_on_offer_is_said_to_need_installing() {
    let rig = Rig::new().await;
    let offered = rig.dir.join("offered/example");
    std::fs::create_dir_all(&offered).unwrap();
    std::fs::write(offered.join(agents::MANIFEST), package("example-program")).unwrap();
    let said = applied_by(&rig, ada(), &doing("early", "status", "{}")).await.unwrap_err();
    assert!(
        said.contains(&format!(
            "`example.status` comes with the extension package `example`, which is not installed on this \
             device; install it with `{} extension add example`",
            brand::CLI
        )),
        "{said}"
    );
    // A name that is no package's is a name that is not known, as ever.
    let said = applied_by(&rig, ada(), &doing("typo", "status", "{}").replace("example.status", "bord.add"))
        .await
        .unwrap_err();
    assert!(said.contains("there is no action called `bord.add`; did you mean `board.add`?"), "{said}");
}

#[tokio::test]
async fn a_step_that_is_cancelled_or_runs_out_of_time_takes_the_program_and_what_it_started_with_it() {
    let rig = installed().await;
    let source = |name: &str| {
        format!(
            "blueprint: {name}\npermissions: [extensions:example]\nsteps:\n  it: {{ do: example.hang, timeout: 30s }}\n"
        )
    };

    // Stopped by hand.
    let request = Apply { source: Some(source("stopped")), ..Apply::new("stopped", ada()) };
    let applied = rig.engine.apply(request).await.unwrap();
    let pids = started(&rig, 2).await;
    assert!(pids.iter().all(|pid| alive(pid)));
    rig.engine.stop(&applied.instance).await.unwrap();
    all_dead(&pids).await;

    // Out of time, by the step's own `timeout`.
    std::fs::remove_file(rig.dir.join("child")).unwrap();
    let request = Apply { source: Some(source("late")), ..Apply::new("late", ada()) };
    let applied = rig.engine.apply(request).await.unwrap();
    let pids = started(&rig, 2).await;
    rig.clock.advance(Duration::from_secs(31));
    let run = soon("the run to finish", rig.engine.wait_run(applied.run.as_ref().unwrap())).await.unwrap();
    assert_eq!(
        (step(&run, "it").status, step(&run, "it").error.as_deref()),
        (StepStatus::Failed, Some("timed out after 30s"))
    );
    all_dead(&pids).await;
}

#[tokio::test]
async fn a_program_that_prints_more_than_is_read_is_stopped_and_nothing_is_taken_from_it() {
    let rig = installed().await;
    let run = run(&rig, &doing("flood", "flood", "{}")).await;
    let said = failure(&run);
    assert!(
        said.ends_with(
            "printed more than 4 MB, which is more than is read of an answer, and was stopped. Nothing was \
             taken from what it printed."
        ),
        "{said}"
    );
    all_dead(&started(&rig, 1).await).await;
}

#[test]
fn a_manifest_whose_actions_are_wrong_is_refused_with_each_mistake_and_where_it_is() {
    let wrong = r#"
extension: example
program: example-program
actions:
  Serve It:
    description: Badly named.
  status:
    args: ["status"]
    answr: json
    out:
      name: ${ answer.name }
  serve:
    description: Serve something.
    with:
      Port: { type: int }
      port: { type: whole, required: true, default: 80 }
      to: { type: string, default: "${ with.port }" }
      loud: { type: bool, default: "yes", described: "Loudly." }
    refuse:
      - when: ${ with.prot < 1024 }
        says: "Too low."
      - when: ${ answer == null }
        says: "Too soon."
      - says: "No condition."
    args: ["serve", "${ params.port }", "${ with.port + }"]
    answer: xml
    fails: nope
    out: [name]
    repeatable: sometimes
    makes: { what: a thing, named: prot, when: now }
  stop:
    description: Stop.
    first: status
    refuse:
      - when: ${ first.state == "up" }
        says: "It is ${ daemon.host }."
    args: ["stop"]
  again:
    description: Stop again.
    first: stop
    makes: { what: a thing, named: port }
    args: ["stop"]
  alone:
    description: After nothing.
    first: nothing
    refuse:
      - when: ${ first.name == null }
        says: "Nothing."
  after:
    description: After one that needs what this does not take.
    first: needy
  needy:
    description: Needs something.
    with:
      what: { type: string, required: true }
"#;
    let Err(problems) = Extension::load(wrong) else { panic!("this must not load") };
    let all = problems.join("\n");
    for expected in [
        "`actions.Serve It` must be named in lowercase letters, digits and dashes",
        "`actions.status.answr` is not a known field; it can have: description, makes, with, first, refuse, args, answer, fails, out, repeatable",
        "`actions.status.description` is needed",
        "`actions.serve.with.Port` must be named in lowercase letters, digits and `_`, starting with a letter",
        "`actions.serve.with.port.type` must be one of: string, int, number, bool, list, object, duration, any",
        "`actions.serve.with.port` is `required` and has a `default`, which it would never be given; say one",
        "`actions.serve.with.to.default` reads `with`; a default can read `daemon` and nothing else",
        "`actions.serve.with.loud.described` is not a known field; it can have: type, required, default, description",
        "`actions.serve.with.loud.default` must be bool, as its `type` says",
        "`actions.serve.refuse[0].when` reads `with.prot`, and `with` has: loud, port, to",
        "`actions.serve.refuse[1].when` reads `answer`, which is not there to read; here there are: with, daemon; `answer` is there once the program has run",
        "`actions.serve.refuse[2]` needs `when`, a condition, and `says`, the sentence it fails with",
        "`actions.serve.args[1]` reads `params`, which is not there to read; here there are: with, daemon",
        "`actions.serve.args[2]` has `${ with.port + }`, which does not read as an expression",
        "`actions.serve.answer` must be `json`, for a program that prints one JSON value, or `text`",
        "`actions.serve.fails` must be a list, each with `when` and `says`",
        "`actions.serve.out` must be a mapping from a name to the expression that is given out under it",
        "`actions.serve.repeatable` must be `true` or `false`",
        "`actions.serve.makes.when` is not a known field; it can have: what, named",
        "`actions.again.makes.named` must be one of the arguments under `with`: the one that says what the thing made is called",
        "`actions.stop.refuse[0].when` reads `first.state`, and `first` has: name",
        "`actions.stop.refuse[0].says` reads `daemon.host`, and `daemon` has: url, user",
        "`actions.again.first` names `stop`, which has a `first` of its own; only one action is done before another",
        "`actions.alone.first` names `nothing`, which is not an action of this package; it has: after, again, alone, needy, serve, status, stop",
        "`actions.after.first` names `needy`, which needs `what`: `needy` is given the arguments of this action that it takes too, so this one has to take `what`",
    ] {
        assert!(all.contains(expected), "missing: {expected}\n{all}");
    }

    // A package may bring actions and nothing else, and then says which
    // program they run.
    let only = "extension: example\nactions:\n  status:\n    description: Ask.\n    args: [\"status\"]\n";
    let Err(problems) = Extension::load(only) else { panic!("actions with no program must not load") };
    assert_eq!(problems, ["`program` is needed: it is what the package's actions run"]);
    let only = Extension::load(&format!("program: example-program\n{only}")).unwrap();
    assert_eq!(only.actions.keys().collect::<Vec<_>>(), ["status"]);
    assert!(!only.agent && only.blueprints.is_empty());
    let Err(problems) = Extension::load("extension: example\nactions: {}\n") else { panic!("must not load") };
    assert!(problems.join("\n").contains("`actions` is empty: leave it out if the package brings none"));
}

#[test]
fn a_packages_blueprints_are_checked_against_the_actions_it_declares() {
    let package = |blueprint: &str| {
        format!(
            "extension: example\nprogram: example-program\nactions:\n  status:\n    description: Ask.\n    \
             with:\n      loudly: {{ type: bool }}\n    args: [\"status\"]\nblueprints:\n  up: |\n{blueprint}"
        )
    };
    let good = "    blueprint: example/up\n    permissions: [extensions:example]\n    steps:\n      \
                ask: { do: example.status, with: { loudly: true } }\n      \
                other: { do: another.thing }\n";
    let loaded = Extension::load(&package(good)).unwrap_or_else(|problems| panic!("{problems:#?}"));
    assert_eq!(loaded.actions["status"].spec.name, "example.status");

    let wrong = |blueprint: &str| Extension::load(&package(blueprint)).err().unwrap().join("\n");
    let undeclared = wrong(&good.replace("example.status", "example.state"));
    assert!(
        undeclared.contains(
            "`blueprints.up` does not check: line 4: this package declares no action called `state`; it \
             has: example.status"
        ),
        "{undeclared}"
    );
    let unlisted = wrong(&good.replace("[extensions:example]", "[shell]"));
    assert!(
        unlisted.contains("this uses `extensions:example`, which is not listed under `permissions`"),
        "{unlisted}"
    );
    let misgiven = wrong(&good.replace("loudly: true", "quietly: true"));
    assert!(misgiven.contains("`example.status` has no argument `quietly`; it takes: loudly"), "{misgiven}");
}

#[tokio::test]
async fn a_package_that_brings_actions_cannot_be_called_what_the_devices_own_actions_are_under() {
    let rig = Rig::new().await;
    let manifest = package("example-program").replace("extension: example", "extension: board");
    let refused = rig.agents.add_extension(&manifest, &ada()).await.err().unwrap();
    assert_eq!(
        refused.message,
        "`board` is already what actions of this device's own are called under (`board.<name>`), so a \
         package that brings actions cannot have that name; call the package something else"
    );
    assert!(rig.agents.extension("board").is_none());
    // One that brings none says nothing an action's name could be taken for.
    let blueprints = "extension: board\nblueprints:\n  up: |\n    blueprint: board/up\n    steps:\n      go: { do: noop }\n";
    rig.agents.add_extension(blueprints, &ada()).await.unwrap();
}

#[tokio::test]
async fn a_package_brings_the_files_its_blueprints_link_found_beside_them_and_listed_among_no_blueprints() {
    let rig = Rig::new().await;
    let manifest = "\
extension: reach
blueprints:
  ask: |
    blueprint: reach/ask
    steps:
      words: { do: text.put, with: { parts: [{ from: ask.md, with: { item: shop-12 } }] } }
prompts:
  ask.md: |
    Look at ${item}.
  seat/among.md: |
    You are one of ${among}.
";
    rig.agents.add_extension(manifest, &ada()).await.unwrap();
    let library = rig.agents.library();
    // Each is a text of the library under the name its blueprint links it by, and ships with the package.
    assert_eq!(library.load("reach/prompts/ask.md", None).as_deref(), Some("Look at ${item}.\n"));
    assert_eq!(
        library.load("reach/prompts/seat/among.md", None).as_deref(),
        Some("You are one of ${among}.\n")
    );
    assert!(library.ships("reach/prompts/ask.md", None));
    assert_eq!(library.names(None), ["reach/ask"], "what is linked is nothing anybody applies");
    // A name that is no file of `prompts/`, or leads out of it, is refused with the package.
    for name in ["ask.txt", "../ask.md", "/ask.md"] {
        let wrong = Extension::load(&manifest.replace("seat/among.md", name)).err().unwrap_or_default();
        assert!(
            wrong.iter().any(|said| said.contains("must be the name of a file that ends `.md`")),
            "{wrong:?}"
        );
    }
    // A file a blueprint links that the package does not bring, or whose blank the blueprint hands
    // nothing for, is refused when the package is added, as a blueprint that does not check is.
    let refused = |manifest: String, said: &str| {
        let wrong = Extension::load(&manifest).err().unwrap_or_default();
        assert!(wrong.iter().any(|one| one.contains(said)), "{wrong:?}");
    };
    refused(
        manifest.replace("  ask.md: |", "  other.md: |"),
        "`blueprints.ask` links `ask.md` (its step `words`), which the package does not bring",
    );
    refused(
        manifest.replace("Look at ${item}.", "Look at ${item} in ${place}."),
        "`blueprints.ask` links `ask.md` (its step `words`): line 1: its blank `${place}` is handed nothing",
    );
    // A manifest that says nothing of them is as it was.
    let none = Extension::load(
        "extension: plain\nblueprints:\n  up: |\n    blueprint: plain/up\n    steps:\n      go: { do: noop }\n",
    );
    assert!(none.is_ok_and(|plain| plain.prompts.is_empty()));
    // Removed, they are gone with its blueprints.
    rig.agents.remove_extension("reach", &ada()).await.unwrap();
    assert_eq!(rig.agents.library().load("reach/prompts/ask.md", None), None);
}

#[test]
fn the_action_written_out_in_the_documentation_is_one_that_checks() {
    let page = include_str!("../../../../docs/extensions.md");
    let section = page.split("## Actions a package brings").nth(1).expect("the page has the section");
    let written = section.split("```yaml\n").nth(1).and_then(|rest| rest.split("```").next()).unwrap();
    let extension = Extension::load(&format!("extension: example\n{written}"))
        .unwrap_or_else(|problems| panic!("the documented example does not check: {problems:#?}"));
    let serve = &extension.actions["serve"].spec;
    assert_eq!(
        (serve.name.as_str(), serve.capability.as_deref()),
        ("example.serve", Some("extensions:example"))
    );
    assert!(serve.idempotent && serve.args.iter().any(|arg| arg.name == "port" && arg.required));
    // Every field the page lists is one the reader knows, and the other way about.
    assert_eq!(serve.makes.as_ref().map(|made| made.named.as_str()), Some("port"));
    for field in
        ["description", "makes", "with", "first", "refuse", "args", "answer", "fails", "out", "repeatable"]
    {
        assert!(section.contains(&format!("| `{field}` |")), "the page does not say what `{field}` is");
    }
}
