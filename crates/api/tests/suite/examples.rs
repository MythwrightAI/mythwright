//! The examples that ship do what they say.
//!
//! An example is something a person copies. One that nothing runs goes
//! wrong without anyone hearing of it: the two-agents example added its
//! cards where no lane took them, and said nothing. So each example is run
//! by a test or a check somewhere, and one test here says so of them all.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::Rig;
use crate::held::{key, said};

/// A build whose tests fail the first time they are run.
#[cfg(unix)]
const FLAKY: &str = include_str!("../../../../examples/scenarios/flaky-build.bp");

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every text under `dir` and the folders in it, read whole.
fn read_all(dir: &Path, into: &mut String) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            read_all(&path, into);
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            into.push_str(&text);
        }
    }
}

/// Unix's alone: the example's own steps are scripts of several lines for
/// a Unix shell (`examples/scenarios/flaky-build.bp`), and the Windows
/// shell is started with one line.
#[cfg(unix)]
#[tokio::test]
async fn a_build_whose_tests_fail_once_is_tried_again_as_one_and_what_follows_waits_for_it() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let applied = rig.send("/api/apply", json!({ "project": "shop", "source": FLAKY, "wait": true })).await;
    let finished = &applied["finished"];
    assert_eq!(finished["status"], "ok", "{applied}");
    let step = |name: &str| {
        finished["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| step["step"] == name)
            .unwrap_or_else(|| panic!("no step `{name}`: {finished}"))
            .clone()
    };
    // The whole of the build was tried twice, and passed the second time.
    assert_eq!(step("build")["attempt"], 2, "{finished}");
    assert_eq!(step("build.test")["output"]["stdout"], "the tests pass");
    // What follows it ran once, after that, with what the build came to.
    assert_eq!(step("ship")["attempt"], 1);
    assert_eq!(step("ship")["output"]["stdout"], "shipped: the tests pass");
    // And what the tests said the first time is still there to read.
    let run = finished["id"].as_str().unwrap();
    let log = rig.read(&format!("/api/runs/{run}/steps/build.test/log")).await;
    assert!(log.to_string().contains("the tests fail the first time"), "{log}");
}

/// A shop with a library and one agent to ask, run by a stand-in that
/// answers as it is told to: briefly, and sure of it; or, told to say
/// first what it would check, at more length and unsure. And the research
/// example among the project's own blueprints.
async fn shop_that_researches() -> Rig {
    let rig = Rig::with("api-research", |config, _| {
        let unsure = "I would need to check what the till keeps. Sales by day (unsure). Next: the till.";
        let asked = ["read", "where", "what you would need to check", "("];
        let checks = ["say", unsure, "say", "DECISION: unsure", ")"];
        let answers = ["say", "Sales by day.", "say", "DECISION: answered"];
        let scholar = crate::standin::argv(&[&asked[..], &checks[..], &answers[..]].concat());
        config.adapters.push(("scholar".into(), scholar));
    })
    .await;
    let rig = Rig::owned(rig).await;
    rig.send("/api/projects", json!({ "name": "Shop", "set_up": ["docs"] })).await;
    rig.send("/api/channels", json!({ "name": "desk" })).await;
    // mw example add research --project shop
    let put = rig.send("/api/projects/shop/examples", json!({ "example": "research" })).await;
    assert_eq!(put["written"].as_array().map(Vec::len), Some(4), "{put}");
    rig
}

/// Every file under `dir` and the folders in it, by where it is within
/// `dir`, with its text.
fn files_of(dir: &Path, within: &str, into: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let (path, name) = (entry.path(), entry.file_name().to_string_lossy().into_owned());
        if path.is_dir() {
            files_of(&path, &format!("{within}{name}/"), into);
        } else {
            into.push((format!("{within}{name}"), std::fs::read_to_string(&path).unwrap()));
        }
    }
}

#[test]
fn an_example_built_into_the_program_is_its_folder_of_the_repository_whole() {
    assert!(!api::examples::OFFERED.is_empty());
    for example in api::examples::OFFERED {
        let mut there = Vec::new();
        files_of(&repo().join("examples").join(example.name), "", &mut there);
        there.sort();
        let mut built_in: Vec<(String, String)> =
            example.files.iter().map(|(file, text)| (file.to_string(), text.to_string())).collect();
        built_in.sort();
        assert_eq!(
            built_in, there,
            "`examples/{}` and what the program has of it differ: a file of the folder that is not listed in \
             `api::examples::OFFERED` is not put into a project with the rest",
            example.name
        );
        assert!(!example.blueprints().is_empty(), "{}: an example brings a blueprint", example.name);
    }
}

