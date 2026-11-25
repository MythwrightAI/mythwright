//! `store`: the store program, which keeps a table's records in a database
//! file and serves them.
//!
//! This is not a command for talking to the daemon. It is a program that a
//! blueprint runs and hosts (`std/store`), the way it would host anything
//! else, so that a table can be kept somewhere a blueprint made. It is in
//! this binary so that there is nothing more to install: a blueprint finds
//! it where the daemon that runs the blueprint is.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use api::store::StoreServer;

use crate::args::Args;
use crate::serve::Asked;

fn fail(message: &str, code: u8) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(code)
}

pub async fn store(args: &Args) -> ExitCode {
    let cli = brand::CLI;
    let usage = format!("`{cli} store init --db FILE` and `{cli} store serve --db FILE [--listen ADDRESS]`");
    let Some(file) = args.value("db").filter(|file| !file.is_empty()).map(PathBuf::from) else {
        return fail(&format!("which database file? It is given as `--db`: {usage}"), 2);
    };
    match args.word(1) {
        Some("init") if args.heed().check().is_err() => ExitCode::from(2),
        Some("init") => match api::store::create(&file).await {
            Ok(0) => {
                println!("{} is ready, with nothing in it", file.display());
                ExitCode::SUCCESS
            }
            Ok(records) => {
                println!("{} is ready, with the {records} record(s) it had", file.display());
                ExitCode::SUCCESS
            }
            Err(err) => fail(&err.message, 1),
        },
        Some("serve") => {
            // Hosted as a service it is told its port the way every
            // service is, in `PORT`, and need not be told twice: a command
            // with no shell under it has nothing to write `$PORT` into.
            let hosted = std::env::var("PORT").ok().filter(|port| !port.is_empty());
            let listen = match (args.value("listen"), hosted) {
                (Some(listen), _) => listen.to_string(),
                (None, Some(port)) => format!("127.0.0.1:{port}"),
                (None, None) => "127.0.0.1:0".to_string(),
            };
            if args.heed().check().is_err() {
                return ExitCode::from(2);
            }
            let Ok(listen) = listen.parse::<SocketAddr>() else {
                return fail(&format!("`{listen}` is not an address like 127.0.0.1:7801"), 2);
            };
            // Before it serves: whatever hosts it may ask it to stop as
            // soon as it answers.
            let mut asked = Asked::listening();
            let server = match StoreServer::start(&file, listen).await {
                Ok(server) => server,
                Err(err) => return fail(&err.message, 1),
            };
            println!("keeping the records in {} at {}", file.display(), server.url());
            asked.heard().await;
            server.stop().await;
            ExitCode::SUCCESS
        }
        Some(other) => fail(&format!("`store {other}` is not a command; there are {usage}"), 2),
        None => fail(&format!("`store` makes a database or serves one: {usage}"), 2),
    }
}
