//! `environment.ensure`: a project's environments made ready by a
//! blueprint, as `mw project environment add` makes one.
//!
//! What an environment is and how it is kept with what is said of it is
//! the runner's (`conductor::environs`, over `environments::project`).
//! This is only the asking, for the project the instance is applied for
//! and no other: each name it has not got is made, with no variables, and
//! one it has is left as it is, so that it is the same done twice. What
//! making one takes is what the route asks of whoever makes one by hand
//! (writing the project, `shell` and `secrets:use`, since what an
//! environment comes to say reaches the commands and the secrets of
//! whatever is applied under it), asked of whoever applied the text.

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionError, ActionResult, Registry};
use kernel::{BoxFut, Error};
use serde_json::{Value, json};

/// What making an environment takes, beside writing the project, which the action says it needs.
const MAKING: &[&str] = &["shell", "secrets:use"];

/// Adds `environment.ensure` to what a blueprint can do.
pub fn register(registry: &mut Registry) {
    let spec = ActionSpec::new(
        "environment.ensure",
        "Make ready the environments of the project this is applied for, each with no variables yet: one it has \
         already is left as it is. Making one takes what making one by hand does: writing the project, `shell` and \
         `secrets:use`.",
    )
    .example(ENSURE_ASKED)
    .required(
        "names",
        ValueType::List,
        "the environments the project is to have, each a name such as `dev`, `staging` or `prod`",
    )
    .capability("projects:write")
    .output("made", "the names that were made now")
    .output("there", "the names the project had already")
    .idempotent();
    registry.add_fn(spec, |cx, args| -> BoxFut<'static, ActionResult> {
        Box::pin(async move {
            let project = cx.project().map(str::to_string).ok_or_else(|| {
                Error::invalid(
                    "`environment.ensure` makes a project's environments, and this is applied for no project: \
                     apply it with `--project`",
                )
            })?;
            let who = cx.principal().clone();
            for scope in MAKING {
                who.require(scope)?;
            }
            let names: Vec<String> = match args.get("names") {
                Some(Value::Array(names)) => names
                    .iter()
                    .map(|name| {
                        name.as_str().map(str::to_string).ok_or_else(|| {
                            ActionError::from(Error::invalid(format!(
                                "`names` holds {name}, which is no name: each is a word such as `dev`"
                            )))
                        })
                    })
                    .collect::<Result<_, _>>()?,
                _ => Vec::new(),
            };
            let engine = cx.engine();
            let had: Vec<String> =
                engine.environments_of(&project).await?.into_iter().map(|environment| environment.name).collect();
            let (mut made, mut there) = (Vec::new(), Vec::new());
            for name in names {
                // Said twice, it is one environment.
                if made.contains(&name) || there.contains(&name) {
                    continue;
                }
                if had.contains(&name) {
                    there.push(name);
                    continue;
                }
                engine.make_environment(&project, &name, &who, cx.source()).await?;
                made.push(name);
            }
            Ok(json!({ "made": made, "there": there }))
        })
    });
}

// What the action is shown by in the Reference and the editor: a whole blueprint
// that asks it, held to checking with no warning (the API suite's `examples.rs`).
const ENSURE_ASKED: &str = "\
blueprint: ready-to-deploy
permissions: [projects, shell, secrets]

steps:
  made:
    do: environment.ensure
    with: { names: [dev, staging, prod] }
";
