//! `text.put`: a text put together from parts, as a blueprint asks it.
//!
//! What the parts come to is a function of the parts and of the files
//! they link, and is the language's ([`blueprint::parts`], which the
//! runner offers as `conductor::words`). What is asked of the runner is
//! finding each linked file as a text of the library is found, on the
//! instance's behalf, before the parts are put together.

use blueprint::{ActionSpec, ValueType};
use conductor::words::{ACTION, FOLDER, Files, linked_as, links, own_as, own_key, owned, put};
use conductor::{ActionError, ActionResult, Registry};
use kernel::BoxFut;
use serde_json::{Value, json};

/// Adds `text.put`, which is [`put`] for a blueprint.
pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(ACTION, "Put a text together from parts, each said only where it is to be.")
.example(PUT_ASKED)
        .deciding()
        .required(
            "parts",
            ValueType::List,
            "what is said, in order: words; or `{ say: ... }` (words, or parts of its own) or `{ from: <file>, \
             with: { ... } }` (a file of `prompts/` beside the blueprint, and what fills its blanks) or `{ own: \
             <file> }` (a file of the `prompts/` of the project's own blueprints, as it was vouched for), with \
             `if: ...` beside it where it is only sometimes said, and `else: ...` for what is said where it is \
             not",
        )
        .arg("otherwise", ValueType::String, "what it comes to where the parts say nothing but space")
        .output("text", "the parts that were said, put together")
        .output("said", "whether any of them said anything")
        .idempotent();
    registry.add_fn(spec, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let parts = args.get("parts").unwrap_or(&Value::Null);
            // Each file that is said is found as a text of the library is,
            // on this instance's behalf: a project's own as it was vouched
            // for, and never by a path put together here.
            //
            // One that ships is the one this instance's tree reached, as a
            // blueprint it starts by name is (`conductor::pinned`): a text
            // and the words it links are released together, and the
            // words of a later program may have blanks this text never
            // hands.
            let mut files = Files::new();
            let mut held = Vec::new();
            for from in links(parts).map_err(ActionError::new)? {
                let name = linked_as(cx.blueprint(), &from);
                // A way of a comparison may have given what started this
                // other words for the file (`conductor::comparing`).
                if let Some(instead) = cx.engine().read_instead(cx.instance(), &name).await {
                    files.insert(from, instead);
                    continue;
                }
                let found = cx.engine().linked_for(cx.instance(), cx.principal(), &name, cx.project()).await;
                let Some(found) = found else {
                    let folder = conductor::library::folder_of(cx.engine().library().as_ref(), cx.project());
                    return Err(ActionError::new(format!(
                        "`{from}` is linked and is not there: it is a file of `{FOLDER}/` beside the blueprint \
                         `{}`, and was looked for as `{name}`. {}",
                        cx.blueprint(),
                        conductor::library::folders::write_as(&folder, &name, cx.project())
                    )));
                };
                if let Some(there) = found.there {
                    held.push((from.clone(), there));
                }
                files.insert(from, found.text);
            }
            // A file of the project's own (`own:`) is found as a project's
            // own text is, on this instance's behalf and as it was vouched
            // for: what a row names, written for the project by somebody
            // who holds what the instance runs with. Nothing ships under
            // such a name, so there is nothing to stand in for it.
            for own in owned(parts).map_err(ActionError::new)? {
                let name = own_as(&own);
                let found = match cx.project() {
                    Some(project) => {
                        cx.engine().text_within(cx.instance(), cx.principal(), &name, Some(project)).await
                    }
                    None => None,
                };
                let Some(text) = found else {
                    let folder = conductor::library::folder_of(cx.engine().library().as_ref(), cx.project());
                    let whose = cx.project().map_or("no project".to_string(), |project| format!("`{project}`"));
                    return Err(ActionError::new(format!(
                        "`{own}` is named as a file of the project's own `{FOLDER}/`, and {whose} has none that \
                         this may read. {} A file written outside the factory is read once somebody who holds what \
                         this runs with saves it through the factory, or applies this again",
                        conductor::library::folders::write_as(&folder, &name, cx.project())
                    )));
                };
                files.insert(own_key(&own), text);
            }
            let otherwise = args.get("otherwise").and_then(Value::as_str);
            let put = match put(parts, otherwise, &files) {
                // Words it was held to that this text cannot fill are not
                // gone on with: the ones there now are tried, as a
                // blueprint this program can no longer run is.
                Err(_) if !held.is_empty() => {
                    files.extend(held);
                    put(parts, otherwise, &files)
                }
                put => put,
            };
            let put = put.map_err(ActionError::new)?;
            Ok(json!({ "text": put.text, "said": put.said }))
        })
    });
}

// What the actions here are shown by in the Reference and the editor: each a whole
// blueprint that asks it, held to checking with no warning (the API suite's `examples.rs`).
const PUT_ASKED: &str = "\
blueprint: greeting-words

params:
  who: \"\"

steps:
  words:
    do: text.put
    with:
      parts: [\"Hello,\", { say: \"${params.who}\", if: \"${ params.who != '' }\", else: everyone }]

  said:
    do: log
    with: { message: \"${steps.words.out.text}.\" }
";
