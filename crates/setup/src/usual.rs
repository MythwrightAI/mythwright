//! What a new project may start with: a board, which is where work is
//! written up; a fleet, which is where it is said who works it; a
//! library, its documents as pages; and, asked for, its features, its
//! card times, A/B tests, its pipelines and its environments.
//!
//! None of it is listed here. Setting a project up is a blueprint,
//! `std/project`, applied for the project as whoever adds it, and what it
//! starts with is its rows: each a blueprint (one that ships, a
//! package's, or one of somebody's own) with what it is handed. What
//! ships as the usual is that blueprint's own rows ([`shipped`]); a
//! factory may keep others ([`crate::setups`]), one of them its default
//! for new projects ([`the_usual`]). Each row is applied as whoever added
//! the project, exactly as they would apply it by hand, so there is one
//! way a board, a fleet or a library comes to be and this is only the
//! asking. What is left out, and why, is said in the answer and never an
//! error: the project was added, and what it lacks can be given it
//! afterwards.
//!
//! Whoever adds a project says what it starts with (`set_up`): `true`
//! for the default's rows that are on, a list of their names (each with
//! what its blueprint is handed where it says, or a row of its own), or a
//! setup by its name. A request that says nothing of it adds the project
//! alone, as it always did.
//!
//! Each row says where a person is offered it (`offered`): as the project
//! is added, or on the page of something the project then has. That is
//! where its switch is drawn and nothing more: asked for by name, any of
//! them is given as the project is added, and any of them is given a
//! project that is there already ([`give_later`]), which is the same
//! asking of the same blueprint as whoever asks.

use conductor::{Apply, Engine, RunStatus};
use deciding::setups::{self, Row};
use kernel::{Error, Principal, Result};
use serde_json::{Value, json};
use studio::Studio;
use studio::projects::Project;

use crate::setups::{DEFAULT, one};

/// What sets a project up.
pub const PROJECT: &str = "std/project";

/// What ships as the usual: `std/project`'s own rows, as its text says
/// them. A text that cannot be read has none, and nothing is offered.
pub fn shipped(engine: &Engine) -> Vec<Row> {
    let read = engine
        .library()
        .load(PROJECT, None)
        .and_then(|text| blueprint::check(&text, &blueprint::NoCatalog).blueprint);
    let default = read.and_then(|read| read.params.into_iter().find(|param| param.name == "rows")?.default);
    default.and_then(|rows| setups::rows(&rows).ok()).unwrap_or_default()
}

/// What a new project is set up with where whoever adds it says nothing
/// else: the factory's default where it keeps one, and what ships where
/// it does not.
pub async fn the_usual(engine: &Engine) -> Result<Vec<Row>> {
    match one(engine.journal(), DEFAULT).await? {
        Some(kept) => Ok(kept.rows),
        None => Ok(shipped(engine)),
    }
}

/// The rows of the setup called `name`: the factory's default, or one it
/// keeps under that name, or refused, saying what there is.
pub async fn of_setup(engine: &Engine, name: &str) -> Result<Vec<Row>> {
    if name == DEFAULT {
        return the_usual(engine).await;
    }
    match one(engine.journal(), name).await? {
        Some(kept) => Ok(kept.rows),
        None => {
            let kept = crate::setups::saved(engine.journal()).await?;
            let names: Vec<String> =
                kept.iter().filter(|one| one.name != DEFAULT).map(|one| format!("`{}`", one.name)).collect();
            let there = match names.is_empty() {
                true => "none is kept but the factory's default".to_string(),
                false => format!("there are {}", names.join(", ")),
            };
            Err(Error::not_found(format!("there is no setup called `{name}`: {there}")))
        }
    }
}

/// What somebody is told who asked for a record of a table the project
/// has not got, or for its records, where the table is one a row of
/// `rows` gives: the refusal, with the one command that gives the project
/// that table. Anything else is handed back as it came.
pub fn lacking(rows: &[Row], project: &str, table: &str, refused: Error) -> Error {
    let gives = rows.iter().find(|row| !row.table.is_empty() && row.table == table);
    match gives {
        Some(row) if refused.kind == kernel::ErrorKind::NotFound => Error::not_found(format!(
            "{}. It is given them by {}",
            refused.message,
            setups::by_hand(row, project, brand::CLI)
        )),
        _ => refused,
    }
}

