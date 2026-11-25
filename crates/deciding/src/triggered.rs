//! When a row of a project's `triggers` fires: which pipelines run for
//! something that happened, under which key and handed what; which rows
//! go by the clock and how; and what whoever runs one by hand has to hold.
//!
//! A row says the pipeline it runs (`pipeline`, a blueprint of the
//! project's), what runs it (`on`: `hand`, `branch`, `column`, `every`,
//! `at`) with what that kind goes by (`branch`, `column`, `every`, `at`),
//! the environment its runs are under (`environment`; none where it says
//! none, which the environment's own rules then hold), the key its runs
//! are under (`key`; the pipeline's name, and the environment's after it,
//! where it says none) and whether it is `paused`. What a run is handed is what the
//! thing that happened says, and only what the pipeline declares that it
//! takes: a branch and its commit, the card that came to a column, and
//! what whoever runs it by hand says. `std/pipeline` asks these and does
//! what it is told; nothing here starts anything.

use blueprint::{ActionSpec, ValueType};
use conductor::{ActionError, Registry};
use serde_json::{Map, Value, json};

use crate::add;

/// What a row says, as `rows.read` hands it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub id: String,
    pub pipeline: String,
    pub on: String,
    pub branch: String,
    pub column: String,
    pub every: String,
    pub at: String,
    /// The project's environment its runs are under; none where empty.
    pub environment: String,
    pub key: String,
    pub paused: bool,
    /// What more its pipeline is handed, each `name=value`.
    pub with: Vec<String>,
    /// Who last wrote `environment` (`environment_by`, which the factory
    /// writes: `writer_of`); nothing where it was written before that was kept.
    pub writer: String,
}

impl Row {
    /// A row as `rows.read` hands it: a mapping of its cells.
    pub fn of(row: &Value) -> Row {
        let cell =
            |name: &str| row.get(name).map(blueprint::expr::display).unwrap_or_default().trim().to_string();
        Row {
            id: cell("id"),
            pipeline: cell("pipeline"),
            on: cell("on"),
            branch: cell("branch"),
            column: cell("column"),
            every: cell("every"),
            at: cell("at"),
            environment: cell("environment"),
            key: cell("key"),
            paused: row.get("paused").and_then(Value::as_bool).unwrap_or(false),
            writer: cell("environment_by"),
            with: row["with"]
                .as_array()
                .into_iter()
                .flatten()
                .map(blueprint::expr::display)
                .filter(|one| !one.trim().is_empty())
                .collect(),
        }
    }

    /// The key its runs are under: what it says, or its pipeline's name
    /// with each `/` a `-`, and the environment it is under after that: an
    /// instance is under one environment for its life, so runs under two
    /// are two keys.
    pub fn key(&self) -> String {
        match (self.key.as_str(), self.environment.as_str()) {
            ("", "") => self.pipeline.replace('/', "-"),
            ("", under) => format!("{}-{under}", self.pipeline.replace('/', "-")),
            (key, _) => key.to_string(),
        }
    }
}

/// What happened, that rows may fire for.
#[derive(Clone, Debug, PartialEq)]
pub enum Happened {
    /// Somebody asked for a pipeline to run, handing it what it takes.
    Hand { pipeline: String, with: Map<String, Value> },
    /// A branch moved to a commit.
    Branch { branch: String, commit: String },
    /// A card came to a column.
    Column { column: String, item: String },
    /// The time a row goes by came (`every` or `at`): the row, by its id.
    Due { row: String },
}

/// One run that is due: the row that fired, the pipeline, the key it is
/// under and what it is handed, before that is held to what it takes.
#[derive(Clone, Debug, PartialEq)]
pub struct Fired {
    pub row: String,
    pub pipeline: String,
    pub key: String,
    /// The environment it is run under, as its row says; none where empty.
    pub environment: String,
    pub with: Map<String, Value>,
    /// What its row says it is handed beside that, each `name=value`.
    pub said: Vec<String>,
    /// Who last wrote the row's environment ([`Row::writer`]).
    pub writer: String,
}

