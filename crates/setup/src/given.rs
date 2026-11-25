//! `project.set_up`: what `std/project` asks, which gives a project each
//! of its rows that is on, as whoever applied the text.
//!
//! What each row comes to is the block's (`deciding::setups::came`, with
//! its cases): there already, left out for the lack of what it needs, or
//! given. Given is applied here, as `usual` applied each before there was
//! a text for it: by the runner as somebody asking (`Engine::apply`),
//! applied by whoever applied `std/project` and handed what the row says
//! and what it is told of the project, under no environment, and waited
//! for. So what is given is something that person applied, at the top
//! and not under `std/project`, as they could have applied it by hand;
//! what they may not apply they do not get by setting a project up.
//!
//! It is the same done twice: what is alive in the project is there and
//! is not applied again, so a step taken up again after a stop finds
//! given what it had given.

use std::collections::BTreeMap;

use blueprint::{ActionSpec, ValueType as T};
use conductor::{ActionCx, ActionError, ActionResult, Apply, Engine, InstanceFilter, Registry, RunStatus};
use deciding::setups::{self, Came, FLEET, Reached, Row};
use kernel::{BoxFut, Fields, Principal};
use serde_json::{Value, json};
use studio::Studio;
use studio::projects::Project;

/// Adds `project.set_up`, and what this crate says, to what a blueprint
/// can do here.
pub fn register(registry: &mut Registry, studio: Studio) {
    registry.emits(crate::events::events());
    let spec = ActionSpec::new(
        "project.set_up",
        "Set the project up with each of these rows that is on: each a blueprint applied for it as whoever applied this, saying what each came to.",
    )
    .required(
        "rows",
        T::List,
        "what the project is set up with, a row for each thing, given in order: its `name`, the `blueprint` applied for it, whether it is `on`, what it `needs` and `why`, which params are `told` the project and what it is handed (`with`)",
    )
    .arg("later", T::Bool, "whether the project was there before: what it has then counts, and what is alive in it is not applied again")
    .example(SET_UP_ASKED)
    .capability("blueprints:run")
    .output("given", "each row given, by its name: the instance that is it, and those it had already")
    .output("there", "the rows the project had already, by name")
    .output("left", "what was left out, a sentence for each")
    .idempotent();
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> {
        let studio = studio.clone();
        Box::pin(async move { set_up(&cx, &studio, &args).await })
    });
}

/// A text of somebody's own that sets a project up with a board and a
/// library kept in its notes folder.
const SET_UP_ASKED: &str = "\
blueprint: set-up-notes
permissions: [blueprints]

steps:
  given:
    do: project.set_up
    with:
      rows:
        - { name: board, blueprint: std/board }
        - { name: notes, blueprint: std/docs, with: { folder: notes } }
";

async fn set_up(cx: &ActionCx, studio: &Studio, args: &Fields) -> ActionResult {
    let project =
        cx.project().ok_or_else(|| ActionError::new("a project is set up by applying this for it"))?;
    let rows = setups::rows(args.get("rows").unwrap_or(&Value::Null)).map_err(ActionError::new)?;
    let later = args.get("later").and_then(Value::as_bool).unwrap_or(false);
    let project = studio.projects.get(project).await?;
    Ok(give(cx.engine(), studio, &project, cx.principal(), &rows, later).await)
}

/// Gives `project` each of `rows` that is on, as `who`, and says what it
/// was given (by the name of each: its instance), what it had already
/// (`there`, only of a project that was there before: `later`) and what it
/// was not, a sentence for each (`left`).
pub async fn give(
    engine: &Engine,
    studio: &Studio,
    project: &Project,
    who: &Principal,
    rows: &[Row],
    later: bool,
) -> Value {
    let id = project.id.as_str();
    let on: Vec<&Row> = rows.iter().filter(|row| row.on).collect();
    // What the project had before, of what something here needs: a project
    // that was only just added has nothing but what it is given here.
    let mut had = Vec::new();
    if later {
        for need in on.iter().flat_map(|row| &row.needs) {
            if had.contains(need) {
                continue;
            }
            if let Some(row) = rows.iter().find(|row| &row.name == need)
                && has(engine, studio, id, row).await
            {
                had.push(need.clone());
            }
        }
    }
    let mut given = BTreeMap::<String, Value>::new();
    let (mut names, mut there, mut left) = (Vec::<String>::new(), Vec::<String>::new(), Vec::<String>::new());
    for row in on {
        let alive = match later {
            true => alive(engine, &row.blueprint, id).await,
            false => None,
        };
        let reached =
            Reached { project: id, given: &names, had: &had, alive: alive.as_deref(), rows, cli: brand::CLI };
        match setups::came(row, &reached) {
            Came::There(instance) => {
                given.insert(row.name.clone(), json!(instance));
                names.push(row.name.clone());
                there.push(row.name.clone());
            }
            Came::Left(why) => left.push(why),
            Came::Give => {
                let applied = match row.blueprint == FLEET {
                    true => fleet(engine, studio, project, who, row).await,
                    false => apply(engine, row, id, who).await.map(Ok),
                };
                match applied {
                    Ok(Ok(instance)) => {
                        given.insert(row.name.clone(), json!(instance));
                        names.push(row.name.clone());
                    }
                    Ok(Err(file)) => left.push(setups::theirs(id, &file, brand::CLI)),
                    Err(why) => left.push(setups::failed(row, id, &why, brand::CLI)),
                }
            }
        }
    }
    json!({ "given": given, "there": there, "left": left })
}

