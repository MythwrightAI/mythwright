//! What is plain in a blueprint and what is worked out: asked of the
//! judge an expression at a time, and of a whole text.

use reader::expr;
use reader::plain::{plain, unplain};

fn judged(source: &str) -> Result<(), &'static str> {
    plain(&expr::parse(source).unwrap_or_else(|err| panic!("`{source}` does not read: {}", err.message)))
}

#[test]
fn a_name_a_plain_question_of_one_and_three_of_those_joined_by_one_word_are_plain() {
    for source in [
        // What a thing is called.
        "item",
        "params.column",
        "steps.card.out.status",
        "event.data.id",
        "steps.rows.out.rows[0]",
        "state.given[item]",
        "steps.card.out.more[params.field]",
        // A word, a number, written out.
        "'ready'",
        "3",
        "-1",
        "true",
        // Compared with a word, a number or another name; looked for among several.
        "steps.decided.out.came == 'begin'",
        "params.agents != 0",
        "event.data.to == params.column",
        "steps.card.out.status in [params.column, 'doing']",
        "event.data.card in state.reviewing",
        // With what stands for it when it says nothing.
        "event.data.note ?? ''",
        "params.title ?? params.column",
        "steps.made.out.dir ?? params.dir ?? ''",
        "state.back ?? []",
        "(event.data.clear ?? false) == true",
        "(steps.card.out.more.taken ?? '') != ''",
        // Held against a number written out, either way round.
        "params.agents > 0",
        "steps.card.out.rounds >= 3",
        "(steps.card.out.rounds ?? 0) < 2",
        "0 < params.agents",
        "params.below <= -1",
        "!(params.agents > 0)",
        // `!` of one of those.
        "!params.merge",
        "!(event.data.clear ?? false)",
        "!(steps.card.out.status in ['done', 'review'])",
        // At most three of those joined, by one word.
        "params.does == 'review' && params.channel != ''",
        "steps.card.failed || steps.decided.out.came == 'gone'",
        "!steps.mine.failed && (event.data.note ?? '') != ''",
        "a == 1 && b == 2 && c == 3",
        "a == 1 || !b || (c ?? '') != ''",
        "params.agents > 0 && params.merge && !steps.card.failed",
        // The same three, however they are bracketed: it means the same.
        "a == 1 && (b == 2 && c == 3)",
    ] {
        assert_eq!(judged(source), Ok(()), "`{source}`");
    }
}

#[test]
fn what_is_worked_out_is_not_plain_and_says_which_way() {
    for (source, why) in [
        ("len(state.back)", "it calls a function"),
        ("len(state.back) == 0", "it calls a function"),
        ("without(state.back, event.data.card)", "it calls a function"),
        ("params.title != '' ? params.title : params.column", "it chooses with `? :`"),
        ("params.column + ' is full'", "it adds, takes away, multiplies or divides"),
        ("params.seats - 1", "it adds, takes away, multiplies or divides"),
        ("params.seats <= params.agents", "it asks which of two names is the greater"),
        ("params.title > 'm'", "it asks which of two names is the greater"),
        ("1 < 2", "it asks which of two names is the greater"),
        ("len(state.back) > 0", "it calls a function"),
        ("params.agents > params.seats - 1", "it adds, takes away, multiplies or divides"),
        ("(a > 0) == b", "it compares what was itself compared"),
        ("a == 1 && b == 2 && c == 3 && d == 4", "it joins more than three things"),
        ("a || b || c || d", "it joins more than three things"),
        ("a == 1 || (b == 2 && c == 3)", "it joins with both `&&` and `||`"),
        ("a == 1 && b == 2 || c == 3", "it joins with both `&&` and `||`"),
        ("(a || b) && c", "it joins with both `&&` and `||`"),
        ("!(a && b)", "it joins things where one thing is wanted"),
        ("(a || b) == c", "it joins things where one thing is wanted"),
        ("a ?? (b || c)", "it joins things where one thing is wanted"),
        ("(a == b) == c", "it compares what was itself compared"),
        ("a ?? string(b)", "it calls a function"),
        ("a ?? (b == c)", "it compares what was itself compared"),
        ("(a ?? b).c", "what stands for a thing is itself worked out"),
        ("rows[len(rows) - 1]", "it looks a thing up by something worked out"),
        ("-params.seats", "it takes a thing from nothing"),
        ("!!params.merge", "it says `!` of a `!`"),
        ("[params.a, params.b]", "it makes a list"),
        ("a in [string(b)]", "it calls a function"),
        ("'a' in ['a', 'b'] == true", "it compares what was itself compared"),
    ] {
        assert_eq!(judged(source), Err(why), "`{source}`");
    }
}

/// A text with something worked out in each place an expression is
/// written: a condition, a hole in text, what a loop goes over, a list.
const WATCHED: &str = r#"blueprint: watched
params:
  column: { type: string }
steps:
  card:
    do: board.get
    with: { id: "${event.data.id}" }
  told:
    when: ${ steps.card.out.status == params.column && len(state.back) > 0 }
    do: channel.post
    with:
      channel: "${params.channel}"
      body: "${event.data.id} is in ${ params.title != '' ? params.title : params.column } again."
  each:
    for_each: ${ pluck(steps.rows.out.rows, "id") }
    do: emit
    with: { topic: looked, at: "${item}", list: ["${ params.seats - 1 }", plain] }
  bare:
    when: steps.card.ok && !steps.told.failed
    do: set
    with: { seen: true }
"#;

#[test]
fn every_expression_of_a_text_is_looked_at_wherever_it_is_written_and_said_by_its_line() {
    let found = unplain(WATCHED).unwrap();
    let said: Vec<(u32, &str, &str)> =
        found.iter().map(|one| (one.line, one.source.as_str(), one.why)).collect();
    assert_eq!(
        said,
        [
            (9, "steps.card.out.status == params.column && len(state.back) > 0", "it calls a function"),
            (13, "params.title != '' ? params.title : params.column", "it chooses with `? :`"),
            (15, "pluck(steps.rows.out.rows, \"id\")", "it calls a function"),
            (17, "params.seats - 1", "it adds, takes away, multiplies or divides"),
        ]
    );
}

const QUIET: &str = r#"blueprint: quiet
steps:
  one:
    when: ${ params.go ?? false }
    do: set
    with: { a: "$${not a hole}", b: "${params.b}" }
"#;

const BROKEN: &str = r#"blueprint: broken
steps:
  one:
    with: { a: "${ params. }" }
"#;

#[test]
fn a_text_with_nothing_worked_out_has_nothing_said_of_it_and_one_that_cannot_be_read_says_so() {
    assert_eq!(unplain(QUIET).unwrap(), []);
    let why = unplain(BROKEN).unwrap_err();
    assert!(why.starts_with("line 4:"), "{why}");
}

#[test]
fn words_of_somebodys_own_under_a_key_that_is_a_condition_elsewhere_are_not_read_as_one() {
    // `when` in what a step hands an action is that action's to read.
    let text = "blueprint: said\nsteps:\n  one:\n    do: remind\n    with: { when: \"tomorrow, at nine\" }\n";
    assert_eq!(unplain(text).unwrap(), []);
}