/// The rows that fire for what happened, each once, in the order they
/// were read. A row that is paused, or says no pipeline, fires for
/// nothing; asked by hand for a pipeline no row runs by hand is refused,
/// in a sentence that says what to add.
pub fn fired(rows: &[Row], happened: &Happened) -> Result<Vec<Fired>, String> {
    let fires = |row: &&Row| {
        !row.paused
            && !row.pipeline.is_empty()
            && match happened {
                Happened::Hand { pipeline, .. } => row.on == "hand" && row.pipeline == *pipeline,
                Happened::Branch { branch, .. } => {
                    row.on == "branch" && (row.branch.is_empty() || row.branch == *branch)
                }
                Happened::Column { column, .. } => row.on == "column" && row.column == *column,
                Happened::Due { row: id } => matches!(row.on.as_str(), "every" | "at") && row.id == *id,
            }
    };
    let handed = match happened {
        Happened::Hand { with, .. } => with.clone(),
        Happened::Branch { branch, commit } => {
            Map::from_iter([("branch".to_string(), json!(branch)), ("commit".to_string(), json!(commit))])
        }
        Happened::Column { column, item } => {
            Map::from_iter([("item".to_string(), json!(item)), ("column".to_string(), json!(column))])
        }
        Happened::Due { .. } => Map::new(),
    };
    let found: Vec<Fired> = rows
        .iter()
        .filter(fires)
        .map(|row| Fired {
            row: row.id.clone(),
            pipeline: row.pipeline.clone(),
            key: row.key(),
            environment: row.environment.clone(),
            with: handed.clone(),
            said: row.with.clone(),
            writer: row.writer.clone(),
        })
        .collect();
    if let (Happened::Hand { pipeline, .. }, true) = (happened, found.is_empty()) {
        return Err(format!(
            "no row of the project's triggers runs `{pipeline}` by hand: add one that says `pipeline: {pipeline}` and \
             `on: hand`, or take `paused` off the one there is"
        ));
    }
    Ok(found)
}

/// What a run is handed: what its row says in `with` (`name=value`,
/// each as the kind of thing the pipeline's param of that name is), and
/// what happened over that, of which only what the pipeline declares that
/// it takes. Asked by hand, a name it does not take is refused; what a
/// branch or a column hands is left out where it is not taken, since the
/// pipeline need not care. And a name the row says that the pipeline
/// does not take is refused, in a sentence.
pub fn fit(
    said: &[String],
    with: &Map<String, Value>,
    takes: &[(String, ValueType)],
    by_hand: bool,
    pipeline: &str,
) -> Result<Map<String, Value>, String> {
    let kind = |name: &str| takes.iter().find(|(taken, _)| taken == name).map(|(_, kind)| *kind);
    let names = || match takes {
        [] => "none".to_string(),
        _ => takes.iter().map(|(taken, _)| format!("`{taken}`")).collect::<Vec<_>>().join(", "),
    };
    let mut fitted = Map::new();
    for entry in said {
        let Some((name, value)) = entry.split_once('=') else {
            return Err(format!(
                "the row of `{pipeline}` says `{entry}` in `with`, where each thing is `name=value`"
            ));
        };
        let name = name.trim();
        let Some(kind) = kind(name) else {
            return Err(format!(
                "the row of `{pipeline}` says `{name}` in `with`, and it takes no param called that: it takes {}",
                names()
            ));
        };
        let title = format!("the row of `{pipeline}`");
        fitted.insert(
            name.to_string(),
            crate::segments::as_kind(name, &Value::from(value.trim()), kind, &title)?,
        );
    }
    for (name, value) in with {
        if kind(name).is_some() {
            fitted.insert(name.clone(), value.clone());
        } else if by_hand {
            return Err(format!("`{pipeline}` takes no param called `{name}`: it takes {}", names()));
        }
    }
    Ok(fitted)
}

