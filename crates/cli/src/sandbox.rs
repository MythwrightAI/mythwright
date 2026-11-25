//! `sandbox`: a blueprint run where nothing is real.
//!
//! The daemon makes a sandbox for the text (a factory of its own, made
//! of nothing of the one in use), applies it there, and this does to it
//! what was asked, in the order it was asked: an event said, a card or a
//! row changed by hand, the clock moved on. Whatever reaches outside
//! the factory's tables is stood in and answers at once, unless what it
//! answers was said. What came of it is printed as it came about, with
//! what was stood in said at its foot, and the sandbox is ended.
//!
//! Nothing is worked out here: what happened, in order, is the daemon's
//! to say (`timeline` of what a sandbox hands back), and each step is
//! said by what it says of itself.

use serde_json::{Map, Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::work::{print_diagnostics, project_of};

/// `key=value` as a pair, the value read as JSON where it reads as it
/// (`3`, `true`, `[a]`) and as the words it is where it does not.
fn pair(said: &str, of: &str) -> Result<(String, Value), String> {
    let (key, value) =
        said.split_once('=').ok_or_else(|| format!("`{said}` after `--{of}` should be `key=value`"))?;
    Ok((key.to_string(), serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()))))
}

fn pairs(said: &[String], of: &str) -> Result<Map<String, Value>, String> {
    said.iter().map(|one| pair(one, of)).collect()
}

/// What was asked after the blueprint, in the order it was asked.
#[derive(Default)]
struct Asked {
    /// What is applied before the text, by name.
    first: Vec<String>,
    /// What a stand-in answers each time, by its action's name.
    told: Map<String, Value>,
    /// What is done once the text is applied.
    acts: Vec<Value>,
}

/// Reads what was asked from the arguments as they were typed: a word
/// that says what is done (`--say`, `--do`, `--advance`) is followed by
/// what it is done with, up to the next word that begins `--`, and they
/// are done in the order they stand.
fn asked(raw: &[String]) -> Result<Asked, String> {
    let mut asked = Asked::default();
    let mut at = 0;
    while at < raw.len() {
        let Some(flag) = raw[at].strip_prefix("--") else {
            at += 1;
            continue;
        };
        let after: Vec<String> =
            raw[at + 1..].iter().take_while(|word| !word.starts_with("--")).cloned().collect();
        let named = || after.first().cloned().ok_or_else(|| format!("`--{flag}` needs to be told what"));
        match flag {
            "first" => asked.first.push(named()?),
            "tell" => {
                asked.told.insert(named()?, Value::Object(pairs(&after[1..], flag)?));
            }
            "say" => {
                asked.acts.push(json!({ "say": { "topic": named()?, "data": pairs(&after[1..], flag)? } }))
            }
            "do" => {
                asked.acts.push(json!({ "do": { "action": named()?, "with": pairs(&after[1..], flag)? } }))
            }
            "advance" => asked.acts.push(json!({ "advance": { "by": named()? } })),
            _ => {}
        }
        at += 1 + after.len();
    }
    Ok(asked)
}

/// A value in a few words: what it is, cut where it is long.
fn brief(value: &Value) -> String {
    let said = match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let said = said.split_whitespace().collect::<Vec<_>>().join(" ");
    match said.char_indices().nth(96) {
        Some((cut, _)) => format!("{}…", &said[..cut]),
        None => said,
    }
}

fn text<'a>(line: &'a Value, key: &str) -> &'a str {
    line[key].as_str().unwrap_or_default()
}

/// What is said of an asking of a stand-in, on its step's line.
fn stood(asking: &Value) -> String {
    let answer = match (asking["failed"].as_str(), text(asking, "by")) {
        (Some(why), _) => format!("told to fail: {why}"),
        (None, "usual") => {
            format!("answered {} (what a stand-in for it answers)", brief(&asking["answered"]))
        }
        (None, "told") => format!("answered {} (as it was told)", brief(&asking["answered"])),
        (None, "called off") => "its step was called off while it waited".to_string(),
        (None, _) if asking["waiting"] == true => "waits to be answered".to_string(),
        (None, _) => format!("answered {}", brief(&asking["answered"])),
    };
    format!("stood in: `{}` was asked {} and {answer}", text(asking, "action"), brief(&asking["asked"]))
}