#[tokio::test]
async fn an_example_is_put_among_a_projects_own_blueprints_once_and_never_over_what_somebody_wrote() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Which there are is said with what each is and the blueprints it brings.
    let listed = rig.read("/api/examples").await;
    let research = listed.as_array().unwrap().iter().find(|one| one["name"] == "research").unwrap();
    assert_eq!(research["blueprints"], json!(["research/answer", "research/ask"]), "{listed}");
    assert!(research["says"].as_str().is_some_and(|says| says.ends_with('.')), "{listed}");

    // One nobody offers is refused saying which there are.
    let (status, none) = rig.post("/api/projects/shop/examples", json!({ "example": "bakery" })).await;
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND, "{none}");
    assert!(said(&none).contains("no example called `bakery`") && said(&none).contains("research"), "{none}");

    // Put there, its files are the project's own, and its blueprints are applied by their names.
    let put = rig.send("/api/projects/shop/examples", json!({ "example": "research" })).await;
    let written = json!([
        "@blueprints/research/answer.bp",
        "@blueprints/research/ask.bp",
        "@blueprints/research/prompts/answer.md",
        "@blueprints/research/prompts/careful.md"
    ]);
    assert_eq!(
        (&put["written"], &put["there"], &put["folder"]),
        (&written, &json!([]), &json!("@blueprints/research"))
    );
    let file = |path: &str| format!("/api/projects/shop/file?path={path}");
    let ask = rig.read(&file("@blueprints/research/ask.bp")).await;
    assert_eq!(ask["content"], include_str!("../../../../examples/research/ask.bp"), "{ask}");
    // It checks in the project: the blueprint it calls and the words that one links are found there.
    let checked =
        rig.send("/api/blueprints/check", json!({ "source": ask["content"], "project": "shop" })).await;
    assert_eq!((&checked["ok"], &checked["name"]), (&json!(true), &json!("research/ask")), "{checked}");

    // Asked again, nothing is written: it is there as it came.
    let again = rig.send("/api/projects/shop/examples", json!({ "example": "research" })).await;
    assert_eq!((&again["written"], &again["there"]), (&json!([]), &written), "{again}");

    // Words somebody changed are theirs. With a file of it gone and another
    // changed, nothing at all is written: not the one that is missing either.
    let words =
        json!({ "path": "@blueprints/research/prompts/careful.md", "content": "Asked: ${question}\n" });
    let token = Some(rig.token.as_str());
    let (status, saved) = rig.call(reqwest::Method::PUT, "/api/projects/shop/file", token, Some(words)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{saved}");
    let gone = rig.call(reqwest::Method::DELETE, &file("@blueprints/research/ask.bp"), token, None).await;
    assert_eq!(gone.0, reqwest::StatusCode::OK, "{}", gone.1);
    let (status, refused) = rig.post("/api/projects/shop/examples", json!({ "example": "research" })).await;
    assert_eq!(status, reqwest::StatusCode::CONFLICT, "{refused}");
    assert!(
        said(&refused).contains("already has `@blueprints/research/prompts/careful.md`")
            && said(&refused).contains("Nothing was written"),
        "{refused}"
    );
    let (status, _) = rig.get(&file("@blueprints/research/ask.bp")).await;
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND, "the file that was gone was not written back");
}

#[tokio::test]
async fn putting_an_example_into_a_project_takes_what_writing_its_files_takes() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A key that may read the project and apply blueprints, and may not write its files.
    let narrow = key(&rig, &["projects:read", "blueprints", "board", "channels"]).await;
    let asked = Some(json!({ "example": "research" }));
    let (status, refused) =
        rig.call(reqwest::Method::POST, "/api/projects/shop/examples", Some(&narrow), asked).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN, "{refused}");
    assert!(said(&refused).contains("projects:write"), "{refused}");
    let (status, _) = rig.get("/api/projects/shop/file?path=@blueprints/research/ask.bp").await;
    assert_eq!(status, reqwest::StatusCode::NOT_FOUND, "nothing was written for it");
}

/// Asks these questions of the shop, and gives what the run came to.
async fn ask(rig: &Rig, questions: &[&str]) -> serde_json::Value {
    let params = json!({ "project": "shop", "questions": questions, "channel": "desk" });
    let asked = json!({ "blueprint": "research/ask", "project": "shop", "params": params, "wait": true });
    rig.send("/api/apply", asked).await
}

