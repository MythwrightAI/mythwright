//! Extension packages: agents the product does not know, run from what a
//! package says about them.
//!
//! The machinery that runs any package, tested against a stand-in
//! program, with a manifest written here that names no real one. The
//! packages kept in this repository are each held to their own examples,
//! and a manifest to being read and refused, by the suite of `extensions`
//! (its `manifests.rs`), where what a package says is.
//!
//! Unix's alone: the stand-in for a package's program is a file of a Unix
//! shell, made a program by its mode.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use agents::adapter::Command;
use agents::extension::Packaged;
use agents::{Adapter, Extension, Turn};
use kernel::Cancel;

use crate::{ada, alive, soon};

// ---- the machinery, against a stand-in ---------------------------------------------

struct Stub {
    dir: PathBuf,
    extension: Arc<Extension>,
    said: Arc<Mutex<Vec<String>>>,
}

impl Drop for Stub {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A package for a made-up agent, and the program it runs: a script that
/// records how it was started and prints what such a program would.
fn stub() -> Stub {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("standin-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let here = dir.display();
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  read asked
  printf '%s' "$asked" > "{here}/asked"
  printf '{{"note":"not the answer"}}\n'
  printf '{{"answer":1,"models":[{{"key":"quick","name":"Quick","about":"For small things","usual":true}},{{"key":"deep","name":"Deep","about":"For hard things"}},{{"key":"old","name":"Old","retired":true}}]}}\n'
  sleep 300
  exit 0
fi
printf '%s\n' "$@" > "{here}/args"
prompt=$(cat)
printf '%s' "$prompt" > "{here}/prompt"
case "$prompt" in
  *crash*) echo "warming up" >&2; echo "boom: out of credits" >&2; exit 3 ;;
  *refuse*) printf '{{"kind":"end","bad":true,"text":"I cannot do that","talk":"s-1"}}\n'; exit 0 ;;
  *hang*) sleep 300 & echo $! > "{here}/child"; echo $$ >> "{here}/child"; wait ;;
esac
printf '{{"kind":"start","talk":"s-1"}}\n'
printf 'this line is not json and must be ignored\n'
printf '{{"kind":"step","parts":[{{"say":"Working on it."}},{{"tool":"Shell"}},{{"neither":true}}]}}\n'
printf '{{"kind":"end","bad":false,"text":"All done.","talk":"s-1","cost":0.0125,"used":{{"in":10,"cached":95,"out":7}}}}\n'
"#
    );
    let program = dir.join("standin");
    crate::program(&program, &script);
    let manifest = format!(
        r#"
extension: standin
title: Stand-in
description: Not a real agent.
agent:
  program: {program}
  args:
    - ["run", "--quiet"]
    - ["--about", "${{ turn.system }}"]
    - when: ${{ turn.permission == "read" }}
      args: ["--careful"]
    - when: ${{ turn.permission == "read" && turn.cli_path != null }}
      args: ["--may-run", "${{ turn.cli }}", "at ${{ turn.cli_path }}"]
    - when: ${{ turn.web == true }}
      args: ["--may-look-things-up"]
    - when: ${{ turn.permission == "full" }}
      args: ["--anything"]
    - when: ${{ turn.model != null }}
      args: ["--model", "${{ turn.model }}"]
    - when: ${{ turn.session != null }}
      args: ["--carry-on", "${{ turn.session }}"]
  input: ${{ turn.prompt }}
  output:
    - when: ${{ event.talk != null }}
      session: ${{ event.talk }}
    - when: ${{ event.kind == "step" }}
      each: ${{ event.parts }}
      show: ${{ item.say }}
    - when: ${{ event.kind == "step" }}
      each: ${{ event.parts }}
      show: '${{ item.tool != null ? "[" + item.tool + "]" : null }}'
    - when: ${{ event.kind == "end" }}
      reply: ${{ event.text }}
      cost_usd: ${{ event.cost }}
      input_tokens: ${{ (event.used.in ?? 0) + (event.used.cached ?? 0) }}
      output_tokens: ${{ event.used.out }}
      failed: ${{ event.bad == true }}
      error: ${{ event.text }}
  models:
    ask:
      args: ["models"]
      send:
        - {{ ask: models, n: 1 }}
      answer: ${{ line.answer == 1 }}
      list: ${{ line.models }}
      id: ${{ model.key }}
      label: ${{ model.name }}
      note: ${{ model.about }}
      hidden: ${{ model.retired == true }}
      default: ${{ model.usual == true }}
"#,
        program = program.display()
    );
    let extension = Extension::load(&manifest).unwrap_or_else(|problems| panic!("{problems:#?}"));
    Stub { dir, extension: Arc::new(extension), said: Arc::default() }
}

