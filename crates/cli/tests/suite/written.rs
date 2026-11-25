//! The experiment of the first run, run as its two texts write it: each
//! command read out of `docs/first-run.md` and the README and run as it
//! stands there, by a copy of the program in a folder of its own.
//!
//! A command written in a text and run nowhere goes stale without anyone
//! hearing; one a test writes out again by hand is held only to what the
//! test says. And what a new person has is a package: a program with no
//! checkout beside it, which reads nothing of the repository. So the
//! commands are taken from the texts, and what runs them is a copy.
//!
//! Unix's alone: the copy of the program is started and stopped as a
//! process, by a signal, and its stand-in agents are lines of a Unix
//! shell.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::compared::line;
use crate::places::{Started, copied};
use crate::soon;

/// A line of a block of commands as its words, as a shell takes them
/// apart where these texts are concerned: words stand apart by spaces,
/// what is in quotes of either kind is one word with the quotes off, and
/// `#` where a word would begin is a remark to the end of the line.
fn words(line: &str) -> Vec<String> {
    let (mut words, mut word, mut quote) = (Vec::new(), None::<String>, None::<char>);
    for letter in line.chars() {
        match (quote, letter) {
            (Some(open), shut) if shut == open => quote = None,
            (Some(_), inside) => word.get_or_insert_with(String::new).push(inside),
            (None, '\'' | '"') => {
                quote = Some(letter);
                word.get_or_insert_with(String::new);
            }
            (None, '#') if word.is_none() => break,
            (None, space) if space.is_whitespace() => words.extend(word.take()),
            (None, other) => word.get_or_insert_with(String::new).push(other),
        }
    }
    assert!(quote.is_none(), "a quote that is not shut in: {line}");
    words.extend(word);
    words
}

/// The commands of the first block of them that follows `after` in a
/// text, each as what the command line is handed: a line that ends in `\`
/// goes on in the next, and each begins with the program's own name,
/// which is left off.
fn commands(text: &str, after: &str) -> Vec<Vec<String>> {
    let from = text.find(after).unwrap_or_else(|| panic!("the text no longer says `{after}`"));
    let block = text[from..].split("```sh\n").nth(1).expect("a block of commands follows");
    let block = block.split("```").next().unwrap().replace("\\\n", " ");
    let mut found = Vec::new();
    for mut command in block.lines().map(words).filter(|words| !words.is_empty()) {
        assert_eq!(command.remove(0), brand::CLI, "each line of the block is a command of this program's");
        found.push(command);
    }
    found
}

fn text_of(file: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(file)).unwrap()
}

