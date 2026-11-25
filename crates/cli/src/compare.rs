//! Comparisons: one blueprint started several ways, side by side.
//!
//! The daemon does the comparing (the crate `comparisons`); this says one
//! from the command line, and shows what was measured under each way
//! beside the others. It prints counts and times and says nothing of
//! which is better.

use serde_json::{Map, Value, json};

use crate::args::Args;
use crate::client::{Client, ago, amount, encode, now_ms, table, took};
use crate::work::{list, show, text};

/// The ways as the daemon is told them: each
/// `--way <way>[=<blueprint>[@<version>]]`, with its `--share <way>=<n>`
/// and every `--with <way>.<param>=<value>`. A way held to a version
/// starts the text that calls itself that, as the factory kept it; and
/// `--prompt <way>.<file>=<other file>` has a way read another file
/// where one is linked, each by its name in the library.
fn ways(args: &Args) -> Result<Vec<Value>, String> {
    let cli = brand::CLI;
    let mut ways: Vec<(String, Map<String, Value>)> = Vec::new();
    for said in args.values("way") {
        let (name, starts) = match said.split_once('=') {
            Some((name, starts)) => (name, Some(starts)),
            None => (said, None),
        };
        let mut way = Map::new();
        way.insert("name".into(), json!(name));
        if let Some(starts) = starts {
            // No blueprint's name has an `@` in it: what follows one is the version it is held to.
            let (starts, version) = match starts.rsplit_once('@') {
                Some((starts, version)) => (starts, Some(version)),
                None => (starts, None),
            };
            if !starts.is_empty() {
                way.insert("blueprint".into(), json!(starts));
            }
            if let Some(version) = version {
                way.insert("version".into(), json!(version));
            }
        }
        ways.push((name.to_string(), way));
    }
    if ways.is_empty() {
        return Err(format!(
            "say at least one way with `--way <name>`; `{cli} help compare` shows a comparison whole"
        ));
    }
    let of =
        |name: &str, flag: &str| format!("`--{flag}` names the way `{name}`, and there is no `--way {name}`");
    for said in args.values("share") {
        let share = said.split_once('=').and_then(|(name, share)| Some((name, share.parse::<u32>().ok()?)));
        let (name, share) = share.ok_or_else(|| format!("`--share {said}` should be `<way>=<number>`"))?;
        let way = ways.iter_mut().find(|(way, _)| way == name).ok_or_else(|| of(name, "share"))?;
        way.1.insert("share".into(), json!(share));
    }
    for said in args.values("with") {
        let parts = said.split_once('=').and_then(|(to, value)| Some((to.split_once('.')?, value)));
        let ((name, param), value) =
            parts.ok_or_else(|| format!("`--with {said}` should be `<way>.<param>=<value>`"))?;
        let way = ways.iter_mut().find(|(way, _)| way == name).ok_or_else(|| of(name, "with"))?;
        let with = way.1.entry("with").or_insert_with(|| json!({}));
        // Read as a param of an apply is: JSON where it parses, else text.
        with[param] = serde_json::from_str(value).unwrap_or_else(|_| json!(value));
    }
    for said in args.values("prompt") {
        let parts = said.split_once('=').and_then(|(to, other)| Some((to.split_once('.')?, other)));
        let ((name, linked), other) = parts.ok_or_else(|| {
            format!(
                "`--prompt {said}` should be `<way>.<file>=<other file>`: the way, the file a step links as the \
                 library calls it, and the file to read in its place, such as `--prompt \
                 strict.std/prompts/review-seat.md=prompts/strict.md`"
            )
        })?;
        let way = ways.iter_mut().find(|(way, _)| way == name).ok_or_else(|| of(name, "prompt"))?;
        let reads = way.1.entry("prompts").or_insert_with(|| json!({}));
        reads[linked] = json!(other);
    }
    Ok(ways.into_iter().map(|(_, way)| Value::Object(way)).collect())
}