/// What a project still lacks of what `blueprint`, just applied for it,
/// cannot do its work without, each said with why and with the command
/// that gives it: a fleet, and no board.
///
/// Where a project is added the fleet is left out for the lack of a board
/// and that is said. Applied by hand it was not said: the fleet came up,
/// its channel told whoever read it to add a card to the board, and there
/// was none. It is not refused. A fleet that is up before its board works
/// the board once that is there, and applying is the one way either comes
/// to be, in whichever order somebody takes them: so it is said, as the
/// project's own file passed over is said, and the apply stands.
pub async fn lacks(engine: &Engine, studio: &Studio, blueprint: &str, project: Option<&str>) -> Vec<String> {
    let rows = the_usual(engine).await.unwrap_or_default();
    let (Some(id), Some(applied)) = (project, rows.iter().find(|row| row.blueprint == blueprint)) else {
        return Vec::new();
    };
    let mut said = Vec::new();
    for need in applied.needs.iter().filter_map(|need| rows.iter().find(|row| &row.name == need)) {
        if !crate::given::has(engine, studio, id, need).await {
            let why = if applied.why.is_empty() {
                format!("{} needs it", applied.title)
            } else {
                applied.why.clone()
            };
            said.push(format!(
                "`{id}` has no {} yet, and {why}: until it has, nothing reaches what was just applied. It is given by {}",
                need.name,
                setups::by_hand(need, id, brand::CLI)
            ));
        }
    }
    said
}

/// The rows as they are handed out: to the client, which draws a switch
/// for each where it says it is offered, and names no blueprint itself.
pub fn listed(rows: &[Row]) -> Value {
    json!(rows.iter().map(Row::value).collect::<Vec<_>>())
}

/// What each of `rows`' blueprint may be handed as a project is set up,
/// for whoever sets it up there: its params as its text says them (what
/// each is, what it is unless said, the choices it has), less what it is
/// told of the project. The fleet is a text of the project's own, which
/// is not there before the project is: it is read from what a project's
/// is written from. One whose text cannot be read is handed nothing to
/// set.
pub fn settings(engine: &Engine, rows: &[Row]) -> Value {
    let library = engine.library();
    let one = |row: &Row| {
        let text = match row.blueprint.as_str() {
            setups::FLEET => Some(studio::projects::templates::fleet(setups::FLEET, setups::FLEET)),
            blueprint => library.load(blueprint, None),
        };
        let params: Vec<Value> = text
            .and_then(|text| blueprint::check(&text, &blueprint::NoCatalog).blueprint)
            .map(|read| blueprint::graph(&read).params)
            .unwrap_or_default()
            .into_iter()
            .filter(|param| !row.told.contains(&param.name))
            .map(|param| json!(param))
            .collect();
        json!({ "name": row.name, "blueprint": row.blueprint, "params": params })
    };
    json!({ "starts": rows.iter().map(one).collect::<Vec<_>>() })
}

/// What a request to add a project asks for it to start with. Nothing
/// said (or `false`) is nothing asked, which is not the same as a list
/// with nothing in it: the first is a project as it always was, the
/// second one whose every row was turned down. Asked as `true`, the rows
/// of the factory's default, given where they are on; as a list, those
/// it names of them on and the rest off, and rows of its own; as
/// `{ "setup": <name> }`, the rows of that setup, or with `rows` those it
/// names of that setup. Every row is handed on, so that what one that is
/// given needs is said by the row that gives it.
pub async fn asked(engine: &Engine, body: &Value) -> Result<Option<Vec<Row>>> {
    match body.get("set_up") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => Ok(None),
        Some(Value::Bool(true)) => Ok(Some(the_usual(engine).await?)),
        Some(Value::Array(asked)) => {
            Ok(Some(setups::chosen(&the_usual(engine).await?, "set_up", asked).map_err(Error::invalid)?))
        }
        Some(Value::Object(said)) if said.get("setup").is_some_and(Value::is_string) => {
            let name = said.get("setup").and_then(Value::as_str).unwrap_or(DEFAULT);
            let rows = of_setup(engine, name).await?;
            match said.get("rows") {
                None | Some(Value::Null) => Ok(Some(rows)),
                Some(Value::Array(asked)) => {
                    Ok(Some(setups::chosen(&rows, "rows", asked).map_err(Error::invalid)?))
                }
                Some(other) => Err(Error::invalid(format!(
                    "`rows` of `set_up` is a list of what the project is to start with of that setup, and not {other}"
                ))),
            }
        }
        Some(_) => {
            let known: Vec<String> =
                the_usual(engine).await?.iter().map(|row| format!("`{}`", row.name)).collect();
            Err(Error::invalid(format!(
                "`set_up` is `true`, a list of what the project is to start with ({}): each by its name, as its \
                 name with what its blueprint is handed (`{{ \"name\": \"board\", \"with\": {{ \"columns\": [...] }} }}`) \
                 or a row of its own (`{{ \"name\": \"notes\", \"blueprint\": \"std/docs\" }}`); or a setup the \
                 factory keeps, by its name (`{{ \"setup\": \"research\" }}`)",
                known.join(", ")
            )))
        }
    }
}

