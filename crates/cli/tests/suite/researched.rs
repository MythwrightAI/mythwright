//! The experiment of the first run, from the terminal, as its commands
//! are written in `docs/first-run.md`: the research example put into a
//! project, each question going one of two ways that differ only in the
//! words the agent is told, and the two set side by side. The texts
//! themselves are read and run as they are written in `written.rs`.

use api::{Config, Node};

use crate::{Rig, soon};

/// A node whose agents run on `on` unless they say otherwise: a stand-in
/// that answers as it is told to (briefly, and sure of it; or, told to
/// say first what it would check, at more length and unsure), or the
/// assistant that is built in, which needs no model.
async fn rig(on: &str) -> Rig {
    let dir = crate::scratch("researched");
    let unsure = "I would need to check what the till keeps. Sales by day (unsure). Next: the till.";
    let asked = ["read", "where", "what you would need to check", "("];
    let checks = ["say", unsure, "say", "DECISION: unsure", ")"];
    let answers = ["say", "Sales by day.", "say", "DECISION: answered"];
    let answers = crate::standin::argv(&[&asked[..], &checks[..], &answers[..]].concat());
    let order = std::sync::Arc::new(journal::Order::default());
    let mut config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        device: Some("test-device".into()),
        default_adapter: Some(on.into()),
        walls: Some(api::Walls::Open),
        stands_between: true,
        disk: journal::Disk::Unwaited,
        order: Some(order.clone()),
        ..Config::new(dir.join("data"))
    };
    config.adapters.push(("scholar".into(), answers));
    let node = Node::start(config).await.unwrap();
    let rig = Rig { url: node.url(), node: Some(node), dir, order };
    rig.run(&["register", "--email", "ada@example.com", "--name", "Ada", "--password", "correct horse"])
        .await
        .ok();
    rig
}

/// The four questions of the first run's experiment, as one param.
const QUESTIONS: &str = "questions=[\"What should a small shop measure first?\",\"When is a queue too long?\",\
     \"Which shelf sells most?\",\"How many tills does a shop need?\"]";

/// A shop as `project create` makes one, with the example put among its
/// blueprints and the comparison of the two prompts started: the first
/// two commands of the experiment, as they are written.
async fn shop_with_the_experiment(rig: &Rig) {
    rig.run(&["project", "create", "Shop"]).await.ok();
    // The example is one of those that came with the program, and is put
    // among the project's own blueprints by name: nothing is copied by hand.
    let offered = rig.run(&["example", "list"]).await.ok();
    assert!(offered.contains("research  Questions put to an agent of a project"), "{offered}");
    assert!(offered.contains("its blueprints: `research/answer` and `research/ask`"), "{offered}");
    let put = rig.run(&["example", "add", "research", "--project", "shop"]).await.ok();
    assert!(
        put.contains("The example `research` is among `shop`'s blueprints, in `@blueprints/research`"),
        "{put}"
    );
    assert!(put.contains("Its blueprints are `research/answer` and `research/ask`"), "{put}");
    for file in ["ask.bp", "answer.bp", "prompts/answer.md", "prompts/careful.md"] {
        assert!(rig.placed("shop").join("research").join(file).is_file(), "{file}");
        assert!(!rig.project("shop").join("blueprints").exists(), "nothing of it in the project's folder");
    }
    // Asked again it says so and writes nothing.
    let again = rig.run(&["example", "add", "research", "--project", "shop"]).await.ok();
    assert!(again.contains("has the example `research` already, as it came"), "{again}");
    let started = rig
        .run(&[
            "compare",
            "start",
            "prompts",
            "--project",
            "shop",
            "--of",
            "research/answer",
            "--per",
            "question",
            "--count",
            "question=research.answered:question:came",
            "--way",
            "brief",
            "--way",
            "careful",
            "--prompt",
            "careful.research/prompts/answer.md=research/prompts/careful.md",
        ])
        .await
        .ok();
    assert!(started.contains("It counts what comes of each thing: question."), "{started}");
}

/// Asks, as the experiment's third command is written.
async fn ask(rig: &Rig, questions: &str) -> String {
    let asked = ["apply", "research/ask", "--project", "shop"];
    let with = ["--param", "project=shop", "--param", "channel=shop", "--param", questions];
    rig.run(&[&asked[..], &with[..]].concat()).await.ok()
}

