//! Manifests: the packages kept in this repository, each held to its own
//! examples (what the program printed, and what that should come to, and
//! what it starts its program with for a turn held each way it can be),
//! and a manifest written here that names no real program.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use extensions::extension::Reading;
use extensions::{Extension, MANIFEST, Turn};
use kernel::{Cancel, Principal, PrincipalKind};
use serde_json::{Value, json};

/// Whoever asks, with everything.
fn ada() -> Principal {
    Principal {
        kind: PrincipalKind::User,
        id: "ada".into(),
        name: "Ada".into(),
        scopes: vec!["*".into()],
        key: None,
    }
}

/// The packages kept beside the application.
fn packages() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../extensions");
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(MANIFEST).is_file())
        .collect();
    found.sort();
    found
}

#[test]
fn every_package_here_checks_and_reads_its_own_examples_as_it_says_it_should() {
    let packages = packages();
    assert!(!packages.is_empty(), "there are packages to hold to their word");
    for package in packages {
        let name = package.file_name().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(package.join(MANIFEST)).unwrap();
        let extension = Extension::load(&source).unwrap_or_else(|problems| panic!("{name}: {problems:#?}"));
        assert_eq!(extension.name, name, "a package is called what its folder is called");
        let examples: Vec<PathBuf> = std::fs::read_dir(package.join("examples"))
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        let ending = |end: &str| examples.iter().filter(|path| path.to_string_lossy().ends_with(end)).count();

        // What its blueprints do is said in cases, which need a daemon to
        // play them: the command line's tests do that. Here, only that
        // there are some, and the stand-in for its program they are played
        // against.
        if !extension.blueprints.is_empty() {
            assert!(
                ending(".case.json") > 0 && ending("stand-in.sh") == 1,
                "{name} has no cases, so nothing says its blueprints do what they say"
            );
        }
        if !extension.agent {
            assert_eq!(ending(".jsonl"), 0, "{name} brings no agent to read the output of");
            continue;
        }

        let mut read = 0;
        for path in examples {
            if path.extension().is_none_or(|extension| extension != "jsonl") {
                continue;
            }
            let expected: Value =
                serde_json::from_str(&std::fs::read_to_string(path.with_extension("expect.json")).unwrap())
                    .unwrap();
            let mut shown = Vec::new();
            let mut reading = Reading::new(&extension);
            for line in std::fs::read_to_string(&path).unwrap().lines() {
                reading.line(line, &mut |text| shown.push(text.to_string()));
            }
            let mut got = json!({
                "reply": reading.reply.text,
                "session": reading.reply.session,
                "model": reading.reply.model,
                "shown": shown,
                "cost_usd": reading.reply.cost_usd,
                "input_tokens": reading.reply.input_tokens,
                "output_tokens": reading.reply.output_tokens,
                "failure": reading.failure,
            });
            // What it was refused, where it says it was.
            if !reading.reply.refused.is_empty() {
                got["refused"] = json!(reading.reply.refused);
            }
            assert_eq!(got, expected, "{}", path.display());
            read += 1;
        }
        assert!(read > 0, "{name} has no examples, so nothing says it reads its program right");

        // And what it starts its program with: for a turn held each way its
        // program can be, the arguments, in order. Written with what a
        // turn is told standing in for itself (`<system>`, `<prompt>`,
        // and the factory's own command, `<cli>` and `<cli-path>`).
        let starts = package.join("examples/starts.json");
        let cases: Vec<Value> =
            serde_json::from_str(&std::fs::read_to_string(&starts).unwrap_or_else(|_| {
                panic!(
                    "{name} has no `examples/starts.json`, so nothing says what it starts its program with"
                )
            }))
            .unwrap();
        let filled = |word: &str| {
            word.replace("<system>", SYSTEM)
                .replace("<prompt>", PROMPT)
                .replace("<cli-path>", CLI_PATH)
                .replace("<cli>", brand::CLI)
        };
        let mut held = std::collections::BTreeSet::new();
        for case in &cases {
            let said = |key: &str| case[key].as_str();
            let permission =
                said("permission").unwrap_or_else(|| panic!("{name}: a case says no permission"));
            let cli = (case["cli"] != json!(false)).then(|| Path::new(CLI_PATH));
            let mut turn = asked(permission, said("model"), said("session"), cli);
            turn.web = case["web"] == json!(true);
            let expected: Vec<String> =
                case["args"].as_array().unwrap().iter().map(|arg| filled(arg.as_str().unwrap())).collect();
            assert_eq!(extension.started_with(&turn), expected, "{name}, held to `{permission}`: {case}");
            held.insert(permission.to_string());
        }
        for permission in extension.permissions() {
            assert!(held.contains(&permission), "{name} has no case of a turn held to `{permission}`");
        }
    }
}