/// What a request asks a project that is there already to be given
/// (`names`): a list of the factory's default's rows by their names, each
/// with what its blueprint is handed where it says, and rows of its own.
pub async fn asked_later(engine: &Engine, body: &Value) -> Result<Vec<Row>> {
    let rows = the_usual(engine).await?;
    match body.get("names") {
        Some(Value::Array(names)) => setups::chosen(&rows, "names", names).map_err(Error::invalid),
        _ => {
            let known: Vec<String> = rows.iter().map(|row| format!("`{}`", row.name)).collect();
            Err(Error::invalid(format!(
                "`names` is needed: a list of what the project is to be given ({})",
                known.join(", ")
            )))
        }
    }
}

/// Whether a project made for what was `asked` is given its first
/// documents: where a library was asked for, and where nothing was said,
/// which is a project as it always was.
pub fn documents(asked: Option<&[Row]>) -> bool {
    asked.is_none_or(|asked| asked.iter().any(|row| row.on && row.blueprint == "std/docs"))
}

/// Gives `project` what was `asked` as `who`, and says what it was given
/// (by the name of each: its instance) and what it was not, a sentence
/// for each (`left`).
pub async fn give(engine: &Engine, project: &Project, who: &Principal, asked: &[Row]) -> Value {
    set_up(engine, project, who, asked, false).await
}

/// Gives a project that is there already what was `asked`, as `who`: the
/// same asking as when one is added, of the same blueprint, with what the
/// project has by now counted as had. What it has already is not applied
/// again and is said to be there (`there`, by name, each with the
/// instance that is it); what something needs may be what the project had
/// before as well as what it is given now.
pub async fn give_later(engine: &Engine, project: &Project, who: &Principal, asked: &[Row]) -> Result<Value> {
    // Asked as itself, this is applying: refused whole, in the words
    // every route says it in, where adding a project went on without.
    who.require("blueprints:run")?;
    Ok(set_up(engine, project, who, asked, true).await)
}

/// Applies `std/project` for `project` as `who` with `rows`, and says
/// what came of each as its run gave it out: by the name of each that was
/// given, its instance; what it had already (`there`); and what was left
/// out (`left`), each said only where there is any.
async fn set_up(engine: &Engine, project: &Project, who: &Principal, rows: &[Row], later: bool) -> Value {
    let id = &project.id;
    let on: Vec<Row> = rows.iter().filter(|row| row.on).cloned().collect();
    if on.is_empty() {
        return json!({});
    }
    // Applying is asked of whoever added it: adding a project is not a
    // way to have something applied that they could not apply themselves.
    if !who.can("blueprints:run") {
        return json!({ "left": [setups::refused(&on, &who.name, id, brand::CLI)] });
    }
    // Every row, those not asked for off: what one that is asked for needs is said by its row.
    let rows: Vec<Value> = rows.iter().map(Row::value).collect();
    let params = json!({ "rows": rows, "later": later });
    let request = Apply {
        blueprint: PROJECT.to_string(),
        source: None,
        params: params.as_object().cloned().unwrap_or_default(),
        key: None,
        parent: None,
        target: None,
        project: Some(id.clone()),
        principal: who.clone(),
        // Asked again with the same rows, it is asked again: what is there is said to be.
        force: true,
        take: false,
        labels: Default::default(),
        // Under no environment, as what it applies is (`given::apply`).
        environment: Some(String::new()),
    };
    let ran = async {
        let applied = engine.apply(request).await?;
        let Some(run) = applied.run else { return Ok(Value::Null) };
        let finished = engine.wait_run(&run).await?;
        match finished.status {
            RunStatus::Ok => Ok(finished.outputs),
            _ => Err(Error::internal(
                finished.error.unwrap_or_else(|| format!("setting it up by `{PROJECT}` did not finish well")),
            )),
        }
    };
    let outputs = match ran.await {
        Ok(outputs) => outputs,
        Err(why) => {
            return json!({ "left": [format!("`{id}` was not set up: {}. By hand: `{} apply {PROJECT} --project {id}`", why.message, brand::CLI)] });
        }
    };
    let mut said = outputs["given"].as_object().cloned().unwrap_or_default();
    for word in ["there", "left"] {
        if outputs[word].as_array().is_some_and(|items| !items.is_empty()) {
            said.insert(word.to_string(), outputs[word].clone());
        }
    }
    Value::Object(said)
}
