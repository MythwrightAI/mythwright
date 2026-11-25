//! What of a text is done when it is applied, which is what two texts are
//! compared by when one is to take the other's place in a running instance.

use blueprint::{outset, version};

const LANE: &str = "\
blueprint: lane
version: 3
description: Takes a card and works on it.
permissions: [board]
params:
  item: { type: string, description: \"The card.\" }
  agent: builder
steps:
  take: { do: board.move, with: { id: \"${params.item}\", to: doing }, description: Claim it. }
  work:
    task:
      build: { do: shell, with: { run: make }, description: Build it. }
on:
  back:
    event: board.item.moved
    steps:
      go: { do: noop }
on_stop:
  - do: noop
";

#[test]
fn what_a_text_does_later_and_says_of_itself_is_not_part_of_how_it_begins() {
    let later = LANE
        .replace("go: { do: noop }", "go: { do: log, with: { message: again } }")
        .replace("  - do: noop", "  - do: log\n    with: { message: bye }")
        .replace("version: 3", "version: 4")
        .replace("permissions: [board]", "permissions: [board, channels]")
        .replace("Takes a card and works on it.", "Takes a card.")
        .replace("The card.", "Which card.")
        .replace("Claim it.", "Take it.")
        .replace("Build it.", "Make it.");
    assert_ne!(later, LANE);
    assert_eq!(outset(&later), outset(LANE));
    assert!(outset(LANE).is_some());
}

#[test]
fn comments_spacing_quoting_and_the_order_of_the_top_do_not_count() {
    let written = "\
# A lane.
blueprint: \"lane\"
steps:
  take:
    do: board.move   # claim it
    with: { id: '${params.item}', to: doing }
  work:
    task:
      build: { do: shell, with: { run: make } }
params:
  item: { type: string }
  agent: builder
";
    assert_eq!(outset(written), outset(LANE));
}

#[test]
fn a_step_a_param_or_their_order_changed_is_another_beginning() {
    let same = outset(LANE);
    for (from, to) in [
        ("to: doing", "to: review"),
        ("agent: builder", "agent: reviewer"),
        ("  agent: builder\n", "  agent: builder\n  channel: general\n"),
        ("run: make", "run: make test"),
        ("blueprint: lane", "blueprint: lane\nstate: { rounds: 0 }"),
    ] {
        let changed = LANE.replace(from, to);
        assert_ne!(changed, LANE, "{from}");
        assert_ne!(outset(&changed), same, "{from} -> {to}");
    }
    let swapped = "\
blueprint: lane
params:
  item: { type: string }
  agent: builder
steps:
  work:
    task:
      build: { do: shell, with: { run: make } }
  take: { do: board.move, with: { id: \"${params.item}\", to: doing } }
";
    assert_ne!(outset(swapped), same, "the steps the other way round");
}

#[test]
fn a_text_says_its_own_version_as_words_or_as_a_number_or_not_at_all() {
    assert_eq!(version(LANE).as_deref(), Some("3"));
    assert_eq!(version("blueprint: x\nversion: 2026-11\n").as_deref(), Some("2026-11"));
    assert_eq!(version("blueprint: x\n"), None);
    assert_eq!(version(": not a blueprint"), None);
}