/// The events to count, as the daemon is told them: each
/// `--count <name>=<topic>:<field of the thing>[:<field>+<field>[:<word>+<word>]]`,
/// the last what it came to that says the thing was accepted; or
/// `--count cards` for the usual ones of a board's cards, which is what
/// is asked where none is said and a thing is a card (`--per item`).
/// `--count none` counts nothing.
fn counts(args: &Args) -> Result<Value, String> {
    let said = args.values("count");
    if said == ["cards"] || (said.is_empty() && args.value("per") == Some("item")) {
        return Ok(json!("cards"));
    }
    if said == ["none"] {
        return Ok(json!([]));
    }
    fn words(said: &str) -> Vec<&str> {
        said.split('+').filter(|word| !word.is_empty()).collect()
    }
    let mut counts = Vec::new();
    for said in said {
        let parts = said.split_once('=').map(|(name, of)| (name, of.split(':').collect::<Vec<_>>()));
        let (name, topic, thing, by, accepts) = match parts.as_ref().map(|(name, of)| (*name, of.as_slice()))
        {
            Some((name, [topic, thing])) => (name, *topic, *thing, Vec::new(), Vec::new()),
            Some((name, [topic, thing, by])) => (name, *topic, *thing, words(by), Vec::new()),
            Some((name, [topic, thing, by, accepts])) => (name, *topic, *thing, words(by), words(accepts)),
            _ => {
                return Err(format!(
                    "`--count {said}` should be `<name>=<topic>:<field>[:<field>+<field>[:<word>+<word>]]`: what \
                     the count is called, the event, the field of it that says which thing it is about, the \
                     fields that say what it came to, and what of that says the thing was accepted, such as \
                     `--count moved=board.item.moved:id:from+to:done`"
                ));
            }
        };
        counts.push(json!({ "name": name, "topic": topic, "thing": thing, "by": by, "accepts": accepts }));
    }
    Ok(json!(counts))
}

/// The rule a comparison's shares change by, as the daemon is told it:
/// `--grows <way> --against <way> --holds <count>=<what it came to>
/// [--at-least] --over <n> --step <n> --most <n> [--least <n>]`, the last
/// three parts of a hundred. Nothing where none of it is said; what is wrong with it is
/// the daemon's to say, where it is given.
fn rule(args: &Args) -> Result<Option<Value>, String> {
    let cli = brand::CLI;
    let said = ["grows", "against", "holds", "over", "step", "most"].map(|flag| args.value(flag));
    let at_least = args.has("at-least");
    let least = args.value("least");
    if said.iter().all(Option::is_none) && !at_least && least.is_none() {
        return Ok(None);
    }
    let [Some(grows), Some(against), Some(holds), Some(over), Some(step), Some(most)] = said else {
        return Err(format!(
            "a rule says each of `--grows <way> --against <way> --holds <count>=<what it came to> --over <n> \
             --step <n> --most <n>`; `{cli} help compare` shows one whole"
        ));
    };
    let (count, came) = holds.split_once('=').ok_or_else(|| {
        format!("`--holds {holds}` should be `<count>=<what it came to>`, such as `verdict=failed`")
    })?;
    let number = |flag: &str, value: &str| {
        value.parse::<u32>().map_err(|_| format!("`--{flag} {value}` should be a whole number"))
    };
    Ok(Some(json!({
        "grows": grows, "against": against, "count": count, "came": came, "at_least": at_least,
        "over": number("over", over)?, "step": number("step", step)?, "most": number("most", most)?,
        "least": least.map(|least| number("least", least)).transpose()?,
    })))
}

/// Each way's share and its part of a hundred, the rule its shares change
/// by with what it has read, and each time they were said again with
/// why, as the daemon lays it out (`beside`'s `rolled`).
fn rolled(laid: &Value) -> String {
    let rolled = &laid["rolled"];
    if rolled.is_null() {
        return String::new();
    }
    let parts: Vec<String> = list(&rolled["parts"])
        .iter()
        .map(|part| format!("{} {}%", text(part, "way"), part["percent"]))
        .collect();
    let mut said = vec![format!("Its things are shared out {}.", parts.join(", "))];
    let rule = &rolled["rule"];
    if !rule.is_null() {
        let read: Vec<String> = list(&rule["read"])
            .iter()
            .map(|read| format!("{} {} of {}", text(read, "way"), read["came"], read["things"]))
            .collect();
        let state = match rule["done"].as_bool() {
            Some(true) => "It has taken its way as far as it goes.".to_string(),
            _ => format!(
                "Since its shares were last set it has read {} (of {} each it reads before it decides).",
                read.join(", "),
                rule["over"]
            ),
        };
        said.push(format!("Its rule: {}. {state}", text(rule, "said")));
    }
    for shared in list(&rolled["shared"]) {
        let by = match text(shared, "by") {
            "rule" => "the rule".to_string(),
            by => by.to_string(),
        };
        said.push(format!(
            "{} by {by}: {}",
            ago(now_ms(), shared["at"].as_i64().unwrap_or(0)),
            text(shared, "why")
        ));
    }
    said.join("\n")
}

/// What a comparison counts of what comes of its things, by what each count is called.
fn counted(one: &Value) -> Vec<&str> {
    let mut names: Vec<&str> = list(&one["counts"]).iter().map(|count| text(count, "name")).collect();
    names.dedup();
    names
}

