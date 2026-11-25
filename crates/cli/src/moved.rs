//! `apart`: has the projects of a stopped factory that an earlier
//! program made moved to journals of their own. Asked of nobody's
//! daemon: the factory has to be stopped, and what is done is done to
//! its files (`api::moved`).

use std::process::ExitCode;

use api::moved::{Counted, Looked, Moved, Project};
use serde_json::json;

use crate::args::Args;

fn fail(message: &str, code: u8) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(code)
}

/// What a project holds, by table, each by what its rows are and
/// leaving out what the database keeps by itself, and then what is kept
/// beside those rows, by what it is: `3 instances, 41 runs, 12 records,
/// 30 lines of their history; and beside them 1 texts they are held to`.
fn rows(counted: &[Counted]) -> String {
    let some = |all: Vec<(String, u64)>| all.into_iter().filter(|(_, rows)| *rows > 0);
    let tables: Vec<String> = some(api::moved::went(counted, false))
        .map(|(table, rows)| format!("{rows} {}", api::moved::rows_of(&table)))
        .collect();
    let beside: Vec<String> = some(api::moved::went(counted, true))
        .map(|(name, kept)| format!("{kept} {}", api::moved::said_of(&name)))
        .collect();
    match (tables.is_empty(), beside.is_empty()) {
        (true, true) => "nothing".to_string(),
        (false, true) => tables.join(", "),
        (true, false) => format!("beside its rows, {}", beside.join(", ")),
        (false, false) => format!("{}; and beside them {}", tables.join(", "), beside.join(", ")),
    }
}

/// How much of each table, what the database keeps by itself among
/// them, or under each name of what is kept beside the rows.
fn by_name(counted: &[Counted], beside: bool) -> std::collections::BTreeMap<String, u64> {
    let meant = counted.iter().filter(|counted| counted.beside == beside);
    meant.map(|counted| (counted.table.clone(), counted.moves)).collect()
}

/// What there is, project by project: said before anything is done, so
/// that whoever asked can still stop it.
fn say(asked: &Looked) {
    let cli = brand::CLI;
    for project in &asked.projects {
        let file = project.file.as_deref().map(|file| format!("journals/{file}")).unwrap_or_default();
        let (id, keeps) = (&project.id, api::moved::keeping(project.keeps));
        if project.apart() {
            println!("`{id}` is kept in a journal of its own already ({file}: {}).", rows(&project.kept));
        } else if let (true, Some(keeps), 0) = (project.in_the_way.is_empty(), keeps, project.held()) {
            // A journal an earlier program made, with nothing of the
            // project left behind it: only what it keeps is said anew.
            println!(
                "`{id}` has a journal of its own that keeps {keeps} ({file}), and the factory's own holds \
                 nothing more of it: its journal would be made to keep the rest of it from now on."
            );
        } else if let (true, Some(keeps)) = (project.in_the_way.is_empty(), keeps) {
            println!(
                "`{id}` has a journal of its own that keeps {keeps} ({file}), and the rest of it would be \
                 moved there: {}.",
                rows(&project.rows)
            );
        } else if project.in_the_way.is_empty() {
            println!(
                "`{id}` would be moved to a journal of its own, with what is kept of it: {}.",
                rows(&project.rows)
            );
        } else {
            println!(
                "`{}` is left where it is, since something of it is under way: {}. Start the factory and let \
                 that finish, or stop it (`{cli} stop <instance>`, `{cli} agent stop <name>`); then stop the \
                 factory and ask again.",
                project.id,
                project.in_the_way.join("; ")
            );
        }
    }
    if asked.projects.is_empty() {
        println!("This factory has no project.");
    }
    if asked.projects.iter().any(Project::moves) {
        let free = match asked.free {
            Some(free) => format!("{} MB are free", free / (1024 * 1024)),
            None => "how much is free could not be told".to_string(),
        };
        println!(
            "It takes about {} s, and up to {} MB of room beside the database while it is done ({free}). What was said of a \
             project goes with it; the database is not made smaller by this (`{cli} repack` does \
             that). Cut off, a project is as it was or is moved, and never between.",
            asked.seconds(),
            asked.needs().div_ceil(1024 * 1024)
        );
    }
}

fn told(asked: &Looked) -> serde_json::Value {
    let projects: Vec<_> = asked
        .projects
        .iter()
        .map(|project| {
            json!({
                "project": project.id, "apart": project.apart(), "moves": project.moves(),
                "in_the_way": project.in_the_way, "keeps": project.keeps, "file": project.file,
                "rows": by_name(&project.rows, false), "beside": by_name(&project.rows, true),
            })
        })
        .collect();
    json!({
        "database": asked.database, "bytes": asked.bytes, "free": asked.free, "needs": asked.needs(),
        "seconds": asked.seconds(),
        "projects": projects,
    })
}

fn done(moved: &Moved) -> serde_json::Value {
    json!({
        "project": moved.project, "journal": moved.journal, "file": moved.file,
        "rows": by_name(&moved.rows, false), "beside": by_name(&moved.rows, true),
        "at": moved.at, "took_ms": moved.took_ms,
    })
}

pub async fn apart(args: &Args) -> ExitCode {
    let cli = brand::CLI;
    let data = match crate::serve::place(args) {
        Ok((_, data)) => data,
        Err(message) => return fail(&message, 2),
    };
    let (json, only) = (args.has("json"), args.word(1));
    let look = args.has("look");
    // A flag nobody reads is refused before anything is done (`main` says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    if look {
        return match api::moved::look(&data, only) {
            Ok(asked) if json => {
                println!("{}", told(&asked));
                ExitCode::SUCCESS
            }
            Ok(asked) => {
                say(&asked);
                match (asked.projects.iter().any(Project::moves), asked.refused()) {
                    (false, _) => println!("There is nothing to move."),
                    (true, Some(why)) => println!("It would not be begun: {why}."),
                    (true, None) => println!("Nothing was changed. `{cli} apart` does it."),
                }
                ExitCode::SUCCESS
            }
            Err(err) => fail(&err.message, 1),
        };
    }
    let mut looked = None;
    let moved = api::moved::run(&data, only, None, |asked| {
        if !json {
            say(asked);
        }
        looked = Some((told(asked), asked.projects.iter().any(|project| !project.in_the_way.is_empty())));
    })
    .await;
    let (looked, left) = looked.unwrap_or_default();
    match moved {
        Ok(moved) if json => {
            let mut said = looked;
            said["moved"] = json!(moved.iter().map(done).collect::<Vec<_>>());
            println!("{said}");
            ExitCode::SUCCESS
        }
        Ok(moved) => {
            for one in &moved {
                let (id, file, took) = (&one.project, &one.file, one.took_ms as f64 / 1000.0);
                match one.rows.iter().all(|counted| counted.moves == 0) {
                    // Its journal kept less than one made now, and the
                    // factory's own held nothing of the rest.
                    true => println!(
                        "`{id}` is kept whole in journals/{file} from now on: the factory's own held nothing \
                         more of it."
                    ),
                    false => println!(
                        "`{id}` was moved in {took:.1} s: it is kept in journals/{file} now ({}).",
                        rows(&one.rows)
                    ),
                }
            }
            match (moved.is_empty(), left) {
                (true, false) => println!("There was nothing to move."),
                (true, true) => println!("Nothing was moved."),
                (false, _) => println!("Start the factory again: `{cli} up`."),
            }
            ExitCode::SUCCESS
        }
        Err(err) => fail(&err.message, 1),
    }
}
