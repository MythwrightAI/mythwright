//! A text put together from parts (`conductor::words::put`, asked by a
//! blueprint as `text.put`): a table of what is handed and what it comes
//! to, what is refused, and once as a blueprint asks it.

use conductor::RunStatus;
use conductor::words::{Files, Put, put};
use serde_json::{Value, json};

use crate::Rig;

fn came_to(text: &str, said: bool) -> Put {
    Put { text: text.to_string(), said }
}

#[test]
fn parts_are_said_in_order_each_only_where_its_if_says_something() {
    let cases: Vec<(Value, Option<&str>, Put)> = vec![
        // Words alone are said as they are, with nothing put between them.
        (json!(["Card 12", ": ", "the price"]), None, came_to("Card 12: the price", true)),
        (json!(["one\n\n", "two"]), None, came_to("one\n\ntwo", true)),
        // A number is said as it is written; nothing says nothing.
        (json!(["You are one of ", 3, "."]), None, came_to("You are one of 3.", true)),
        (json!(["a", null, "b"]), None, came_to("ab", true)),
        // No parts, and parts that say nothing, come to nothing.
        (json!([]), None, came_to("", false)),
        (json!([null, ""]), None, came_to("", false)),
        // `if` that is true or false.
        (json!([{ "if": true, "say": "yes" }]), None, came_to("yes", true)),
        (json!([{ "if": false, "say": "yes" }]), None, came_to("", false)),
        (json!([{ "if": false, "say": "yes", "else": "no" }]), None, came_to("no", true)),
        (json!([{ "if": true, "say": "yes", "else": "no" }]), None, came_to("yes", true)),
        // `if` that is a text: said where it is not empty, as `!= ''` finds it.
        (json!([{ "if": "docs", "say": "in docs" }]), None, came_to("in docs", true)),
        (json!([{ "if": "", "say": "in docs" }]), None, came_to("", false)),
        (json!([{ "if": "", "say": "in docs", "else": "at the top" }]), None, came_to("at the top", true)),
        (json!([{ "if": " ", "say": "in docs", "else": "at the top" }]), None, came_to("in docs", true)),
        // `if` that is a list: said where it holds anything.
        (json!([{ "if": ["a"], "say": "some" }]), None, came_to("some", true)),
        (json!([{ "if": [], "say": "some", "else": "none" }]), None, came_to("none", true)),
        // `if` that is nothing: a name that led nowhere.
        (json!([{ "if": null, "say": "some", "else": "none" }]), None, came_to("none", true)),
        (json!([{ "if": null, "say": "some" }]), None, came_to("", false)),
        // What is said may itself be nothing, or a number.
        (json!([{ "if": true, "say": null }]), None, came_to("", false)),
        (json!([{ "if": true, "say": 2 }, " of them"]), None, came_to("2 of them", true)),
        (json!([{ "if": false, "say": "x", "else": null }]), None, came_to("", false)),
        // The first of two that says anything: a name, or what stands for it.
        (json!([{ "if": "ours", "say": "ours", "else": "worker" }]), None, came_to("ours", true)),
        (json!([{ "if": "", "say": "", "else": "worker" }]), None, came_to("worker", true)),
        // Several, some said and some not, in the order written.
        (
            json!([
                "It was passed. ",
                { "if": false, "say": "It could not be asked. " },
                { "if": true, "say": "Nobody holds it. " },
                "It is in done."
            ]),
            None,
            came_to("It was passed. Nobody holds it. It is in done.", true),
        ),
        // Words that are themselves parts: said where the part is, each where its own `if` says.
        (
            json!([
                "It came back",
                { "if": true, "say": [{ "if": true, "say": ", with this said:" }, { "if": false, "say": "." }] },
            ]),
            None,
            came_to("It came back, with this said:", true),
        ),
        (
            json!([{ "if": false, "say": [{ "if": true, "say": "never" }], "else": "new" }]),
            None,
            came_to("new", true),
        ),
        (json!([{ "if": true, "say": [] }]), Some("nothing"), came_to("nothing", false)),
        // What stands for parts that say nothing but space, and only then.
        (json!([]), Some("(it printed nothing)"), came_to("(it printed nothing)", false)),
        (json!([" \n"]), Some("(it printed nothing)"), came_to("(it printed nothing)", false)),
        (json!([{ "if": true, "say": "" }]), Some("nothing"), came_to("nothing", false)),
        (json!(["  out\n"]), Some("(it printed nothing)"), came_to("  out\n", true)),
        (json!([]), Some(""), came_to("", false)),
    ];
    for (parts, otherwise, comes_to) in cases {
        assert_eq!(put(&parts, otherwise, &Files::new()), Ok(comes_to), "{parts} otherwise {otherwise:?}");
    }
}

