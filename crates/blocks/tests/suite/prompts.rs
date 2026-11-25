//! A file a blueprint links (`from:` of `text.put`) as a text of the
//! library: found by its name beside the blueprint, held to being there
//! and to having its blanks handed when the blueprint is checked, and
//! listed among no blueprints.

use conductor::{Library, RunStatus};
use serde_json::json;

use crate::Rig;

const ASKS: &str = r#"
blueprint: shop/ask
params:
  item: { type: string }
  among: { type: int, default: 1 }
steps:
  words:
    do: text.put
    with:
      parts:
        - from: ask.md
          with:
            item: "${params.item}"
            among:
              - { if: "${ params.among > 1 }", from: ask/among.md, with: { among: "${params.among}" } }
        - "\n"
outputs:
  text: "${steps.words.out.text}"
"#;

fn with_files(rig: &Rig) {
    rig.library.put("shop/prompts/ask.md", "Look at ${item}.${among}\n");
    rig.library.put("shop/prompts/ask/among.md", " You are one of ${among}.\n");
}

#[tokio::test]
async fn a_step_is_handed_the_file_it_links_from_the_prompts_beside_its_blueprint() {
    let rig = Rig::new().await;
    with_files(&rig);
    let run = rig.run(ASKS, json!({ "item": "shop-12" })).await;
    assert_eq!(run.status, RunStatus::Ok, "{:?}", run.error);
    assert_eq!(run.outputs["text"], "Look at shop-12.\n");
    let run = rig.run(ASKS, json!({ "item": "shop-12", "among": 3 })).await;
    assert_eq!(run.outputs["text"], "Look at shop-12. You are one of 3.\n");

    // What is linked is no blueprint, and is listed among none.
    let names = rig.library.names(None);
    assert!(!names.iter().any(|name| name.contains("prompts/")), "{names:?}");
    assert_eq!(rig.library.load("shop/prompts/ask.md", None).as_deref(), Some("Look at ${item}.${among}\n"));
}

#[tokio::test]
async fn a_blueprint_that_links_a_file_that_is_not_there_is_refused_when_it_is_applied_saying_where_it_looked()
 {
    let rig = Rig::new().await;
    // Neither file is there.
    let refused =
        rig.engine.apply(rig.request(ASKS, json!({ "item": "shop-12" }))).await.unwrap_err().message;
    assert!(
        refused.contains(
            "this step links `shop/prompts/ask.md`, which is not there: it is a file of `prompts/` beside the \
             blueprint that links it. Write it as `@blueprints/shop/prompts/ask.md` in the project"
        ),
        "{refused}"
    );
    // A copy of a blueprint that ships, kept where its links do not reach
    // the files that ship: what to do is said, both ways.
    rig.library.put("std/prompts/ask.md", "Look at ${item}.${among}\n");
    let refused =
        rig.engine.apply(rig.request(ASKS, json!({ "item": "shop-12" }))).await.unwrap_err().message;
    assert!(
        refused.contains(
            "which is not there: it is a file of `prompts/` beside the blueprint that links it. A file of that \
             name ships (`std/prompts/ask.md`): keep this blueprint under `@blueprints/std/` and it is told what \
             ships, or copy that file to `@blueprints/shop/prompts/ask.md` in the project"
        ),
        "{refused}"
    );
    // The file is there and what it hands a blank is not.
    rig.library.put("shop/prompts/ask.md", "Look at ${item}.${among}\n");
    let refused =
        rig.engine.apply(rig.request(ASKS, json!({ "item": "shop-12" }))).await.unwrap_err().message;
    assert!(refused.contains("this step links `shop/prompts/ask/among.md`, which is not there"), "{refused}");
}

#[tokio::test]
async fn a_linked_file_that_leaves_a_blank_unhanded_or_works_something_out_is_refused_when_its_blueprint_is_applied()
 {
    let rig = Rig::new().await;
    with_files(&rig);
    rig.library.put("shop/prompts/ask.md", "Look at ${item}, ${title}.${among}\n");
    let refused =
        rig.engine.apply(rig.request(ASKS, json!({ "item": "shop-12" }))).await.unwrap_err().message;
    assert!(
        refused.contains(
            "this step links `shop/prompts/ask.md`: line 1: its blank `${title}` is handed nothing: say `title` under \
             `with`"
        ),
        "{refused}"
    );
    rig.library.put("shop/prompts/ask.md", "Look at ${ upper(item) }.${among}\n");
    let refused =
        rig.engine.apply(rig.request(ASKS, json!({ "item": "shop-12" }))).await.unwrap_err().message;
    assert!(
        refused.contains(
            "this step links `shop/prompts/ask.md`: line 1: its blank `${ upper(item) }` works something out"
        ),
        "{refused}"
    );
}

#[tokio::test]
async fn a_file_named_when_the_step_runs_and_not_there_fails_the_step_saying_where_it_was_looked_for() {
    let rig = Rig::new().await;
    let source = "blueprint: shop/later\nparams: { which: { type: string } }\nsteps:\n  words: { do: text.put, with: { \
                  parts: [{ from: \"${params.which}\" }] } }\n";
    let run = rig.run(source, json!({ "which": "gone.md" })).await;
    assert_eq!(run.status, RunStatus::Failed);
    let why = run.error.unwrap_or_default();
    assert!(
        why.contains(
            "`gone.md` is linked and is not there: it is a file of `prompts/` beside the blueprint `shop/later`, \
             and was looked for as `shop/prompts/gone.md`"
        ),
        "{why}"
    );
}