impl Stub {
    fn turn(&self, prompt: &str, session: Option<&str>) -> Turn {
        let said = self.said.clone();
        Turn {
            slot: "builder".into(),
            principal: ada(),
            system: "You are builder.".into(),
            prompt: prompt.into(),
            session: session.map(str::to_string),
            model: Some("quick".into()),
            workdir: self.dir.clone(),
            env: BTreeMap::from([("FLEET_URL".to_string(), "http://127.0.0.1:7700".to_string())]),
            without: Vec::new(),
            hosting: None,
            behind: None,
            permission_mode: "edit".into(),
            cli: Some(self.dir.join("bin").join(brand::CLI)),
            web: false,
            cancel: Cancel::new(),
            progress: Arc::new(move |text: &str| said.lock().unwrap().push(text.to_string())),
            kept: None,
        }
    }

    fn read(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.join(name)).unwrap_or_default()
    }

    fn adapter(&self) -> Packaged {
        Packaged(self.extension.clone())
    }
}

#[tokio::test]
async fn a_turn_is_started_and_read_the_way_the_package_says() {
    let stub = stub();
    let reply = stub.adapter().run(stub.turn("Build the cart.\nWith tests.", None)).await.unwrap();

    assert_eq!(reply.text, "All done.");
    assert_eq!(reply.session.as_deref(), Some("s-1"));
    assert_eq!(reply.cost_usd, Some(0.0125));
    assert_eq!((reply.input_tokens, reply.output_tokens), (105, 7), "the package says what counts as input");
    assert_eq!(
        *stub.said.lock().unwrap(),
        ["Working on it.", "[Shell]"],
        "what it says and what it reaches for are relayed as they happen, and nothing else is"
    );

    assert_eq!(
        stub.read("prompt"),
        "Build the cart.\nWith tests.",
        "what it is asked goes in on standard input, whole"
    );
    let args: Vec<String> = stub.read("args").lines().map(str::to_string).collect();
    assert_eq!(args, ["run", "--quiet", "--about", "You are builder.", "--model", "quick"]);
    assert!(!args.iter().any(|arg| arg.contains("Build the cart")), "and is never on the command line");
}

#[tokio::test]
async fn how_much_an_agent_may_do_is_put_to_its_program_in_the_programs_own_terms() {
    let stub = stub();
    for (mode, flag) in [("read", Some("--careful")), ("edit", None), ("full", Some("--anything"))] {
        let mut turn = stub.turn("hello", None);
        turn.permission_mode = mode.into();
        stub.adapter().run(turn).await.unwrap();
        let args = stub.read("args");
        let passed =
            ["--careful", "--anything"].into_iter().find(|flag| args.lines().any(|arg| arg == *flag));
        assert_eq!(passed, flag, "{mode}: {args}");
    }
}

#[tokio::test]
async fn a_package_is_told_what_the_factorys_own_command_is_called_and_where_it_is_for_its_program() {
    let stub = stub();
    let mut turn = stub.turn("hello", None);
    turn.permission_mode = "read".into();
    stub.adapter().run(turn).await.unwrap();
    let args: Vec<String> = stub.read("args").lines().map(str::to_string).collect();
    let at = args.iter().position(|arg| arg == "--may-run").expect("the package's own flag for it");
    assert_eq!(args[at + 1], brand::CLI, "by its name, which the package never writes");
    assert_eq!(args[at + 2], format!("at {}", stub.dir.join("bin").join(brand::CLI).display()));

    // An adapter run by itself has no factory around it, and is told of none.
    let mut alone = stub.turn("hello", None);
    (alone.permission_mode, alone.cli) = ("read".into(), None);
    stub.adapter().run(alone).await.unwrap();
    assert!(!stub.read("args").lines().any(|arg| arg == "--may-run"), "{}", stub.read("args"));
}

#[tokio::test]
async fn a_package_is_told_whether_its_agent_may_use_the_web_unasked_and_says_what_that_is_to_its_program() {
    let stub = stub();
    let looks = |args: String| args.lines().any(|arg| arg == "--may-look-things-up");
    stub.adapter().run(stub.turn("hello", None)).await.unwrap();
    assert!(!looks(stub.read("args")), "nothing was said of it, and its program is started as it was");
    let mut turn = stub.turn("hello", None);
    turn.web = true;
    stub.adapter().run(turn).await.unwrap();
    assert!(looks(stub.read("args")), "{}", stub.read("args"));
}

#[tokio::test]
async fn a_continued_conversation_carries_on_the_programs_own_session() {
    let stub = stub();
    stub.adapter().run(stub.turn("And payments?", Some("s-1"))).await.unwrap();
    let args = stub.read("args");
    assert!(args.ends_with("--carry-on\ns-1\n"), "{args}");
}

#[tokio::test]
async fn an_error_the_agent_reports_is_the_turns_error() {
    let stub = stub();
    let err = stub.adapter().run(stub.turn("please refuse", None)).await.unwrap_err().message;
    assert_eq!(err, "I cannot do that");
}