/// What whoever runs a pipeline by hand is to hold beside running
/// blueprints (`blueprints:run`): what the pipeline runs with, which is
/// what its text says it may use. One that says nothing runs with all
/// that whoever applied the triggers may use, which nobody is asked of,
/// and is refused in a sentence that says what to write.
pub fn holds(permissions: Option<&[String]>, pipeline: &str) -> Result<Vec<String>, String> {
    match permissions {
        Some(said) => Ok(said.to_vec()),
        None => Err(format!(
            "`{pipeline}` says nothing of what it may use, so it is run by hand by nobody: write its `permissions`, \
             as `permissions: [shell, projects]`, and whoever has those may run it"
        )),
    }
}

/// What whoever asked for `pipeline` by hand is told where it was not
/// run: why (`says`, a sentence), or what they do not hold (`lacks`).
pub fn refusal(pipeline: &str, says: &str, lacks: &[String]) -> String {
    let what = match pipeline {
        "" => "No pipeline was run".to_string(),
        _ => format!("`{pipeline}` was not run"),
    };
    match (says.trim(), lacks) {
        ("", []) => format!("{what}: what may run it could not be asked"),
        ("", [one]) => format!("{what}: it takes `{one}`, which you do not hold"),
        ("", many) => {
            let (last, rest) = many.split_last().expect("not empty");
            let rest: Vec<String> = rest.iter().map(|one| format!("`{one}`")).collect();
            format!("{what}: it takes {} and `{last}`, which you do not hold", rest.join(", "))
        }
        (says, _) => format!("{what}: {says}"),
    }
}

/// Which rows go by the clock, and whether any goes by a branch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Timed {
    /// Each row that fires so long after it last did: its id, and how long.
    pub every: Vec<(String, String)>,
    /// Each row that fires at a time of day: its id, and the time with its zone.
    pub at: Vec<(String, String)>,
    /// Whether any row fires as a branch moves: the project's branches
    /// are then followed.
    pub follows: bool,
    /// Each row that says something that cannot be gone by, with why.
    pub wrong: Vec<(String, String)>,
}

/// What the rows that go by the clock and by a branch come to. A row that
/// is paused goes by nothing.
pub fn timed(rows: &[Row]) -> Timed {
    let mut timed = Timed::default();
    for row in rows.iter().filter(|row| !row.paused && !row.pipeline.is_empty()) {
        match row.on.as_str() {
            "every" => match blueprint::duration::parse(&row.every) {
                Ok(ms) if ms >= MOST_OFTEN_MS => timed.every.push((row.id.clone(), row.every.clone())),
                Ok(_) => timed.wrong.push((
                    row.id.clone(),
                    format!(
                        "`every: {}` is more often than once a minute, which a pipeline never runs",
                        row.every
                    ),
                )),
                Err(why) => timed.wrong.push((
                    row.id.clone(),
                    format!("`every` says how long after it last ran it runs again, as `6h`: {why}"),
                )),
            },
            "at" => match blueprint::at::read(&row.at) {
                Ok(_) => timed.at.push((row.id.clone(), row.at.clone())),
                Err(why) => timed.wrong.push((row.id.clone(), why)),
            },
            "branch" => timed.follows = true,
            _ => {}
        }
    }
    timed
}

/// The least time between two runs that a row of `every` says.
pub const MOST_OFTEN_MS: u64 = 60_000;

/// The rows `rows.read` handed, as rows.
fn rows_of(args: &kernel::Fields) -> Vec<Row> {
    args.get("rows").and_then(Value::as_array).into_iter().flatten().map(Row::of).collect()
}

fn text(args: &kernel::Fields, name: &str) -> String {
    args.get(name).map(blueprint::expr::display).unwrap_or_default()
}

/// The row a step names: by its id, or by the key of what goes by the
/// clock for it (`every-<id>`, `at-<id>`).
fn row_of(args: &kernel::Fields) -> String {
    let (row, key) = (text(args, "row"), text(args, "key"));
    match row.as_str() {
        "" => {
            ["every-", "at-"].iter().find_map(|kind| key.strip_prefix(kind)).unwrap_or_default().to_string()
        }
        _ => row,
    }
}