/// What was said in the shop's channel, each line.
async fn said_at_the_desk(rig: &Rig) -> Vec<String> {
    let messages = rig.read("/api/channels/desk/messages").await;
    let all = messages["messages"].as_array().unwrap().iter();
    all.map(|message| message["body"].as_str().unwrap_or_default().to_string()).collect()
}

#[tokio::test]
async fn a_question_is_put_to_an_agent_in_the_words_of_a_file_and_its_answer_is_a_document() {
    let rig = shop_that_researches().await;
    rig.send("/api/agents", json!({ "name": "scholar", "adapter": "scholar", "project": "shop" })).await;
    let question = "What should a small shop measure first?";
    let applied = ask(&rig, &[question]).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // It was asked in the words of the file, with the question in them.
    let asked = crate::fleets::given(&rig, "scholar").await;
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(asked[0].contains(question) && asked[0].contains("five lines or fewer"), "{asked:?}");
    // What it answered is a document of the library, called after the question.
    let kept = rig.read("/api/records/shop-docs:what-should-a-small-shop-measure-first").await;
    assert_eq!(kept["data"]["title"], question, "{kept}");
    assert_eq!(kept["data"]["body"], "Sales by day.", "{kept}");
    // The agent said it where the work is reported, and where it was kept is said after it.
    let said = said_at_the_desk(&rig).await;
    assert!(said.iter().any(|line| line == "Sales by day."), "{said:?}");
    assert!(
        said.iter().any(|line| line.ends_with("shop-docs:what-should-a-small-shop-measure-first.")),
        "{said:?}"
    );
    // And what came of asking is said, for whatever counts it.
    let came = rig.event("research.answered", |_| true).await;
    assert_eq!((&came.data["question"], &came.data["came"]), (&json!(question), &json!("answered")));
}