/// `compare show prompts`, once it has a line that begins with each of these.
async fn shown(rig: &Rig, lines: &[&str]) -> String {
    soon("what came of each question to be counted", async {
        loop {
            let shown = rig.run(&["compare", "show", "prompts"]).await.ok();
            if lines.iter().all(|line| !crate::compared::line(&shown, line).is_empty()) {
                return shown;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn the_first_runs_experiment_sets_two_ways_of_asking_side_by_side_from_the_terminal() {
    let rig = rig("scholar").await;
    shop_with_the_experiment(&rig).await;
    let asked = ask(&rig, QUESTIONS).await;
    assert!(asked.contains(" ok in "), "{asked}");

    let line = crate::compared::line;
    let shown = shown(&rig, &["question answered", "question unsure"]).await;
    // Each way with the words it read, and how many questions it was given.
    let read = line(&shown, "research/prompts/answer.md");
    assert_eq!(
        (read[1], &read[2][..read[2].len() - 9]),
        ("as it is linked", "research/prompts/careful.md"),
        "{shown}"
    );
    assert_eq!(line(&shown, "things given")[1..], ["2", "2"], "{shown}");
    // What came of them, and what the agent chose, under each: counted, and no more said of it.
    assert_eq!(line(&shown, "question answered")[1], "2", "{shown}");
    assert_eq!(line(&shown, "question unsure")[1..3], ["-", "2"], "{shown}");
    assert_eq!(line(&shown, "agent.turn answered")[1], "2", "{shown}");
    assert_eq!(line(&shown, "agent.turn unsure")[1..3], ["-", "2"], "{shown}");
    assert!(shown.trim_end().ends_with("Which way did better is yours to say."), "{shown}");

    // The answers are said in the project's channel by whoever gave them,
    // and each is a document of its library, called after its question.
    let said = rig.run(&["read", "shop"]).await.ok();
    assert!(said.contains("] Sales by day.\n"), "{said}");
    assert!(
        said.contains("The answer to \"When is a queue too long?\" is in the library, as shop-docs:when-is-a-queue-too-long."),
        "{said}"
    );
    let kept = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    assert_eq!(kept.lines().filter(|line| line.contains("tags research")).count(), 4, "{kept}");

    // Ended saying which way goes on: from then every question is asked in those words.
    let ended = rig.run(&["compare", "end", "prompts", "--goes", "careful"]).await.ok();
    assert!(ended.contains("everything goes `careful`"), "{ended}");
    ask(&rig, "questions=[\"Who opens the shop?\",\"Who shuts it?\"]").await;
    let kept = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    let asked_since = |title: &str| {
        let one = kept.lines().find(|line| line.contains(title)).unwrap_or_default();
        one.contains("I would need to check")
    };
    assert!(asked_since("Who opens the shop?") && asked_since("Who shuts it?"), "{kept}");
    // Ended again with no way said: questions are asked as the example is written.
    rig.run(&["compare", "end", "prompts"]).await.ok();
    ask(&rig, "questions=[\"Who sweeps the floor?\"]").await;
    let kept = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    let swept = kept.lines().find(|line| line.contains("Who sweeps the floor?")).unwrap_or_default();
    assert!(swept.contains("body Sales by day."), "{kept}");
}

#[tokio::test(flavor = "multi_thread")]
async fn on_the_assistant_that_is_built_in_the_experiment_runs_and_says_that_nothing_was_answered() {
    let rig = rig("builtin").await;
    shop_with_the_experiment(&rig).await;
    let asked = ask(&rig, QUESTIONS).await;
    assert!(asked.contains(" ok in "), "{asked}");
    // Each way was given its questions and read its own words; none was answered, and that is what is counted.
    let line = crate::compared::line;
    let shown = shown(&rig, &["question unanswered"]).await;
    assert_eq!(line(&shown, "things given")[1..], ["2", "2"], "{shown}");
    let unanswered = line(&shown, "question unanswered");
    assert_eq!((unanswered[1], unanswered[3]), ("2", "2"), "{shown}");
    assert!(line(&shown, "question answered").is_empty(), "{shown}");
    // Why is said where the work is reported, in the assistant's own words, with what would do it.
    let said = rig.run(&["read", "shop"]).await.ok();
    assert!(
        said.contains("Nothing was answered to \"Which shelf sells most?\": ")
            && said.contains("the built-in assistant cannot do this work")
            && said.contains("help extensions"),
        "{said}"
    );
    let kept = rig.run(&["record", "list", "shop", "docs"]).await.ok();
    assert!(!kept.contains("tags research"), "{kept}");
}

#[tokio::test(flavor = "multi_thread")]
async fn other_words_written_into_the_file_are_what_a_comparison_started_again_asks_in() {
    let rig = rig("scholar").await;
    shop_with_the_experiment(&rig).await;
    ask(&rig, QUESTIONS).await;
    rig.run(&["compare", "end", "prompts"]).await.ok();
    // Words of one's own, written into the project's file with an editor. Nothing is applied or rebuilt.
    let file = rig.placed("shop").join("research/prompts/careful.md");
    let words =
        "Asked: ${question}\n\nSay at length what you would need to check. Choose `answered` or `unsure`.\n";
    std::fs::write(&file, words).unwrap();
    // A comparison's name is its own for good: another is called something else.
    let again = started(&rig, "prompts").await;
    assert!(again.err.contains("call this one something else"), "{}", again.err);
    started(&rig, "words").await.ok();
    ask(&rig, "questions=[\"Who opens the shop?\",\"Who shuts it?\"]").await;
    let turns = rig.node.as_ref().unwrap().app.agents.turns("shop-architect-1", 2).await.unwrap();
    let asked: Vec<&str> = turns.iter().map(|turn| turn.prompt.as_str()).collect();
    assert_eq!(asked.iter().filter(|prompt| prompt.starts_with("Asked: Who ")).count(), 1, "{asked:?}");
    assert_eq!(asked.iter().filter(|prompt| prompt.contains("five lines or fewer")).count(), 1, "{asked:?}");
}

/// A comparison of the two prompts started under `name`, as the experiment starts its own.
async fn started(rig: &Rig, name: &str) -> crate::Ran {
    let start = ["compare", "start", name, "--project", "shop", "--of", "research/answer"];
    let ways = ["--per", "question", "--count", "question=research.answered:question:came", "--way", "brief"];
    let other =
        ["--way", "careful", "--prompt", "careful.research/prompts/answer.md=research/prompts/careful.md"];
    rig.run(&[&start[..], &ways[..], &other[..]].concat()).await
}