/// Runs each command as it is written, and gives what each said, by its
/// first two words. What sets the two ways side by side is asked until it
/// has a line that begins with each of `counted`: what came of a question
/// is counted a moment after its answer.
async fn run_all(copy: &Started, commands: &[Vec<String>], counted: &[&str]) -> Vec<(String, String)> {
    let mut said = Vec::new();
    for command in commands {
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        let out = match args[..2] == ["compare", "show"] {
            false => copy.run(&args).await,
            true => {
                soon("what came of each question to be counted", async {
                    loop {
                        let shown = copy.run(&args).await;
                        if counted.iter().all(|start| !line(&shown, start).is_empty()) {
                            return shown;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
            }
        };
        said.push((args[..2].join(" "), out));
    }
    said
}

/// What a command said, the first time it was run.
fn said<'a>(all: &'a [(String, String)], command: &str) -> &'a str {
    let found = all.iter().find(|(name, _)| name == command);
    &found.unwrap_or_else(|| panic!("the text no longer has `{command}`")).1
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_of_the_program_runs_the_first_runs_experiment_as_its_commands_are_written() {
    let (made, app) = copied("written");
    // Whoever is asked answers as it is told to: briefly, and sure of it;
    // or, told to say first what it would check, at more length and unsure.
    let scholar = made.0.join("scholar.sh");
    let answers = "#!/bin/sh\nasked=$(cat)\ncase \"$asked\" in\n  *'what you would need to check'*)\n    \
                   echo 'I would need to check what the till keeps. Sales by day (unsure). Next: the till.'\n    \
                   echo 'DECISION: unsure';;\n  *) echo 'Sales by day.'; echo 'DECISION: answered';;\nesac\n";
    std::fs::write(&scholar, answers).unwrap();
    std::fs::set_permissions(&scholar, std::fs::Permissions::from_mode(0o755)).unwrap();
    let runs = format!("scholar={}", scholar.display());
    let agents = ["--adapter", "scholar", "--command", runs.as_str(), "--walls", "off"];
    let copy = Started::told(&app.join(crate::places::PROGRAM), &agents).await;
    assert_eq!(copy.runs_from().await, "installed", "it has no checkout to read an example from");
    copy.register().await;
    // "With a project called Shop that has its fleet and its library (step 3)".
    copy.run(&["project", "create", "Shop"]).await;

    let commands = commands(&text_of("docs/first-run.md"), "## 9. An experiment");
    let all = run_all(&copy, &commands, &["question answered", "question unsure"]).await;
    let put = said(&all, "example add");
    assert!(put.contains("is among `shop`'s blueprints"), "{put}");
    assert!(said(&all, "compare start").contains("It counts what comes of each thing: question."));
    assert!(said(&all, "apply research/ask").contains(" ok in "), "{}", said(&all, "apply research/ask"));
    // Side by side: each way with the words it read, the questions it was
    // given, what came of them and what its agent chose.
    let shown = said(&all, "compare show");
    assert_eq!(line(shown, "research/prompts/answer.md")[1], "as it is linked", "{shown}");
    assert_eq!(line(shown, "things given")[1..], ["2", "2"], "{shown}");
    assert_eq!(line(shown, "question answered")[1], "2", "{shown}");
    assert_eq!(line(shown, "question unsure")[1..3], ["-", "2"], "{shown}");
    assert_eq!(line(shown, "agent.turn answered")[1], "2", "{shown}");
    assert_eq!(line(shown, "agent.turn unsure")[1..3], ["-", "2"], "{shown}");
    // The answers, and where each was kept.
    assert!(said(&all, "read shop").contains("] Sales by day.\n"), "{}", said(&all, "read shop"));
    let kept = said(&all, "record list");
    assert_eq!(kept.lines().filter(|line| line.contains("tags research")).count(), 4, "{kept}");
    // Ended saying which way goes on, and then as the example is written again.
    let ended: Vec<&str> =
        all.iter().filter(|(name, _)| name == "compare end").map(|(_, out)| &**out).collect();
    assert_eq!(ended.len(), 2, "{commands:?}");
    assert!(ended[0].contains("everything goes `careful`"), "{}", ended[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_with_only_the_assistant_that_is_built_in_runs_the_readmes_experiment_as_written() {
    let (_made, app) = copied("written-readme");
    let copy = Started::from(&app.join(crate::places::PROGRAM)).await;
    copy.register().await;
    copy.run(&["project", "create", "Shop"]).await;
    let commands = commands(&text_of("README.md"), "Next, try something two ways.");
    let all = run_all(&copy, &commands, &["question unanswered"]).await;
    assert_eq!(all.len(), 5, "the README's experiment is five commands: {commands:?}");
    assert!(said(&all, "example add").contains("is among `shop`'s blueprints"));
    // Each way was given its questions; none was answered, and that is what is counted.
    let shown = said(&all, "compare show");
    assert_eq!(line(shown, "things given")[1..], ["2", "2"], "{shown}");
    let unanswered = line(shown, "question unanswered");
    assert_eq!((unanswered[1], unanswered[3]), ("2", "2"), "{shown}");
    assert!(line(shown, "question answered").is_empty(), "{shown}");
    assert!(said(&all, "compare end").contains("everything goes `careful`"));
}

/// What a command says, asked again until it says what the text says it will.
async fn until(copy: &Started, args: &[&str], wanted: fn(&str) -> bool) -> String {
    soon("what the text says is shown to be there", async {
        loop {
            let shown = copy.run(args).await;
            if wanted(&shown) {
                return shown;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_of_the_program_compares_two_ways_of_working_cards_as_the_first_runs_commands_are_written() {
    let (made, app) = copied("written-cards");
    // Somebody who can do the work, for when the rows are told of it: it says that it did.
    let hand = made.0.join("hand.sh");
    std::fs::write(&hand, "#!/bin/sh\ncat >/dev/null\necho 'Done, and looked over once more.'\n").unwrap();
    std::fs::set_permissions(&hand, std::fs::Permissions::from_mode(0o755)).unwrap();
    let runs = format!("hand={}", hand.display());
    // It is there to be named by a row, and nothing runs on it unasked:
    // the fleet comes up on the assistant that is built in, as a new person's does.
    let copy =
        Started::told(&app.join(crate::places::PROGRAM), &["--command", runs.as_str(), "--walls", "off"])
            .await;
    copy.register().await;
    copy.run(&["project", "create", "Shop"]).await;

    let commands = commands(&text_of("docs/first-run.md"), "### Two ways of working a card");
    assert_eq!(commands.len(), 7, "{commands:?}");
    let both = |shown: &str, row: &str| {
        let cells = line(shown, row);
        cells.len() > 3 && cells[1] == "1" && cells[3] == "1"
    };
    let mut said = Vec::new();
    for command in &commands {
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        let out = match args[..2] {
            // "The fleet brings its agent up at once: `agent list` shows shop-careful-1."
            ["record", "add"] => {
                let added = copy.run(&args).await;
                let up = |listed: &str| {
                    listed.lines().any(|one| one.starts_with("shop-careful-1") && one.contains("active"))
                };
                until(&copy, &["agent", "list"], up).await;
                added
            }
            // On the assistant that is built in: each card given its way and taken, and no more.
            ["compare", "show"] => {
                until(&copy, &args, |shown| {
                    // Each way's card taken, and its agent's one turn counted as what it came to.
                    let cells = line(shown, "moved ready>doing");
                    let turns = line(shown, "agent.turn");
                    cells.len() > 3 && cells[1] == "1" && cells[3] == "1" && turns.len() == 5
                })
                .await
            }
            // Before it is ended: "give the rows their `adapter`, put the cards
            // back in ready, and they are worked under the ways they were given".
            ["compare", "end"] => {
                for row in ["shop-agents-2", "shop-agents-4"] {
                    copy.run(&["record", "set", row, "adapter=hand"]).await;
                }
                let on_it = |listed: &str| {
                    ["shop-worker-1", "shop-careful-1"]
                        .iter()
                        .all(|name| listed.lines().any(|one| one.starts_with(name) && one.contains("hand")))
                };
                until(&copy, &["agent", "list"], on_it).await;
                for card in ["shop-1", "shop-2"] {
                    copy.run(&["board", "move", card, "ready"]).await;
                }
                let worked = until(&copy, &["compare", "show", "workers"], |shown| {
                    let cells = line(shown, "moved doing>review");
                    cells.len() > 3 && cells[1] == "1" && cells[3] == "1"
                })
                .await;
                said.push(("worked".to_string(), worked));
                copy.run(&args).await
            }
            _ => copy.run(&args).await,
        };
        said.push((args[..2].join(" "), out));
    }
    let of =
        |name: &str| &said.iter().find(|(said, _)| said == name).unwrap_or_else(|| panic!("no `{name}`")).1;
    assert!(of("record add").contains("added shop-agents-4 to agents"), "{}", of("record add"));
    assert!(
        of("compare start")
            .contains("It counts what comes of each thing: moved, verdict, asked, left, after.")
    );
    // Side by side on the built-in assistant: a card each, taken, a turn each that failed, and both in doing.
    let shown = of("compare show");
    assert_eq!(line(shown, "things given")[1..], ["1", "1"], "{shown}");
    assert!(both(shown, "moved ready>doing"), "{shown}");
    assert!(line(shown, "moved doing>review").is_empty(), "{shown}");
    let turns = line(shown, "agent.turn");
    assert_eq!((turns[1], turns[3]), ("1 (1 failed)", "1 (1 failed)"), "{shown}");
    assert!(of("board shop").contains("doing (2)"), "{}", of("board shop"));
    // The rows told what to run on and the cards put back, each is worked under the way it was given.
    let worked = of("worked");
    assert_eq!(line(worked, "things given")[1..], ["1", "1"], "no card was given another way: {worked}");
    assert!(both(worked, "moved doing>review"), "{worked}");
    assert!(of("compare end").contains("everything goes `careful`"), "{}", of("compare end"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_of_the_program_puts_cards_in_order_as_the_first_runs_commands_are_written() {
    let (made, app) = copied("written-ordered");
    // Whoever plans is shown the cards a feature gathers, each beginning
    // a line with its id, and says they are done the first two side by
    // side and the third after both. Asked anything else, it leaves it.
    let planner = made.0.join("planner.sh");
    let plans = "#!/bin/sh\nasked=$(cat)\ncase \"$asked\" in\n  *'It gathers these cards'*)\n    \
                 set -- $(printf '%s\\n' \"$asked\" | sed -n 's/^\\([a-z]*-[0-9]*\\), in [a-z]*: .*/\\1/p')\n    \
                 printf 'The first two side by side, then the third.\\n\\n```tickets\\n'\n    \
                 printf '[{\"card\":\"%s\"},{\"card\":\"%s\"},{\"card\":\"%s\",\"after\":[1,2]}]\\n' \"$1\" \"$2\" \"$3\"\n    \
                 printf '```\\n'; echo 'DECISION: propose';;\n  *) echo 'There is nothing here for me.'; echo 'DECISION: leave';;\nesac\n";
    std::fs::write(&planner, plans).unwrap();
    std::fs::set_permissions(&planner, std::fs::Permissions::from_mode(0o755)).unwrap();
    let runs = format!("planner={}", planner.display());
    let agents = ["--adapter", "planner", "--command", runs.as_str(), "--walls", "off"];
    let copy = Started::told(&app.join(crate::places::PROGRAM), &agents).await;
    copy.register().await;
    // "A project is given features where it is asked for, or afterwards, as here."
    copy.run(&["project", "create", "Shop"]).await;

    let commands = commands(&text_of("docs/first-run.md"), "## 10. Cards put in order together");
    assert_eq!(commands.len(), 8, "{commands:?}");
    let mut said = Vec::new();
    for command in &commands {
        let args: Vec<&str> = command.iter().map(String::as_str).collect();
        // What is listed is asked until it says what the text says it
        // will: the feature with a plan, and then its cards in order.
        let until = match args[..] {
            ["record", "list", _, "features"] => Some("status planned"),
            ["record", "list", _, "board"] => Some("after shop-1, shop-2"),
            _ => None,
        };
        let out = match until {
            None => copy.run(&args).await,
            Some(words) => {
                soon("what the text says is listed to be there", async {
                    loop {
                        let shown = copy.run(&args).await;
                        if shown.contains(words) {
                            return shown;
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await
            }
        };
        said.push((args[..2].join(" "), out));
    }
    // The label gathered the three cards, and whoever plans said how.
    let features =
        &said.iter().find(|(name, out)| name == "record list" && out.contains("status planned")).unwrap().1;
    assert!(features.contains("gathers shop-1, shop-2, shop-3"), "{features}");
    // Accepted, each card says what it waits on and what it is part of.
    let board = &said.last().unwrap().1;
    let cards: Vec<&str> = board.lines().filter(|line| line.contains("part_of shop-features-1")).collect();
    assert_eq!(cards.len(), 3, "{board}");
    assert!(
        cards.iter().any(|card| card.starts_with("shop-3") && card.contains("after shop-1, shop-2")),
        "{board}"
    );
    assert!(!cards.iter().any(|card| !card.starts_with("shop-3") && card.contains("after shop-")), "{board}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_architect_asked_in_its_channel_to_put_cards_in_order_adds_a_feature_that_gathers_them_and_moves_nothing_on()
 {
    let (made, app) = copied("written-asked");
    // An architect that does as the fleet's row tells one to: asked in
    // its channel to put the cards with a label in order, it adds a
    // feature that gathers them, with the factory's own command and as
    // itself; shown the cards a feature gathers, it plans them; and it
    // moves nothing on.
    let architect = made.0.join("architect.sh");
    let does = format!(
        "#!/bin/sh\nasked=$(cat)\ncase \"$asked\" in\n  *'It gathers these cards'*)\n    \
         set -- $(printf '%s\\n' \"$asked\" | sed -n 's/^\\([a-z]*-[0-9]*\\), in [a-z]*: .*/\\1/p')\n    \
         printf 'The first two side by side, then the third.\\n\\n```tickets\\n'\n    \
         printf '[{{\"card\":\"%s\"}},{{\"card\":\"%s\"}},{{\"card\":\"%s\",\"after\":[1,2]}}]\\n' \"$1\" \"$2\" \"$3\"\n    \
         printf '```\\n'; echo 'DECISION: propose';;\n  *'labelled opening in order'*)\n    \
         \"$FACTORY_CLI\" record add \"{project}\" features 'title=Open the shop' status=backlog labelled=opening >/dev/null\n    \
         echo 'I have added a feature that gathers them; its plan is yours to accept.';;\n  \
         *) echo 'There is nothing here for me.'; echo 'DECISION: leave';;\nesac\n",
        project = format_args!("${}", brand::env_var("PROJECT")),
    );
    std::fs::write(&architect, does).unwrap();
    std::fs::set_permissions(&architect, std::fs::Permissions::from_mode(0o755)).unwrap();
    let runs = format!("architect={}", architect.display());
    let agents = ["--adapter", "architect", "--command", runs.as_str(), "--walls", "off"];
    let copy = Started::told(&app.join(crate::places::PROGRAM), &agents).await;
    copy.register().await;
    // A project that is given features as it is added.
    let created = copy.run(&["project", "create", "Shop", "--with", "features"]).await;
    assert!(created.contains("It has features"), "{created}");
    for title in ["The till", "The shelves", "The opening day"] {
        copy.run(&["board", "add", "shop", title, "--label", "opening"]).await;
    }
    copy.run(&[
        "post",
        "shop",
        "@shop-architect-1",
        "please",
        "put",
        "the",
        "cards",
        "labelled",
        "opening",
        "in",
        "order",
    ])
    .await;
    // The feature is there, asked for by the architect, and planned.
    let features = soon("the architect's feature to have its plan", async {
        loop {
            let listed = copy.run(&["record", "list", "shop", "features"]).await;
            if listed.contains("status planned") {
                return listed;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        features.contains("gathers shop-1, shop-2, shop-3") && features.contains("labelled opening"),
        "{features}"
    );
    assert!(features.contains("title Open the shop"), "{features}");
    let history = copy.run(&["board", "history", "shop-features-1"]).await;
    assert!(history.contains("shop-architect-1 asked for it"), "{history}");
    // Nothing was moved on, and no card says it is a piece of anything yet.
    assert!(!history.contains("it was accepted"), "{history}");
    let board = copy.run(&["record", "list", "shop", "board"]).await;
    assert!(!board.contains("part_of shop-features-1"), "{board}");
}