#[tokio::test]
async fn with_nobody_who_can_answer_nothing_is_kept_and_why_is_said() {
    let rig = shop_that_researches().await;
    // Nobody at all: nothing is answered, and nothing fails.
    let applied = ask(&rig, &["Where do the boxes go?"]).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let said = said_at_the_desk(&rig).await;
    assert_eq!(said, ["Nothing was answered to \"Where do the boxes go?\": there is no agent to ask"]);
    // An agent on the assistant that is built in, which needs no model:
    // it cannot do this, and what it says of that is what is said.
    rig.send("/api/agents", json!({ "name": "helper", "project": "shop" })).await;
    let applied = ask(&rig, &["Who counts the till?"]).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let said = said_at_the_desk(&rig).await;
    let last = said.last().unwrap();
    assert!(
        last.starts_with("Nothing was answered to \"Who counts the till?\": ")
            && last.contains("the built-in assistant cannot do this work")
            && last.contains("help extensions"),
        "{said:?}"
    );
    let came = rig.event("research.answered", |event| event.data["question"] == "Who counts the till?").await;
    assert_eq!(came.data["came"], "unanswered");
    let records = rig.read("/api/collections/shop/docs/records").await;
    let titles: Vec<&str> = records["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|one| one["data"]["title"].as_str())
        .collect();
    assert!(!titles.iter().any(|title| title.contains("till") || title.contains("boxes")), "{titles:?}");
}

#[tokio::test]
async fn two_ways_of_asking_are_compared_with_each_question_going_one_of_them() {
    let rig = shop_that_researches().await;
    rig.send("/api/agents", json!({ "name": "scholar", "adapter": "scholar", "project": "shop" })).await;
    // mw compare start prompts --project shop --of research/answer --per question \
    //     --count question=research.answered:question:came \
    //     --way brief --way careful --prompt careful.research/prompts/answer.md=research/prompts/careful.md
    let asked = json!({
        "name": "prompts", "project": "shop", "of": "research/answer", "per": "question",
        "counts": [{ "name": "question", "topic": "research.answered", "thing": "question", "by": ["came"] }],
        "ways": [
            { "name": "brief" },
            { "name": "careful", "prompts": { "research/prompts/answer.md": "research/prompts/careful.md" } },
        ],
    });
    rig.send("/api/comparisons", asked).await;
    let questions = [
        "What should a small shop measure first?",
        "When is a queue too long?",
        "Which shelf sells most?",
        "How many tills does a shop need?",
    ];
    let applied = ask(&rig, &questions).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // Two were asked in the words that are linked and two in the others, and each was kept.
    let told = crate::fleets::given(&rig, "scholar").await;
    let careful = told.iter().filter(|prompt| prompt.contains("what you would need to check")).count();
    let brief = told.iter().filter(|prompt| prompt.contains("five lines or fewer")).count();
    assert_eq!((brief, careful), (2, 2), "{told:?}");
    let records = rig.read("/api/collections/shop/docs/records").await;
    let kept = records["records"].as_array().unwrap().iter();
    assert_eq!(kept.filter(|one| one["data"]["tags"] == json!(["research"])).count(), 4, "{records}");

    // Laid beside each other: each way with the words it read, how many
    // questions it was given, what came of them and what its agent chose.
    let laid = crate::soon("what came of each question to be counted", async {
        loop {
            let laid = rig.read("/api/comparisons/prompts").await["beside"].clone();
            let rows = laid["rows"].as_array().unwrap();
            let counted = rows.iter().filter(|row| row["kind"] == "came").count();
            if counted == 2 {
                return laid;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    let columns = laid["columns"].as_array().unwrap();
    assert_eq!(columns.len(), 2, "{laid}");
    assert_eq!((&columns[0]["way"], &columns[0]["reads"]), (&json!("brief"), &json!([])), "{laid}");
    let reads = &columns[1]["reads"][0];
    assert_eq!(
        (&columns[1]["way"], &reads["for"], &reads["name"]),
        (&json!("careful"), &json!("research/prompts/answer.md"), &json!("research/prompts/careful.md")),
        "{laid}"
    );
    assert_eq!((&columns[0]["given"], &columns[1]["given"]), (&json!(2), &json!(2)), "{laid}");
    // What each ran is told apart: the blueprint's text and the words it read, together.
    assert_ne!(columns[0]["text"], columns[1]["text"], "{laid}");
    let row = |what: &str| {
        let rows = laid["rows"].as_array().unwrap();
        rows.iter().find(|row| row["what"] == what).unwrap_or_else(|| panic!("no row {what}: {laid}")).clone()
    };
    let count = |what: &str, column: usize| row(what)["by"][column]["count"].as_u64().unwrap_or(0);
    assert_eq!((count("question answered", 0), count("question answered", 1)), (2, 0), "{laid}");
    assert_eq!((count("question unsure", 0), count("question unsure", 1)), (0, 2), "{laid}");
    let chose: Vec<String> = laid["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == "chose")
        .map(|row| row["what"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(chose, ["agent.turn answered", "agent.turn unsure"], "{laid}");
    // Nothing in it says which way is the better one.
    let written = laid.to_string();
    for word in ["better", "best", "winner", "score"] {
        assert!(!written.contains(word), "`{word}` in {written}");
    }
}

#[test]
fn an_example_built_into_the_program_works_nothing_out_for_itself() {
    // An example is not held to what ships by the gate, and one that is
    // put into a project by name is what a new person reads first: it is
    // held to it here.
    for example in api::examples::OFFERED {
        for (path, text) in example.files.iter().filter(|(path, _)| brand::is_blueprint_path(path)) {
            let found = blueprint::plain::unplain(text).unwrap_or_else(|why| panic!("{path}: {why}"));
            let said: Vec<String> = found.iter().map(|one| format!("{}: {}", one.why, one.source)).collect();
            assert!(said.is_empty(), "{}/{path} works something out: {said:?}", example.name);
        }
    }
}

#[test]
fn every_example_that_ships_is_run_by_a_test_or_a_check() {
    let repo = repo();
    // Whatever runs something here: each crate's tests, the client's
    // checks, and the tests of an example that is a program.
    let mut runs = String::new();
    for entry in std::fs::read_dir(repo.join("crates")).unwrap().flatten() {
        read_all(&entry.path().join("tests"), &mut runs);
    }
    read_all(&repo.join("ui/scripts"), &mut runs);
    for entry in std::fs::read_dir(repo.join("examples")).unwrap().flatten() {
        read_all(&entry.path().join("tests"), &mut runs);
    }
    let mut unrun = Vec::new();
    for folder in ["examples/scenarios", "examples/blueprints", "examples/research"] {
        for entry in std::fs::read_dir(repo.join(folder)).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(&format!(".{}", brand::BLUEPRINT_EXT)) && !runs.contains(&name) {
                unrun.push(format!("{folder}/{name}"));
            }
        }
    }
    assert!(
        unrun.is_empty(),
        "nothing runs {unrun:?}. An example that nothing runs goes wrong unnoticed: \
         apply it in a test (as `examples.rs` does) and say there what it is to do"
    );
}