/// Whether `project` has what `row` gives: its board where that is the
/// board, its environments where those, and otherwise something of its
/// blueprint running for it.
pub(crate) async fn has(engine: &Engine, studio: &Studio, project: &str, row: &Row) -> bool {
    match row.blueprint.as_str() {
        "std/board" => studio.board.columns(project).await.is_ok(),
        // What makes them ready holds nothing and is over once they are made: the project has them.
        "std/environments" => {
            engine.environments_of(project).await.is_ok_and(|environments| !environments.is_empty())
        }
        blueprint => alive(engine, blueprint, project).await.is_some(),
    }
}

/// What is alive in `project` of `blueprint`: an instance of it, the
/// newest where there are several. That is what the project's list of
/// the blueprints it uses goes by, and so what a page goes by that says
/// whether the project has it.
async fn alive(engine: &Engine, blueprint: &str, project: &str) -> Option<String> {
    let filter = InstanceFilter {
        live_only: true,
        project: Some(project.to_string()),
        blueprint: Some(blueprint.to_string()),
        limit: 1,
        ..InstanceFilter::default()
    };
    engine.instances(filter).await.ok()?.into_iter().next().map(|instance| instance.id)
}

/// What became of a fleet: its instance, or the file of one the project
/// came with, which was not applied.
type Fleet = std::result::Result<String, String>;

/// Writes the project's fleet blueprint and applies it, unless the
/// project came with one of its own, which is left for whoever added it
/// to read.
async fn fleet(
    engine: &Engine,
    studio: &Studio,
    project: &Project,
    who: &Principal,
    row: &Row,
) -> std::result::Result<Fleet, String> {
    let id = &project.id;
    // A folder that was already there may have come with a fleet blueprint
    // of its own. Whoever wrote the folder wrote it, and applying it would
    // be running their text for somebody who has not read it. One that is
    // word for word what this factory writes is nobody's text but ours: a
    // project that was let go of and is taken up again has its fleet back.
    let files = brand::blueprint_files(FLEET);
    let theirs = studio
        .projects
        .tree(id)
        .await
        .ok()
        .and_then(|tree| tree.into_iter().find(|entry| files.contains(&entry.path)));
    if let Some(theirs) = theirs
        && project.brownfield
        && !studio.projects.fleet_as_written(id).await.unwrap_or(false)
    {
        return Ok(Err(theirs.path));
    }
    studio.projects.init_fleet(id, who).await.map_err(|err| err.message)?;
    apply(engine, row, id, who).await.map(Ok)
}

/// Applies `row`'s blueprint to the project as `who`, handed what the row
/// says beside what it is told of the project, and waits for its steps:
/// the instance, or why there is none that is as its text says.
async fn apply(
    engine: &Engine,
    row: &Row,
    project: &str,
    who: &Principal,
) -> std::result::Result<String, String> {
    let blueprint = &row.blueprint;
    // What whoever asked set of it, and what it is told of the project:
    // which it is for, and to report where the project's fleet does.
    let mut params = row.with.clone();
    for param in &row.told {
        params.insert(param.clone(), json!(project));
    }
    let request = Apply {
        blueprint: blueprint.clone(),
        source: None,
        params,
        key: None,
        parent: None,
        target: None,
        project: Some(project.to_string()),
        principal: who.clone(),
        force: false,
        take: false,
        labels: Default::default(),
        // What a project starts with looks after its tables, which are
        // the project's whatever it deploys to: under no environment, said
        // so, in a project that has some too (`environments::held::chosen`).
        environment: Some(String::new()),
    };
    let applied = engine.apply(request).await.map_err(|err| err.message)?;
    if let Some(run) = &applied.run {
        let finished = engine.wait_run(run).await.map_err(|err| err.message)?;
        if finished.status != RunStatus::Ok {
            return Err(finished
                .error
                .unwrap_or_else(|| format!("applying `{blueprint}` did not finish well")));
        }
    }
    Ok(applied.instance)
}