/// Prints what happened in a sandbox, in the order it came about: a
/// heading for each run as it comes to say something, its steps under
/// it, and what was said and started between them.
fn timeline(shown: &Value) {
    let empty = Vec::new();
    let instances = shown["instances"].as_array().unwrap_or(&empty);
    let called = |id: &Value| {
        let found = instances.iter().find(|one| &one["id"] == id);
        found.map(|one| format!("{} (`{}`)", text(one, "blueprint"), text(one, "key"))).unwrap_or_default()
    };
    let askings = shown["asked"].as_array().unwrap_or(&empty);
    let mut under = Value::Null;
    // A run is named once, when something of it is first said after
    // something of another: its heading, then what is said.
    let mut of_run = |line: &Value, said: String| {
        if line["run"].is_null() {
            // What is no run's, or whose run is not known, stands apart
            // from the run before it.
            if !under.is_null() {
                println!();
            }
            under = Value::Null;
        } else if line["run"] != under {
            under = line["run"].clone();
            let flow = match text(line, "flow").strip_prefix("on.") {
                Some(handler) => format!("handler `{handler}`"),
                None => "its steps".to_string(),
            };
            println!("\n{}: {flow}", called(&line["instance"]));
        }
        println!("{said}");
    };
    let lines = shown["timeline"].as_array().unwrap_or(&empty);
    let passed = |line: &Value| line["what"] == "step" && line["status"] == "skipped";
    for (at, line) in lines.iter().enumerate() {
        let whose = called(&line["instance"]);
        // Steps passed over one after another are one line: nothing
        // happened in any of them.
        if passed(line) {
            if at > 0 && passed(&lines[at - 1]) && lines[at - 1]["run"] == line["run"] {
                continue;
            }
            let over = lines[at..].iter().take_while(|next| passed(next) && next["run"] == line["run"]);
            let keys: Vec<String> = over.map(|next| format!("[{}]", text(next, "step"))).collect();
            of_run(line, format!("  passed over {}", keys.join(" ")));
            continue;
        }
        match text(line, "what") {
            "instance" => match text(line, "did") {
                "started" => println!("  + started {whose}"),
                "active" => println!("  = {whose} is up: its steps went well and it stays, listening"),
                "failed" => println!("  ! {whose} failed: {}", text(line, "error")),
                "stopped" => println!("  - {whose} was stopped"),
                "finished" if line["status"] == "done" => println!("  - {whose} is over"),
                _ => {}
            },
            "run" => match (text(line, "did"), text(line, "status")) {
                ("started", _) if !line["heard"].is_null() => {
                    of_run(line, format!("  heard `{}`", text(line, "heard")));
                }
                ("finished", "failed") => of_run(line, format!("  ended: failed. {}", text(line, "error"))),
                ("finished", "cancelled") => of_run(line, "  ended: cut short".to_string()),
                _ => {}
            },
            "step" => {
                let (key, says) = (text(line, "step"), text(line, "says"));
                let named = if says.is_empty() { key.to_string() } else { format!("{says} [{key}]") };
                let status = match (text(line, "status"), line["tolerated"] == true) {
                    ("running", _) => "under way",
                    // One that failed and whose run goes on all the same.
                    ("failed", true) => "let fail",
                    (other, _) => other,
                };
                of_run(line, format!("  {status:<10} {named}  ({})", text(line, "does")));
                let numbers = line["stood"].as_array().unwrap_or(&empty);
                for asking in askings.iter().filter(|asking| numbers.contains(&asking["id"])) {
                    println!("             {}", stood(asking));
                }
                let nothing = [Value::Null, json!({}), json!([])].contains(&line["gave"]);
                match (text(line, "status"), line["error"].as_str()) {
                    ("failed", Some(why)) => println!("             why: {why}"),
                    ("ok", _) if !nothing && numbers.is_empty() => {
                        println!("             gave {}", brief(&line["gave"]));
                    }
                    _ => {}
                }
            }
            "waits" => match line["event"].as_str() {
                Some(event) => {
                    of_run(line, format!("  waits      [{}] for `{event}` to be said", text(line, "step")));
                }
                None => of_run(line, format!("  waits      [{}] for the clock", text(line, "step"))),
            },
            "said" => {
                let said = format!("    said `{}` {}", text(line, "topic"), brief(&line["data"]));
                // What a step said is under its run, with the step that
                // said it, and what somebody said for no run stands apart.
                // What the factory itself says of something (a line of a
                // record's history) is where it came.
                match line["run"].is_null() && line["by"] == "system" {
                    true => println!("{said}"),
                    false => of_run(line, said),
                }
            }
            _ => {}
        }
    }
}

