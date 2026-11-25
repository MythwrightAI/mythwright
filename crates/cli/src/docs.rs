//! `docs export <folder>`: everything the program says of itself, written
//! out as pages, with no daemon asked (`api::documented`).

use std::path::Path;
use std::process::ExitCode;

use crate::args::Args;

pub async fn docs(args: &Args) -> ExitCode {
    let (sub, folder) = (args.word(1), args.word(2));
    let json = args.has("json");
    // A flag nobody reads is refused before anything is written (`main`
    // says it).
    if args.heed().check().is_err() {
        return ExitCode::from(2);
    }
    let refused = |why: String| {
        eprintln!("error: {why}");
        ExitCode::from(2)
    };
    let Some(folder) = folder.filter(|_| sub == Some("export")) else {
        return refused(format!(
            "say where the pages are written: `{} docs export <folder>` (`{} help docs`)",
            brand::CLI,
            brand::CLI
        ));
    };
    let pages = match api::documented::alone().await {
        Ok(pages) => pages,
        Err(err) => {
            eprintln!("error: {}", err.message);
            return ExitCode::FAILURE;
        }
    };
    match api::documented::put_in(Path::new(folder), &pages) {
        Ok(changed) => {
            if json {
                let said = serde_json::json!({
                    "to": folder, "pages": pages.len(), "version": manual::VERSION, "changed": changed,
                });
                println!("{said:#}");
            } else if changed {
                println!(
                    "{} pages of {} {} written to {folder}",
                    pages.len(),
                    brand::DISPLAY,
                    manual::VERSION
                );
            } else {
                println!(
                    "{folder} already holds the {} pages of {} {}: nothing was written",
                    pages.len(),
                    brand::DISPLAY,
                    manual::VERSION
                );
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {}", err.message);
            ExitCode::FAILURE
        }
    }
}
