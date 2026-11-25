//! What a blueprint asks of a project's outline: that things be shown in
//! a folder of it (`outline.put`), as whoever applied the blueprint, so
//! that a blueprint or a script that declares a table can show it where
//! somebody would look for it. A person, a terminal and a blueprint are
//! equal doors: this asks what the route asks, of the one keeper.

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionError, Registry};
use kernel::{BoxFut, Fields};
use serde_json::{Value, json};

use crate::Outlines;

/// An example, for the Reference.
const PUT: &str = "\
blueprint: runs-shown
description: Keep a table of the runs of a script, shown in a folder of the project called Insights.
permissions: [projects]

steps:
  runs:
    do: collection.ensure
    with:
      name: test-runs
      fields: { title: { type: text, required: true } }

  shown:
    needs: [runs]
    do: outline.put
    with: { things: [\"table:test-runs\"], folder: Insights }
";

fn words(args: &Fields, key: &str) -> Result<Vec<String>, ActionError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::String(one)) => Ok(vec![one.clone()]),
        Some(Value::Array(several)) => several
            .iter()
            .map(|one| one.as_str().map(str::to_string))
            .collect::<Option<Vec<String>>>()
            .ok_or_else(|| ActionError::new(format!("`{key}` is a list of words"))),
        Some(other) => Err(ActionError::new(format!("`{key}` is a list of words; it was given {other}"))),
    }
}

/// Adds `outline.put`.
pub fn register(registry: &mut Registry, outlines: Outlines) {
    registry.add_fn(
        ActionSpec::new(
            "outline.put",
            "Show some of a project's things in a folder of the side panel, making the folder where it is not there.",
        )
        .example(PUT)
        .required(
            "things",
            T::List,
            "what is put, each by its key as `project outline` lists it: `board`, `table:<name>`, `library:<name>`, \
             `mockups:<name>`, `environments`, `pipelines`, `blueprints`, `git` or `repo`",
        )
        .arg(
            "folder",
            T::String,
            "the folder they are put in, by its names from the top joined by `/` (`Insights/Runs`), whatever their \
             capitals: each that is not there is made at the end of the one before it. The top of the project where \
             it is said none",
        )
        .arg("project", T::String, "which project (default: this instance's)")
        .capability("projects:write")
        .output("folder", "the folder they were put in, by its id; nothing for the top")
        .output("changed", "whether anything was moved or made: asked again, nothing is")
        // The same done twice: a folder of the path that is there is the
        // one used, and a thing already in it stays where it is.
        .idempotent(),
        move |cx, args| -> BoxFut<'static, conductor::ActionResult> {
            let outlines = outlines.clone();
            Box::pin(async move {
                let project = match args.get("project").and_then(Value::as_str) {
                    Some(project) => project.to_string(),
                    None => cx.project().map(str::to_string).ok_or_else(|| {
                        ActionError::new("which project? give `project`, or apply this blueprint within one")
                    })?,
                };
                let path = args
                    .get("folder")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .split('/')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .collect();
                let changed = outlines.put_in(&project, path, words(&args, "things")?, cx.principal()).await?;
                Ok(json!({ "folder": changed.done["folder"], "changed": changed.changed }))
            })
        },
    );
}