#[test]
fn parts_that_make_no_sense_are_refused_saying_which_and_why() {
    let cases: Vec<(Value, &str)> = vec![
        (json!(null), "`parts` is a list of what is said, in the order it is said; it is nothing"),
        (
            json!("Card 12"),
            "`parts` is a list of what is said, in the order it is said; it is the text `Card 12`",
        ),
        (
            json!({ "if": true, "say": "x" }),
            "`parts` is a list of what is said, in the order it is said; it is a mapping",
        ),
        (json!(["a", true]), "part 2 of `parts`: a part is words, and this is `true`"),
        (json!([["a"]]), "part 1 of `parts`: a part is words, and this is a list"),
        (
            json!(["a", "b", { "unless": true, "say": "x" }]),
            "part 3 of `parts`: it says `unless`; a part that is not words alone says `say` (words), `from` (a \
             file of `prompts/`, with what fills its blanks under `with`) or `own` (a file of the project's own \
             `prompts/`), and where it is only sometimes said, `if` and what is said `else`",
        ),
        (
            json!([{ "say": "x" }]),
            "part 1 of `parts`: it says `say` with no `if`: words that are always said are written alone",
        ),
        (
            json!([{ "if": true }]),
            "part 1 of `parts`: it says neither `say` (words), `from` (a file of `prompts/`) nor `own` (a file of the project's own `prompts/`)",
        ),
        (
            json!([{ "if": 2, "say": "x" }]),
            "part 1 of `parts`: `if` is true or false, or a text or a list that may say nothing; it is the number \
             2, and what is to be asked of that is a question (`${ x > 0 }`)",
        ),
        (
            json!([{ "if": { "a": 1 }, "say": "x" }]),
            "part 1 of `parts`: `if` is true or false, or a text or a list that may say nothing; it is a mapping, \
             and what is to be asked of that is a question (`${ x > 0 }`)",
        ),
        (
            json!([{ "if": true, "say": { "x": 1 } }]),
            "part 1 of `parts`: `say` is words, and this is a mapping",
        ),
        // Parts said in the place of words are held to what a part is, each said where it stands.
        (
            json!([{ "if": true, "say": ["x", { "say": "y" }] }]),
            "part 1 of `parts`: part 2 of its `say`: it says `say` with no `if`: words that are always said are \
             written alone",
        ),
        (
            json!([{ "if": false, "say": "x", "else": false }]),
            "part 1 of `parts`: `else` is words, and this is `false`",
        ),
    ];
    for (parts, why) in cases {
        assert_eq!(put(&parts, None, &Files::new()), Err(why.to_string()), "{parts}");
    }
    // What is said where `if` does not hold is not looked at, as a step
    // that is passed over is not: only what is said is held to being words.
    assert_eq!(
        put(&json!([{ "if": true, "say": "x", "else": false }]), None, &Files::new()),
        Ok(came_to("x", true))
    );
}

const ASKED: &str = r#"
blueprint: asked
params:
  item: { type: string }
  changed: { type: string, default: "" }
  among: { type: int, default: 1 }
  maybe: { type: any, default: null }
steps:
  words:
    do: text.put
    with:
      parts:
        - "Card ${params.item}. "
        - { if: "${params.changed}", say: "What was changed: ${params.changed}. " }
        - { if: "${ params.among > 1 }", say: "You are one of ${params.among}.", else: "You look alone." }
        - { if: "${params.maybe.said}", say: "never" }
  nothing:
    do: text.put
    with:
      parts: ["${params.changed}"]
      otherwise: "(nothing was changed)"
outputs:
  text: "${steps.words.out.text}"
  said: ${steps.words.out.said}
  nothing: "${steps.nothing.out.text}"
  none_said: ${steps.nothing.out.said}
"#;

#[tokio::test]
async fn a_blueprint_asks_for_a_text_of_parts_and_is_given_it_and_whether_anything_was_said() {
    let rig = Rig::new().await;
    let run = rig.run(ASKED, json!({ "item": "shop-12" })).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(
        run.outputs,
        json!({
            "text": "Card shop-12. You look alone.",
            "said": true,
            "nothing": "(nothing was changed)",
            "none_said": false
        })
    );
    let run = rig.run(ASKED, json!({ "item": "shop-12", "changed": "a.rs", "among": 3 })).await;
    assert_eq!(
        run.outputs,
        json!({
            "text": "Card shop-12. What was changed: a.rs. You are one of 3.",
            "said": true,
            "nothing": "a.rs",
            "none_said": true
        })
    );
}

#[tokio::test]
async fn a_step_that_hands_it_parts_that_make_no_sense_fails_with_the_sentence() {
    let rig = Rig::new().await;
    let source =
        "blueprint: wrong\nsteps:\n  words: { do: text.put, with: { parts: [{ if: 3, say: three }] } }\n";
    let run = rig.run(source, json!({})).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = run.error.unwrap_or_default();
    assert!(why.contains("part 1 of `parts`: `if` is true or false, or a text or a list"), "{why}");
}