/// The same in a line, for when one is started; nothing where it counts nothing.
fn counting(one: &Value) -> String {
    match counted(one) {
        names if names.is_empty() => String::new(),
        names => {
            let accepts: Vec<String> = list(&one["counts"])
                .iter()
                .flat_map(|count| list(&count["accepts"]).iter().map(move |word| (count, word)))
                .map(|(count, word)| format!("{} {}", text(count, "name"), word.as_str().unwrap_or_default()))
                .collect();
            let accepted = match accepts.is_empty() {
                true => String::new(),
                false => format!("\nA thing is accepted where it comes to {}.", accepts.join(" or ")),
            };
            format!("\nIt counts what comes of each thing: {}.{accepted}", names.join(", "))
        }
    }
}

/// What a way starts and hands, in a few words.
fn does(way: &Value) -> String {
    let mut said = Vec::new();
    if let Some(starts) = way["blueprint"].as_str() {
        said.push(match way["version"].as_str() {
            Some(version) => format!("starts {starts} at version {version}"),
            None => format!("starts {starts}"),
        });
    }
    if let Some(with) = way["with"].as_object() {
        said.extend(with.iter().map(|(param, value)| match value {
            Value::String(text) => format!("{param}={text}"),
            other => format!("{param}={other}"),
        }));
    }
    if let Some(reads) = way["prompts"].as_object() {
        said.extend(
            reads.iter().map(|(linked, reads)| format!("reads {} for {linked}", text(reads, "name"))),
        );
    }
    match said.is_empty() {
        true => "as it is written".into(),
        false => said.join(", "),
    }
}

/// One line for a comparison: what it is of and where it stands.
fn stands(one: &Value) -> String {
    let place = match text(one, "project") {
        "" => String::new(),
        project => format!(" in {project}"),
    };
    let state = match (one["ended"].is_null(), one["goes"].as_str()) {
        (true, _) => format!("going, started {}", ago(now_ms(), one["started"].as_i64().unwrap_or(0))),
        (false, Some(goes)) => format!("ended; everything goes `{goes}`"),
        (false, None) => "ended; everything goes as it is written".into(),
    };
    let asked = match one["asked"].as_bool() {
        Some(true) => ", its ways named by the start",
        _ => "",
    };
    format!("{} of {}{place}{asked}: {state}", text(one, "name"), text(one, "of"))
}

/// A text a way ran, as it is said: the version it says, the start of
/// its revision, and whether it is the one the way runs now.
fn ran(way: &Value, text: &str) -> String {
    let texts = list(&way["texts"]);
    let Some(at) = texts.iter().position(|one| text == crate::work::text(one, "text")) else {
        return if text.is_empty() { "-".into() } else { text.to_string() };
    };
    let version =
        texts[at]["version"].as_str().map(|version| format!("version {version} ")).unwrap_or_default();
    let now = if at + 1 == texts.len() && texts.len() > 1 { " (now)" } else { "" };
    format!("{version}{text}{now}")
}

