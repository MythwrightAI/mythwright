//! `profiles`: the factories on this device that have a profile, a name
//! such as `prod` or `dev1` (`env` before 0.1.0, which is still taken).
//!
//! Each writes itself into a list as it starts: what it is called, where
//! its data is and where it listens. This reads the list and asks each
//! address whether the factory is there, which is all that is known of one
//! factory from outside it. Nothing is asked as anyone: no credential is
//! in the list, and none is sent.

use std::process::ExitCode;
use std::time::Duration;

use api::registry::{Factory, Registry};
use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{table, url_of};

/// How long a factory is given to say it is there. It is on this device.
const PATIENCE: Duration = Duration::from_secs(2);

/// Whether the factory is answering where it last listened: `Ok` when it
/// is, and otherwise what is there instead.
async fn answering(http: &reqwest::Client, factory: &Factory) -> Result<(), &'static str> {
    let asked = http.get(format!("{}/api/meta", url_of(factory.address))).timeout(PATIENCE).send().await;
    let Ok(response) = asked else { return Err("not running") };
    let meta: Value = response.json().await.unwrap_or(Value::Null);
    // `environment` is what a program before 0.1.0 answers it as.
    if meta["profile"].as_str().or(meta["environment"].as_str()) == Some(&factory.name) {
        Ok(())
    } else {
        // It stopped, and something else has its address since.
        Err("not running (something else answers there)")
    }
}

pub async fn list(args: &Args) -> ExitCode {
    let json = args.has("json");
    // A flag nobody reads is refused before anything is done (`main` says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let factories = match Registry::of_this_user().factories() {
        Ok(factories) => factories,
        Err(err) => {
            eprintln!("error: {}", err.message);
            return ExitCode::FAILURE;
        }
    };
    let http = reqwest::Client::new();
    let mut listed = Vec::new();
    for factory in &factories {
        listed.push((factory, answering(&http, factory).await));
    }
    if json {
        let entries: Vec<Value> = listed
            .iter()
            .map(|(factory, answering)| {
                json!({
                    "name": factory.name,
                    "url": url_of(factory.address),
                    "answering": answering.is_ok(),
                    "data": factory.data,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json!({ "profiles": entries })).unwrap_or_default());
        return ExitCode::SUCCESS;
    }
    if listed.is_empty() {
        println!(
            "no factory on this device has a profile yet; `{} up --profile dev1` starts one called dev1",
            brand::CLI
        );
        return ExitCode::SUCCESS;
    }
    let rows: Vec<Vec<String>> = listed
        .iter()
        .map(|(factory, answering)| {
            vec![
                factory.name.clone(),
                url_of(factory.address),
                answering.map_or_else(|why| why.to_string(), |()| "answering".to_string()),
                format!("data in {}", factory.data.display()),
            ]
        })
        .collect();
    println!("{}", table(&rows));
    ExitCode::SUCCESS
}