#[tokio::test]
async fn an_agent_that_dies_without_a_result_is_explained_by_what_it_printed() {
    let stub = stub();
    let err = stub.adapter().run(stub.turn("please crash", None)).await.unwrap_err().message;
    assert!(err.contains("ended (exit status: 3) without a result"), "{err}");
    assert!(err.ends_with("warming up / boom: out of credits"), "{err}");

    let elsewhere = stub.extension.source.replace("/standin\n", "/not-installed\n");
    let missing = Arc::new(Extension::load(&elsewhere).unwrap());
    assert!(
        stub.extension.available() && !missing.available(),
        "whether the program is here is known up front"
    );
    let err = Packaged(missing).run(stub.turn("hello", None)).await.unwrap_err().message;
    assert!(err.starts_with("could not start"), "{err}");
}

#[tokio::test]
async fn cancelling_a_turn_takes_everything_it_started_with_it() {
    let stub = stub();
    let turn = stub.turn("please hang", None);
    let cancel = turn.cancel.clone();
    let running = tokio::spawn(stub.adapter().run(turn));
    let child_file = stub.dir.join("child");
    soon("the agent to start its child", async {
        while std::fs::read_to_string(&child_file).map_or(0, |text| text.lines().count()) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await;
    let pids: Vec<String> = stub.read("child").lines().map(str::to_string).collect();
    assert!(pids.iter().all(|pid| alive(pid)));

    cancel.cancel();
    assert_eq!(running.await.unwrap().unwrap_err().message, "the turn was cancelled");
    soon("its processes to die", async {
        while pids.iter().any(|pid| alive(pid)) {
            tokio::task::yield_now().await;
        }
    })
    .await;
}

#[tokio::test]
async fn a_program_is_asked_which_models_it_offers_and_is_not_left_running() {
    let stub = stub();
    let models = soon("the program to answer", stub.extension.ask_models(None)).await.unwrap();
    let seen: Vec<(&str, &str, &str, bool)> =
        models.iter().map(|m| (m.id.as_str(), m.label.as_str(), m.note.as_str(), m.default)).collect();
    assert_eq!(
        seen,
        [("quick", "Quick", "For small things", true), ("deep", "Deep", "For hard things", false)],
        "what it is selected by, what a person is shown, and which is its usual; a retired one is left out"
    );
    assert_eq!(stub.read("asked"), r#"{"ask":"models","n":1}"#, "it was asked the way the package says");
}

#[tokio::test]
async fn a_program_that_prints_its_models_as_one_value_over_many_lines_and_under_their_names_is_read_too() {
    let dir =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("lister-{}", &kernel::id::secret()[..12]));
    std::fs::create_dir_all(&dir).unwrap();
    let program = dir.join("lister");
    let script = r#"#!/bin/sh
if [ "$1" = "which" ]; then
  echo "2 models"
  echo "Usual one: house/deep"
  exit 0
fi
cat <<'END'
{
  "models": {
    "house/quick": { "shown": "Quick" },
    "house/deep": {}
  }
}
END
"#;
    crate::program(&program, script);
    let manifest = format!(
        r#"
extension: lister
agent:
  program: {program}
  input: ${{ turn.prompt }}
  output:
    - reply: ${{ event.text }}
  models:
    ask:
      args: ["list"]
      answer: ${{ line.models != null }}
      list: ${{ line.models }}
      id: ${{ model.key }}
      label: ${{ model.value.shown ?? model.key }}
"#,
        program = program.display()
    );
    let extension = Extension::load(&manifest).unwrap_or_else(|problems| panic!("{problems:#?}"));
    let models = extension.ask_models(None).await.unwrap();
    let listed: Vec<(&str, &str)> = models.iter().map(|m| (m.id.as_str(), m.label.as_str())).collect();
    assert_eq!(listed, [("house/deep", "house/deep"), ("house/quick", "Quick")]);
    assert!(models.iter().all(|model| !model.default), "it did not say which is its usual one");

    // Which one it uses when none is named, it says elsewhere, in plain
    // words. A package that knows where asks it that as well.
    let asking = format!(
        "{manifest}      usual:\n        args: [\"which\"]\n        when: ${{ starts_with(line, \"Usual one: \") }}\n        id: ${{ replace(line, \"Usual one: \", \"\") }}\n"
    );
    let extension = Extension::load(&asking).unwrap_or_else(|problems| panic!("{problems:#?}"));
    let models = extension.ask_models(None).await.unwrap();
    let usual: Vec<&str> = models.iter().filter(|m| m.default).map(|m| m.id.as_str()).collect();
    assert_eq!(usual, ["house/deep"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn any_program_that_reads_a_prompt_and_prints_a_reply_can_be_an_agent() {
    let stub = stub();
    let adapter =
        Command { argv: vec!["sh".into(), "-c".into(), "echo \"model={model}\"; tr a-z A-Z".into()] };
    let reply = adapter.run(stub.turn("hello there", None)).await.unwrap();
    assert_eq!(reply.text, "model=quick\nYOU ARE BUILDER.\n\nHELLO THERE");

    let failing = Command { argv: vec!["sh".into(), "-c".into(), "echo nope >&2; exit 2".into()] };
    let err = failing.run(stub.turn("hello", None)).await.unwrap_err().message;
    assert!(err.contains("failed") && err.ends_with("nope"), "{err}");
}