/// The figures of one kind of row, as they are headed and written: for
/// each column two, side by side.
struct Kind {
    /// What the rows of this kind are, over their names.
    head: &'static str,
    figures: [&'static str; 2],
}

fn kind(word: &str) -> Kind {
    match word {
        "came" => Kind { head: "came of them", figures: ["how many", "after"] },
        "chose" => Kind { head: "chosen", figures: ["how many", "average"] },
        "spent" => Kind { head: "added up", figures: ["in all", "a thing"] },
        "accepted" => Kind { head: "accepted", figures: ["how many", "of those given"] },
        "per_accepted" => Kind { head: "for what was accepted", figures: ["in all", "each accepted"] },
        _ => Kind { head: "measured", figures: ["how many", "average"] },
    }
}

/// One figure of the two a column has for a row.
fn figures(row: &Value, cell: &Value) -> [String; 2] {
    if cell.is_null() {
        return ["-".into(), String::new()];
    }
    let number = |key: &str| cell[key].as_f64().unwrap_or(0.0);
    let or_none = |figure: Option<String>| figure.unwrap_or_else(|| "-".into());
    match (text(row, "kind"), row["unit"].as_str()) {
        // How many were accepted, and that over how many were given.
        ("accepted", _) => {
            let rate = cell["rate"].as_f64().map(|rate| format!("{:.0}%", rate * 100.0));
            return [cell["count"].to_string(), or_none(rate)];
        }
        // What was spent in all, over how many were accepted.
        ("per_accepted", Some(unit)) => {
            let each = cell["per_accepted"].as_f64().map(|each| amount(each, unit));
            return [amount(number("sum"), unit), or_none(each)];
        }
        _ => {}
    }
    if let Some(unit) = row["unit"].as_str() {
        // What it came to in all, and for each thing the column was given.
        let each = cell["per_thing"].as_f64().map(|each| amount(each, unit));
        return [amount(number("sum"), unit), each.unwrap_or_else(|| "-".into())];
    }
    let count = match cell["errors"].as_u64().unwrap_or(0) {
        0 => cell["count"].to_string(),
        failed => format!("{} ({failed} failed)", cell["count"]),
    };
    [count, took(number("avg") as i64)]
}

/// What was measured under a comparison, laid beside each other as the
/// daemon lays it (`beside`): a column for each text of each way, with
/// how many things it was given, and under that what came of them, what
/// was chosen, what was added up and what else was measured. One screen;
/// nothing in it says which column is the better.
fn beside(one: &Value, laid: &Value) -> String {
    let (columns, rows) = (list(&laid["columns"]), list(&laid["rows"]));
    let across = |cells: &dyn Fn(&Value) -> [String; 2]| columns.iter().flat_map(cells).collect::<Vec<_>>();
    let line = |first: &str, rest: Vec<String>| [vec![first.to_string()], rest].concat();
    let ways = list(&one["ways"]);
    let way = |column: &Value| ways.iter().find(|way| text(way, "name") == text(column, "way"));
    let mut lines = vec![line("", across(&|column| [text(column, "way").to_string(), String::new()]))];
    // Which text each column is of, where any is known: a way that ran several has a column for each.
    if columns.iter().any(|column| !text(column, "text").is_empty()) {
        let said = |column: &Value| match way(column) {
            Some(way) => [ran(way, text(column, "text")), String::new()],
            None => [text(column, "text").to_string(), String::new()],
        };
        lines.push(line("", across(&said)));
    }
    // Which words each column was run with, where a way reads others in place of a file that is linked.
    let mut linked: Vec<&str> =
        columns.iter().flat_map(|column| list(&column["reads"])).map(|reads| text(reads, "for")).collect();
    linked.sort_unstable();
    linked.dedup();
    for file in linked {
        let read = |column: &Value| {
            let reads = list(&column["reads"]).iter().find(|reads| text(reads, "for") == file);
            match reads {
                Some(reads) => [
                    format!("{} {}", text(reads, "name"), text(reads, "words")).trim().to_string(),
                    String::new(),
                ],
                None => ["as it is linked".to_string(), String::new()],
            }
        };
        lines.push(line(file, across(&read)));
    }
    let given = |column: &Value| match column["given"].as_u64() {
        Some(given) => [given.to_string(), String::new()],
        None => ["-".into(), String::new()],
    };
    lines.push(line("things given", across(&given)));
    let mut last = "";
    for row in rows {
        // Each kind is headed once, with what its two figures are.
        if text(row, "kind") != last {
            last = text(row, "kind");
            let kind = kind(last);
            lines.push(Vec::new());
            lines.push(line(kind.head, across(&|_| kind.figures.map(str::to_string))));
        }
        let by = list(&row["by"]);
        lines.push(line(text(row, "what"), by.iter().flat_map(|cell| figures(row, cell)).collect()));
    }
    table(&lines)
}

/// A comparison, its ways, and what was measured: each way's each text a
/// column of its own, so that what one text did is never added to what
/// another did.
fn shown(found: &Value) -> String {
    let one = &found["comparison"];
    let ways = list(&one["ways"]);
    let mut rows = vec![vec!["way".to_string(), "share".into(), "went".into(), "what".into(), "runs".into()]];
    for way in ways {
        let now = list(&way["texts"]).last().map(|last| ran(way, text(last, "text")).replace(" (now)", ""));
        rows.push(vec![
            text(way, "name").into(),
            way["share"].to_string(),
            way["went"].to_string(),
            does(way),
            now.unwrap_or_else(|| "-".into()),
        ]);
    }
    let per = match text(one, "per") {
        "" => "a thing is a place in the tree".to_string(),
        per => format!("a thing is what `{per}` says"),
    };
    let laid = &found["beside"];
    let accepted = match list(&laid["rows"]).iter().any(|row| text(row, "kind") == "accepted") {
        true => {
            " `of those given` is how many of the things a way was given were accepted; `each accepted` is \
                 what it spent in all, its work that was not accepted with it, over how many were."
        }
        false => "",
    };
    let measured = match list(&laid["rows"]).is_empty() {
        true => "nothing has been measured under it yet".to_string(),
        false => format!(
            "{}\n\n`after` is how long after a thing was given its way, on average; `a thing` is what was added \
             up, for each thing given.{accepted} Which way did better is yours to say.",
            beside(one, laid)
        ),
    };
    let counts = match counted(one) {
        names if names.is_empty() => String::new(),
        names => format!("; it counts {}", names.join(", ")),
    };
    let rolled = match rolled(laid) {
        said if said.is_empty() => said,
        said => format!("\n{said}"),
    };
    format!("{}; {per}{counts}\n{}{rolled}\n\n{measured}", stands(one), table(&rows))
}

/// `compare [list] | start <name> … | show <name> | share <name> <way>=<n>… | rule <name> … | end <name> [--goes <way>]`.
pub async fn compare(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    match args.word(1) {
        None | Some("list") => {
            let project =
                args.value("project").map(|p| format!("?project={}", encode(p))).unwrap_or_default();
            let found = client.get(&format!("/api/comparisons{project}")).await?;
            show(client, &found, || {
                let lines: Vec<String> = list(&found["comparisons"]).iter().map(stands).collect();
                match lines.is_empty() {
                    true => format!("nothing is being compared. `{cli} help compare` says how something is"),
                    false => lines.join("\n"),
                }
            });
            Ok(())
        }
        Some("start") => {
            let name = args.need(2, "what to call the comparison, such as `models`")?;
            let of = args.value("of").ok_or_else(|| {
                "missing `--of <blueprint>`: the blueprint whose starting is compared, such as `std/lane-round`"
                    .to_string()
            })?;
            let asked = json!({
                "name": name,
                "project": args.value("project"),
                "of": of,
                "per": args.value("per"),
                "also": args.values("also"),
                "ways": ways(args)?,
                "counts": counts(args)?,
                "asked": args.has("asked"),
                "rule": rule(args)?,
            });
            let made = client.post("/api/comparisons", asked).await?;
            let goes = match made["asked"].as_bool() == Some(true) {
                true => format!(
                    "a start of `{of}` whose params say `compared: {name}` and `way: <one of its ways>` goes that \
                     way and keeps to it; nothing else is its"
                ),
                false => format!("each thing `{of}` is started for goes one of its ways and keeps to it"),
            };
            show(client, &made, || {
                format!(
                    "{}\nFrom now, {goes}.{} \
                     `{cli} compare show {name}` sets what is measured under each beside the others; \
                     `{cli} compare end {name} [--goes <way>]` ends it.",
                    stands(&made),
                    counting(&made)
                )
            });
            Ok(())
        }
        Some("show") => {
            let name = args.need(2, "the comparison's name")?;
            let found = client.get(&format!("/api/comparisons/{}", encode(name))).await?;
            show(client, &found, || shown(&found));
            Ok(())
        }
        Some("end") => {
            let name = args.need(2, "the comparison's name")?;
            let path = format!("/api/comparisons/{}/end", encode(name));
            let ended = client.post(&path, json!({ "goes": args.value("goes") })).await?;
            show(client, &ended, || stands(&ended));
            Ok(())
        }
        Some("share") => {
            let name = args.need(2, "the comparison's name")?;
            let mut shares = Map::new();
            for said in args.words_from(3) {
                let share =
                    said.split_once('=').and_then(|(way, share)| Some((way, share.parse::<u32>().ok()?)));
                let (way, share) =
                    share.ok_or_else(|| format!("`{said}` should be `<way>=<share>`, such as `slate=1`"))?;
                shares.insert(way.to_string(), json!(share));
            }
            let path = format!("/api/comparisons/{}/shares", encode(name));
            let shared = client.post(&path, json!({ "shares": shares, "why": args.value("why") })).await?;
            show(client, &shared, || stands(&shared));
            Ok(())
        }
        Some("rule") => {
            let name = args.need(2, "the comparison's name")?;
            let rule = match args.has("none") {
                true => None,
                false => Some(rule(args)?.ok_or_else(|| {
                    format!("say the rule, or `--none` to take it away; `{cli} help compare` shows one whole")
                })?),
            };
            let path = format!("/api/comparisons/{}/rule", encode(name));
            let ruled = client.call(reqwest::Method::PUT, &path, Some(json!({ "rule": rule }))).await?;
            show(client, &ruled, || stands(&ruled));
            Ok(())
        }
        Some(other) => Err(format!(
            "`compare {other}` is not a command; there are `list`, `start <name>`, `show <name>`, `share <name>`, \
             `rule <name>` and `end <name>`"
        )),
    }
}
