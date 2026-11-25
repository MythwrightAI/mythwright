//! The one binary.
//!
//! It is the daemon (`serve`), the thing that keeps the daemon running
//! (`up`), and the command-line client that people and agents use to act on
//! the factory. One binary means one thing to install and no version skew
//! between the daemon and the tool that talks to it.

mod applied;
mod args;
mod asked;
mod attached;
mod blueprinted;
mod callers;
mod channels;
mod client;
mod collab;
mod compare;
mod created;
mod credential;
mod devices;
mod docs;
mod drained;
mod environs;
mod examples;
mod explain;
mod extensions;
mod external;
mod files;
mod flagged;
mod fmt;
mod followed;
mod follows;
mod gitseen;
mod grouped;
mod inbox;
mod joined;
mod moved;
mod opened;
mod outlined;
mod owned;
mod profiled;
mod profiles;
mod ran;
mod records;
mod removed;
mod repack;
mod sandbox;
mod schema;
mod serve;
mod signins;
mod sought;
mod spent;
mod store;
mod texts;
mod topics;
mod trash;
mod work;
mod workspaces;

use std::process::ExitCode;

use args::Args;
use client::Client;

fn main() -> ExitCode {
    // Started by the daemon as the wrapper of a program it starts
    // (`hosting::wrapped`): asked of nobody, and read before anything
    // else, since what follows the word is another program's command
    // line and none of it is this one's to understand, or to refuse for
    // not being text.
    let mut raw = std::env::args_os().skip(1);
    if raw.next().is_some_and(|word| word == hosting::wrapped::WRAPS) {
        let words: Vec<std::ffi::OsString> = raw.collect();
        return ExitCode::from(hosting::wrapped::wraps(&words).clamp(0, 255) as u8);
    }
    // And so whatever this process starts from here on may be wrapped by
    // this program started again.
    hosting::wrapped::answers();
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => return usage(&message),
    };
    // How many threads the work is done on is said before there are any
    // (`serve::workers`): the runtime is made here and not for us.
    let workers = match serve::workers(&args) {
        Ok(workers) => workers,
        Err(message) => return usage(&message),
    };
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(workers).enable_all().build();
    match runtime {
        Ok(runtime) => runtime.block_on(run(args)),
        Err(err) => {
            eprintln!("error: this program could not start its threads: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the command, and refuses it where it was given a flag it never
/// asked for: before anything was sent, where it sends anything (the
/// client asks first), and otherwise as it ends. Said once, as a mistake
/// in what was typed. A command that failed before it came to read all
/// it takes said why, and is not said to take none of what it did not
/// come to.
async fn run(args: Args) -> ExitCode {
    let done = act(&args).await;
    let heed = args.heed();
    let refused = match heed.refused() {
        Some(refusal) => Some(refusal),
        None if done == ExitCode::SUCCESS => heed.check().err(),
        None => None,
    };
    match refused {
        None => done,
        Some(refusal) => usage(&refusal),
    }
}

async fn act(args: &Args) -> ExitCode {
    // What only prints is refused before it prints, as the rest are.
    let prints = matches!(args.word(0), None | Some("help" | "version"));
    if prints && args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let Some(command) = args.word(0) else {
        println!("{}", manual::overview());
        return ExitCode::SUCCESS;
    };
    match command {
        "serve" => return serve::serve(args).await,
        "up" => return serve::up(args).await,
        // Asked of a factory that is stopped, and done to its files.
        "repack" if !args.has("help") => return repack::repack(args),
        "apart" if !args.has("help") => return moved::apart(args).await,
        // Laying blueprints out is a matter of their text and nobody's daemon.
        "fmt" if !args.has("help") => return fmt::fmt(args),
        // A program of its own, which a blueprint hosts: it talks to no daemon.
        "store" if !args.has("help") => return store::store(args).await,
        // Asked by git, of a step that sends a branch as a key the factory
        // keeps: it talks to no daemon either.
        "credential" if !args.has("help") => return credential::credential(args),
        // What is on this device, whichever of it is running: asked of the
        // list the factories keep, not of a daemon.
        "profiles" | "env" | "environments" if !args.has("help") => return profiles::list(args).await,
        // A program written out as pages, which asks no daemon either.
        "docs" if !args.has("help") => return docs::docs(args).await,
        "help" => {
            return match args.word(1) {
                // Every command, its flags and the topics, as values.
                None if args.has("json") => {
                    println!("{:#}", manual::commands::described());
                    ExitCode::SUCCESS
                }
                None => {
                    println!("{}", manual::overview());
                    ExitCode::SUCCESS
                }
                Some(topic) => match manual::topic(topic) {
                    Some(text) => {
                        println!("{text}");
                        ExitCode::SUCCESS
                    }
                    None => usage(&format!(
                        "there is no help on `{topic}`; `{} help` lists every command",
                        brand::CLI
                    )),
                },
            };
        }
        "version" => {
            // This program, which is not always the one a running daemon
            // is: `status` says that one.
            if args.has("json") {
                let said = serde_json::json!({
                    "program": brand::CLI, "product": brand::DISPLAY, "version": api::place::version(),
                    "reads": api::place::said(),
                });
                println!("{said:#}");
                return ExitCode::SUCCESS;
            }
            // A nightly says which night and which commit it is.
            println!("{} {}", brand::DISPLAY, api::place::version());
            println!("{}", api::place::said());
            return ExitCode::SUCCESS;
        }
        _ => {}
    }
    if args.has("help") {
        println!("{}", manual::topic(command).unwrap_or_else(manual::overview));
        return ExitCode::SUCCESS;
    }

    let mut client = match Client::new(args) {
        Ok(client) => client,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    let outcome = match command {
        "open" => opened::open(&client, args).await,
        "status" => work::status(&client).await,
        "down" => work::down(&client, args).await,
        "register" => opened::register(&mut client, args).await,
        "login" => opened::login(&client, args).await,
        "logout" => work::logout(&client, args).await,
        "whoami" => work::whoami(&client).await,
        "project" | "projects" => work::project(&client, args).await,
        "file" | "files" => files::file(&client, args).await,
        "workspace" | "workspaces" => workspaces::workspace(&client, args).await,
        "fleet" => work::fleet(&client, args).await,
        "check" => work::check(&client, args).await,
        "explain" => explain::explain(&client, args).await,
        "sandbox" => sandbox::sandbox(&client, args).await,
        "graph" => work::graph(&client, args).await,
        "schema" => schema::schema(&client).await,
        "apply" => work::apply(&client, args).await,
        "instances" => work::instances(&client, args).await,
        "instance" => work::instance(&client, args).await,
        "stop" => work::stop(&client, args).await,
        "runs" => ran::runs(&client, args).await,
        "run" => ran::run(&client, args).await,
        "logs" => ran::logs(&client, args).await,
        "trash" => trash::trash(&client, args).await,
        "actions" => work::actions(&client, args).await,
        "channel" | "channels" => channels::channel(&client, args).await,
        "read" => channels::read(&client, args).await,
        "post" => channels::post(&client, args).await,
        "attachment" => attached::attachment(&client, args).await,
        "inbox" => inbox::inbox(&client, args).await,
        "to" => inbox::to(&client, args).await,
        "ask" => asked::ask(&client, args).await,
        "asked" => asked::asked(&client, args).await,
        "approve" => asked::decide(&client, args, true).await,
        "decline" => asked::decide(&client, args, false).await,
        "board" => collab::board(&client, args).await,
        "record" | "records" => records::record(&client, args).await,
        "topic" | "topics" => topics::topic(&client, args).await,
        "follow" => follows::follow(&client, args, true).await,
        "unfollow" => follows::follow(&client, args, false).await,
        "following" => follows::following(&client).await,
        "agent" | "agents" => collab::agent(&client, args).await,
        "search" => sought::search(&client, args).await,
        "events" => collab::events(&client, args).await,
        "emit" => collab::emit(&client, args).await,
        "metrics" => collab::metrics(&client, args).await,
        "compare" | "comparisons" => compare::compare(&client, args).await,
        "texts" => texts::texts(&client, args).await,
        "own" => texts::own(&client, args).await,
        "example" | "examples" => examples::example(&client, args).await,
        "services" => collab::services(&client).await,
        "device" | "devices" => devices::device(&client, args).await,
        "names" | "name" => devices::names(&client, args).await,
        "extension" | "extensions" => extensions::extension(&client, args).await,
        "signin" | "signins" => signins::signin(&client, args).await,
        "secret" | "secrets" => signins::secret(&client, args).await,
        "caller" | "callers" => callers::caller(&client, args).await,
        "invite" | "tier" | "key" | "keys" | "give" => collab::admin(&client, args).await,
        "group" | "groups" => grouped::group(&client, args).await,
        "flag" | "flags" => flagged::run(&client, args).await,
        other => return usage(&format!("`{other}` is not a command; `{} help` lists them", brand::CLI)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        // A flag refused is said once, as `run` says it.
        Err(message) if args.heed().refused().as_ref() == Some(&message) => ExitCode::FAILURE,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("error: {message}");
            }
            ExitCode::FAILURE
        }
    }
}

fn usage(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