/// Prints what the sandbox ended with, and what was stood in.
fn foot(shown: &Value) {
    let empty = Vec::new();
    println!();
    for table in shown["tables"].as_array().unwrap_or(&empty) {
        let rows = table["records"].as_array().map_or(0, Vec::len);
        println!("table `{}`: {rows} {}", text(table, "name"), if rows == 1 { "row" } else { "rows" });
    }
    for channel in shown["channels"].as_array().unwrap_or(&empty) {
        for message in channel["messages"].as_array().unwrap_or(&empty) {
            println!(
                "#{} {}: {}",
                text(channel, "name"),
                text(message, "author_name"),
                brief(&message["body"])
            );
        }
    }
    // What was told to whom, the first told first: in a sandbox the one
    // inbox is that of the made-up person who applied the text.
    for line in shown["inbox"].as_array().unwrap_or(&empty).iter().rev() {
        let whom = match line["owner"] == shown["somebody"] {
            true => "whoever applied it",
            false => text(line, "owner"),
        };
        let times = match line["count"].as_u64() {
            Some(many) if many > 1 => format!(", {many} times"),
            _ => String::new(),
        };
        println!("told {whom} ({}{times}): {}", text(line, "why"), brief(&line["says"]));
    }
    // What was stood in, in the daemon's own sentence.
    println!();
    println!("{}", text(shown, "stood_said"));
    if let Some(over) = shown["over"].as_str() {
        println!("{over}");
    }
    println!("{}", text(shown, "said"));
}

pub async fn sandbox(client: &Client, args: &Args) -> Result<(), String> {
    let Some(named) = args.word(1) else {
        return Err(format!(
            "missing the blueprint to run in a sandbox: a file, or a blueprint's name such as `std/board` \
             (`{} help sandbox`)",
            brand::CLI
        ));
    };
    let asked = asked(args.raw_of(&["first", "tell", "say", "do", "advance"]))?;
    let mut params = Map::new();
    for said in args.values("param") {
        let (key, value) = pair(said, "param")?;
        params.insert(key, value);
    }
    let mut body = json!({
        "project": project_of(args),
        "params": params,
        "first": asked.first,
        "told": asked.told,
        // One that nothing was said of answers at once: nobody is there to be asked.
        "answers": "default",
    });
    match std::path::Path::new(named).is_file() {
        true => {
            body["source"] =
                json!(std::fs::read_to_string(named).map_err(|err| format!("cannot read {named}: {err}"))?)
        }
        false => body["name"] = json!(named),
    }
    let made = client.post("/api/sandboxes", body).await?;
    if made["ok"] != true {
        print_diagnostics(named, &made["diagnostics"]);
        return Err(format!(
            "`{named}` does not check, so it was not run in a sandbox: what is wrong with it is said above"
        ));
    }
    let id = made["sandbox"].as_str().ok_or("the daemon made no sandbox")?.to_string();
    let path = format!("/api/sandboxes/{id}");
    // Whatever becomes of an act, the sandbox is read and ended.
    let mut refused = None;
    for act in asked.acts {
        if let Err(why) = client.post(&format!("{path}/acts"), act).await {
            refused = Some(why);
            break;
        }
    }
    let shown = client.get(&path).await;
    let _ = client.call(reqwest::Method::DELETE, &path, None).await;
    let shown = shown?;
    if client.json {
        println!("{}", serde_json::to_string_pretty(&shown).unwrap_or_default());
    } else {
        println!(
            "A sandbox test of {named}: it is run in a factory of its own, with one made-up project called \
             `{}`, and nothing here is real.",
            text(&shown, "project")
        );
        timeline(&shown);
        foot(&shown);
    }
    if let Some(why) = refused {
        return Err(format!("{why}. What was asked after that was not done"));
    }
    let failed =
        shown["instances"].as_array().is_some_and(|all| all.iter().any(|one| one["status"] == "failed"));
    match (failed, shown["over"].is_null()) {
        (false, true) => Ok(()),
        (true, _) => Err("something failed in the sandbox: what, and why, is said above".to_string()),
        (false, false) => Err("the sandbox was stopped before it came to rest".to_string()),
    }
}