const SYSTEM: &str = "You are builder.";
const PROMPT: &str = "Build the cart.";
/// Where a made-up factory keeps its own command for an agent's program.
const CLI_PATH: &str = "/factory/bin/its-command";

/// A turn as a slot asks for one, with nothing around it.
fn asked(permission: &str, model: Option<&str>, session: Option<&str>, cli: Option<&Path>) -> Turn {
    Turn {
        slot: "builder".into(),
        principal: ada(),
        system: SYSTEM.into(),
        prompt: PROMPT.into(),
        session: session.map(str::to_string),
        model: model.map(str::to_string),
        workdir: PathBuf::from("/work"),
        env: BTreeMap::new(),
        without: Vec::new(),
        hosting: None,
        behind: None,
        permission_mode: permission.into(),
        cli: cli.map(Path::to_path_buf),
        web: false,
        cancel: Cancel::new(),
        progress: Arc::new(|_: &str| {}),
        kept: None,
    }
}

#[test]
fn a_manifest_with_mistakes_is_refused_with_all_of_them_and_where_each_is() {
    let wrong = r#"
extension: My Agent
agent:
  programme: thing
  args: nope
  output:
    - when: ${ event.kind == }
      sesion: ${ event.id }
  models:
    ask: { args: ["models"] }
"#;
    let Err(problems) = Extension::load(wrong) else { panic!("this must not load") };
    let all = problems.join("\n");
    for expected in [
        "`extension` must be a short name",
        "`agent.programme` is not a known field",
        "`agent.program` is needed",
        "`agent.args` must be a list",
        "`agent.output[0].when` does not read as a condition",
        "`agent.output[0].sesion` is not a known field",
        "`agent.output` needs at least one rule with `reply`",
        "`agent.models.ask` needs `answer`, `list` and `id`",
    ] {
        assert!(all.contains(expected), "missing: {expected}\n{all}");
    }
    assert!(Extension::load("extension: [").is_err(), "and one that is not even the right shape says so");
}

#[test]
fn a_package_brings_an_agent_or_blueprints_or_both_and_names_its_program_once() {
    let agent = "extension: both\nagent:\n  program: thing\n  output:\n    - reply: ${ event.text }\n";
    let blueprints =
        "blueprints:\n  setup: |\n    blueprint: both/setup\n    steps:\n      go: { do: noop }\n";
    let load = |source: String| Extension::load(&source).unwrap_or_else(|problems| panic!("{problems:#?}"));
    let brought = |extension: &Extension| extension.blueprints.keys().cloned().collect::<Vec<_>>();

    let only_agent = load(agent.to_string());
    assert!(only_agent.agent && only_agent.blueprints.is_empty());

    let both = load(format!("{agent}{blueprints}"));
    assert!(both.agent);
    assert_eq!(brought(&both), ["both/setup"], "under the names they are looked up by");
    assert_eq!(both.blueprints["both/setup"], "blueprint: both/setup\nsteps:\n  go: { do: noop }\n");
    assert_eq!(both.program, "thing", "the agent's program is the package's");

    // Blueprints alone: the program they run is named at the top, for
    // whoever asks whether it is here, and is nobody's agent.
    let only_blueprints = load(format!("extension: both\nprogram: no-such-program-here\n{blueprints}"));
    assert!(!only_blueprints.agent && !only_blueprints.available());
    assert_eq!(
        (only_blueprints.program.as_str(), only_blueprints.models_kind()),
        ("no-such-program-here", "none")
    );
    let no_program = load(format!("extension: both\n{blueprints}"));
    assert!(no_program.available(), "one that names no program has none to be missing");

    let Err(problems) = Extension::load(&format!("program: other\n{agent}{blueprints}")) else {
        panic!("a program said twice must not load")
    };
    assert_eq!(problems, ["`program` is said under `agent` when the package brings one; say it there only"]);
}
