//! `repack`: has a stopped factory's database written out again as one is
//! made now. Asked of nobody's daemon: the factory has to be stopped, and
//! what is done is done to its files (`api::repack`).

use std::process::ExitCode;

use api::repack::Looked;
use serde_json::json;

use crate::args::Args;

fn fail(message: &str, code: u8) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(code)
}

fn mb(bytes: u64) -> u64 {
    bytes.div_ceil(1024 * 1024)
}

/// What there is, and what writing it out again takes: said before
/// anything is done, so that whoever asked can still stop it.
fn say(asked: &Looked) {
    let plan = &asked.plan;
    let numbers = if plan.numbered { "the database numbers" } else { "the journal numbers" };
    println!(
        "{} is {} MB, with pages of {} bytes and events {numbers}.",
        asked.database.display(),
        mb(plan.bytes),
        plan.page
    );
    if !plan.wanted() {
        return;
    }
    let free = match asked.free {
        Some(free) => format!("{} MB are free", mb(free)),
        None => "how much is free could not be told".to_string(),
    };
    println!(
        "Writing it out again as one is made now (pages of half the size, and events the journal numbers \
         itself, so that every write costs the disk less) takes about {} s, and {} MB of room beside it \
         while it is done ({free}). Cut off, it leaves the database as it was.",
        plan.seconds(),
        mb(plan.needs())
    );
}

fn told(asked: &Looked) -> serde_json::Value {
    json!({
        "database": asked.database, "bytes": asked.plan.bytes, "page": asked.plan.page,
        "numbered_by_the_database": asked.plan.numbered, "wanted": asked.plan.wanted(),
        "needs": asked.plan.needs(), "seconds": asked.plan.seconds(), "free": asked.free,
    })
}

pub fn repack(args: &Args) -> ExitCode {
    let cli = brand::CLI;
    let data = match crate::serve::place(args) {
        Ok((_, data)) => data,
        Err(message) => return fail(&message, 2),
    };
    let json = args.has("json");
    let look = args.has("look");
    // A flag nobody reads is refused before anything is done (`main` says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    if look {
        return match api::repack::look(&data) {
            Ok(asked) if json => {
                println!("{}", told(&asked));
                ExitCode::SUCCESS
            }
            Ok(asked) => {
                say(&asked);
                match (asked.plan.wanted(), asked.refused()) {
                    (false, _) => println!("It is as a database is made now: there is nothing to do."),
                    (true, Some(why)) => println!("It would not be begun: {why}."),
                    (true, None) => println!("Nothing was changed. `{cli} repack` does it."),
                }
                ExitCode::SUCCESS
            }
            Err(err) => fail(&err.message, 1),
        };
    }
    let mut looked = None;
    let done = api::repack::run(&data, |asked| {
        if !json {
            say(asked);
        }
        looked = Some(told(asked));
    });
    match done {
        Ok(done) if json => {
            let mut said = looked.unwrap_or_default();
            said["done"] = json!(done.as_ref().map(|done| {
                json!({ "before": done.before, "after": done.after, "events": done.events, "took_ms": done.took_ms })
            }));
            println!("{said}");
            ExitCode::SUCCESS
        }
        Ok(Some(done)) => {
            println!(
                "Done in {:.1} s: it is {} MB, with its {} events as they were. Start the factory again: `{cli} up`.",
                done.took_ms as f64 / 1000.0,
                mb(done.after),
                done.events
            );
            ExitCode::SUCCESS
        }
        Ok(None) => {
            println!("It is as a database is made now: there is nothing to do.");
            ExitCode::SUCCESS
        }
        Err(err) => fail(&err.message, 1),
    }
}