/// What happened, as a step hands it.
fn happened(args: &kernel::Fields) -> Result<Happened, String> {
    Ok(match text(args, "on").as_str() {
        "hand" => Happened::Hand {
            pipeline: text(args, "pipeline"),
            with: args.get("with").and_then(Value::as_object).cloned().unwrap_or_default(),
        },
        "branch" => Happened::Branch { branch: text(args, "branch"), commit: text(args, "commit") },
        "column" => Happened::Column { column: text(args, "column"), item: text(args, "item") },
        "due" => Happened::Due { row: row_of(args) },
        other => {
            return Err(format!(
                "`on` is `hand`, `branch`, `column` or `due`, and what was said is `{other}`"
            ));
        }
    })
}

/// Adds the blocks of a project's triggers.
pub fn register(registry: &mut Registry) {
    use ValueType as T;
    add(
        registry,
        ActionSpec::new(
            "triggers.fired",
            "Say which rows of a project's triggers fire for something that happened, each with the pipeline it runs, the key that run is under and what it is handed of what happened, which is what the pipeline takes.",
        )
        .deciding()
        .arg("rows", T::List, "the rows of the table, as `rows.read` gives them")
        .required("on", T::String, "what happened: `hand`, somebody asked for `pipeline` with `with`; `branch`, `branch` moved to `commit`; `column`, `item` came to `column`; `due`, the time `row` goes by came")
        .arg("pipeline", T::String, "with `hand`, the pipeline asked for")
        .arg("with", T::Object, "with `hand`, what it is to be handed")
        .arg("by", T::String, "with `hand`, who asked, as an actor")
        .arg("branch", T::String, "with `branch`, the branch")
        .arg("commit", T::String, "with `branch`, the commit it moved to")
        .arg("column", T::String, "with `column`, the column")
        .arg("item", T::String, "with `column`, the card that came there")
        .arg("row", T::String, "with `due`, the row whose time came")
        .arg("key", T::String, "with `due` and no `row`, what goes by the clock for the row was called (`every-<id>`)")
        .output("fires", "each run that is due: `row`, `pipeline`, `key`, `environment` (the one it is run under; empty for none), `with`, what it is handed, and `asks`, whether one of those the environment's rule names lets it go on first: unless whoever applied the triggers, whoever last wrote the row's environment and whoever asked by hand are each one of them")
        .output("holds", "with `hand`, what whoever asked is to hold beside running blueprints: what the pipeline runs with")
        .output("refused", "each row that fires and cannot be run, with why (`row`, `says`): its pipeline is not there to be run here, or does not take what is handed")
        .output("says", "with `hand`, why nothing runs, where nothing does; empty otherwise")
        .output("due", "with `due`, the row whose time came, by its id; empty where the key names none")
        .idempotent(),
        |cx, args| async move {
            let happened = happened(&args).map_err(ActionError::new)?;
            let by_hand = matches!(happened, Happened::Hand { .. });
            let due = match &happened {
                Happened::Due { row } => row.clone(),
                _ => String::new(),
            };
            let found = match fired(&rows_of(&args), &happened) {
                Ok(found) => found,
                Err(says) => return Ok(json!({ "fires": [], "holds": [], "refused": [], "says": says, "due": due })),
            };
            let (mut fires, mut refused, mut held) = (Vec::new(), Vec::new(), Vec::new());
            let (applier, asker) = (cx.principal().actor(), text(&args, "by"));
            for one in found {
                // Read as whoever asks may run it: a project's own as it
                // was vouched for, and never one nobody vouched for.
                let text = cx.engine().text_within(cx.instance(), cx.principal(), &one.pipeline, cx.project()).await;
                let read = text.and_then(|text| blueprint::check(&text, &blueprint::NoCatalog).blueprint);
                let Some(read) = read else {
                    refused.push(json!({ "row": one.row, "says": format!(
                        "there is no pipeline called `{}` that may be run here: one of the project's own runs as it was saved through the factory",
                        one.pipeline
                    ) }));
                    continue;
                };
                let takes: Vec<(String, ValueType)> =
                    read.params.iter().map(|param| (param.name.clone(), param.ty)).collect();
                let fitted = fit(&one.said, &one.with, &takes, by_hand, &one.pipeline);
                let asked = match by_hand {
                    true => holds(read.permissions.as_deref(), &one.pipeline),
                    false => Ok(Vec::new()),
                };
                match fitted.and_then(|with| asked.map(|asked| (with, asked))) {
                    Ok((with, asked)) => {
                        held = asked;
                        // Whoever it is begun for is each one of those the environment's rule names, or one of them lets it go on first.
                        let rule = match one.environment.as_str() {
                            "" => Vec::new(),
                            under => cx.engine().environment_of(cx.project().unwrap_or_default(), under).await.ok().flatten().map(|held| held.rules.approve).unwrap_or_default(),
                        };
                        let mut whom = vec![applier.as_str(), one.writer.as_str()];
                        whom.extend(by_hand.then_some(asker.as_str()));
                        let asks = kernel::go::first(&rule, &whom);
                        fires.push(json!({ "row": one.row, "pipeline": one.pipeline, "key": one.key, "environment": one.environment, "with": with, "asks": asks }));
                    }
                    Err(says) if by_hand => {
                        return Ok(json!({ "fires": [], "holds": [], "refused": [], "says": says, "due": due }));
                    }
                    Err(says) => refused.push(json!({ "row": one.row, "says": says })),
                }
            }
            Ok(json!({ "fires": fires, "holds": held, "refused": refused, "says": "", "due": due }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "triggers.refused",
            "Say in a sentence why a pipeline asked for by hand was not run: why, or what whoever asked does not hold.",
        )
        .deciding()
        .arg("pipeline", T::String, "the pipeline asked for")
        .arg("says", T::String, "why, where it was refused for something said of it")
        .arg("lacks", T::List, "what whoever asked does not hold, where it was refused for that")
        .output("says", "the sentence")
        .idempotent(),
        |_, args| async move {
            let lacks: Vec<String> =
                args.get("lacks").and_then(Value::as_array).into_iter().flatten().map(blueprint::expr::display).collect();
            Ok(json!({ "says": refusal(&text(&args, "pipeline"), &text(&args, "says"), &lacks) }))
        },
    );
    add(
        registry,
        ActionSpec::new(
            "triggers.timed",
            "Say which rows of a project's triggers go by the clock, and how, and whether any goes by a branch moving; and of one row whether it goes by `every` or `at` now.",
        )
        .deciding()
        .arg("rows", T::List, "the rows of the table, as `rows.read` gives them")
        .arg("row", T::String, "a row that was just added, changed or taken away, by its id")
        .arg("key", T::String, "with no `row`, what goes by the clock for the row was called (`every-<id>`)")
        .output("every", "each row that runs so long after it last did: `id` and `after`, how long")
        .output("at", "each row that runs at a time of day: `id` and `at`, the time with its zone")
        .output("follows", "whether any row runs as a branch moves")
        .output("wrong", "each row that says what cannot be gone by: `id` and `says`, why")
        .output("row_every", "whether `row` runs so long after it last did")
        .output("row_at", "whether `row` runs at a time of day")
        .output("row_after", "how long after it last ran `row` runs again; empty where it does not go so")
        .idempotent(),
        |_, args| async move {
            let timed = timed(&rows_of(&args));
            let row = row_of(&args);
            let pairs = |list: &[(String, String)], name: &str| -> Vec<Value> {
                list.iter().map(|(id, said)| json!({ "id": id, name: said })).collect()
            };
            Ok(json!({
                "every": pairs(&timed.every, "after"),
                "at": pairs(&timed.at, "at"),
                "follows": timed.follows,
                "wrong": pairs(&timed.wrong, "says"),
                "row_every": timed.every.iter().any(|(id, _)| *id == row),
                "row_at": timed.at.iter().any(|(id, _)| *id == row),
                "row_after": timed.every.iter().find(|(id, _)| *id == row).map(|(_, after)| after.clone()).unwrap_or_default(),
            }))
        },
    );
}
