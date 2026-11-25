//! What moved between two texts of a blueprint, in words: a sentence for
//! each thing, what each is about, and what it leaves unsaid.

use blueprint::differs::{Difference, between, differences};

const WAS: &str = "\
blueprint: shop/round
version: 3
description: Works a card.
params:
  tone: { type: string, default: plain }
  most: { type: int, default: 3 }
steps:
  claim: { do: emit, with: { topic: shop.claimed } }
  build:
    task:
      fetch: { do: emit, with: { topic: shop.fetched } }
      make: { do: emit, with: { topic: shop.made } }
  tidy: { do: emit, with: { topic: shop.tidied } }
on:
  poked:
    event: shop.poke
    steps:
      say: { do: emit, with: { topic: shop.heard } }
      note: { do: emit, with: { topic: shop.noted } }
";

/// The same with a handler added, a step changed (inside a task, and
/// inside a handler), a param's default changed and a step taken away.
const NOW: &str = "\
blueprint: shop/round
version: 3
description: Works a card.
params:
  tone: { type: string, default: warm }
  most: { type: int, default: 3 }
steps:
  claim: { do: emit, with: { topic: shop.claimed } }
  build:
    task:
      fetch: { do: emit, with: { topic: shop.fetched } }
      make: { do: emit, with: { topic: shop.built } }
on:
  poked:
    event: shop.poke
    steps:
      say: { do: emit, with: { topic: shop.said } }
      note: { do: emit, with: { topic: shop.noted } }
  asked:
    event: shop.asked
    steps:
      answer: { do: emit, with: { topic: shop.answered } }
";

fn texts(said: &[Difference]) -> Vec<&str> {
    said.iter().map(|one| one.text.as_str()).collect()
}

#[test]
fn a_handler_added_a_step_changed_a_default_changed_and_a_step_taken_away_are_each_a_sentence() {
    let said = differences(WAS, NOW);
    assert_eq!(
        said,
        [
            "the param `tone` is changed",
            "the step `build` is changed",
            "the step `tidy` is gone",
            "the handler `asked` is new",
            "the handler `poked` is changed",
        ]
    );
    // The other way round, each is its opposite.
    assert_eq!(
        differences(NOW, WAS),
        [
            "the param `tone` is changed",
            "the step `build` is changed",
            "the step `tidy` is new",
            "the handler `poked` is changed",
            "the handler `asked` is gone",
        ]
    );
}

#[test]
fn each_thing_that_moved_says_what_it_is_about_and_what_moved_inside_it() {
    let said = between(WAS, NOW);
    assert_eq!(texts(&said), differences(WAS, NOW), "the sentences are the same ones");
    let about: Vec<(Option<&str>, Option<&str>)> =
        said.iter().map(|one| (one.at.as_deref(), one.key.as_deref())).collect();
    assert_eq!(
        about,
        [
            // A param is on no tile of the graph; what is gone is nowhere in the later text.
            (None, Some("tone")),
            (Some("main/build"), Some("build")),
            (None, Some("tidy")),
            (Some("on.asked/@"), Some("asked")),
            (Some("on.poked/@"), Some("poked")),
        ]
    );
    // Inside a task and inside a handler, the step that moved, by the name the graph gives it.
    let build = &said[1];
    assert_eq!(texts(&build.under), ["the step `build.make` is changed"]);
    assert_eq!(build.under[0].at.as_deref(), Some("main/build.make"));
    let poked = &said[4];
    assert_eq!(texts(&poked.under), ["the step `say` is changed"]);
    assert_eq!(poked.under[0].at.as_deref(), Some("on.poked/say"));
}

#[test]
fn what_else_of_a_text_is_another_is_said_where_everything_is_and_nowhere_else() {
    let other = NOW
        .replace("description: Works a card.", "description: Works a card twice.\nstate: { seen: 0 }")
        .replace(
            "      say: { do: emit, with: { topic: shop.said } }\n      note: { do: emit, with: { topic: shop.noted } }",
            "      note: { do: emit, with: { topic: shop.noted } }\n      say: { do: emit, with: { topic: shop.said } }",
        );
    // Whoever is told that a text moved is told what they always were.
    assert_eq!(differences(NOW, &other), ["the handler `poked` is changed"]);
    let said = between(NOW, &other);
    assert_eq!(
        texts(&said),
        [
            "the handler `poked` is changed",
            "what it says under `description` is changed",
            "it now says `state`"
        ]
    );
    assert_eq!(texts(&said[0].under), ["its steps are in another order"]);
    assert_eq!(texts(&between(&other, NOW))[2], "it no longer says `state`");
}

#[test]
fn a_text_laid_out_another_way_or_given_a_comment_has_not_moved() {
    let laid = blueprint::format(NOW, &blueprint::Settings::default()).expect("it is laid out");
    assert_ne!(laid, NOW, "laid out, it is written another way");
    let noted =
        format!("# A comment at its head.\n\n{}", laid.replace("steps:\n", "steps:\n  # About the steps.\n"));
    assert!(blueprint::means_the_same(NOW, &noted));
    assert_eq!(between(NOW, &noted), []);
    // And what moved is the same said of either.
    assert_eq!(between(WAS, &noted), between(WAS, NOW));
    assert_eq!(between(&noted, WAS), between(NOW, WAS));
}

#[test]
fn only_the_version_being_another_is_nothing_said_here() {
    // It is said first, by whoever says what moved.
    assert_eq!(between(WAS, &WAS.replace("version: 3", "version: 4")), []);
    // And a text that does not read as a blueprint has nothing said of it.
    assert_eq!(between(WAS, "steps: [1, 2]"), []);
}
