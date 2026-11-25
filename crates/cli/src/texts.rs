//! `texts`: how what is running comes by new texts, said and changed.
//!
//! The setting is the daemon's (`conductor::taking`); this says it in
//! words, and says of a change what it does to what is running now.
//!
//! `own`: a project's own copy of a blueprint that ships, which is a fork
//! of it: made, looked at against what it was forked from and against the
//! one that ships, and taken away again.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::{Client, ago, encode, now_ms};
use crate::work::{list, show, text};

/// The setting, as a person is told it.
fn said(found: &Value) -> String {
    let cli = brand::CLI;
    let come = match (text(found, "come"), text(found, "usual")) {
        ("held", _) => format!(
            "texts are held: what is running goes on with the blueprints it has until somebody applies it \
             saying to take them as they now are (`{cli} apply ... --take`)"
        ),
        ("follow", _) => format!(
            "texts are followed: a blueprint that changes is taken as it changes by everything running that \
             reaches it, and nobody is asked. {}",
            sentence(text(found, "gives_up"))
        ),
        (_, "held") => "nothing was said of how texts are come by, so it is as usual for an installed factory: \
                        what is running goes on with the blueprints that came with the program until somebody \
                        applies it with `--take`, and a project's own are run as they were vouched for"
            .to_string(),
        _ => format!(
            "nothing was said of how texts are come by, so it is as usual for a factory run from its checkout: \
             the checkout's blueprints are followed, and one that changes is taken as it changes by what is \
             running. A project's own are run as they were vouched for until somebody says `{cli} texts follow`"
        ),
    };
    let land = match text(found, "land") {
        "again" => "A text that is taken lands by its steps being run again",
        _ => "A text that is taken lands in place: what is running is carried over, or takes it where it is",
    };
    let by = match (found["by"].as_str(), found["at"].as_i64()) {
        (Some(by), Some(at)) => format!("\nsaid by {by}, {}", ago(now_ms(), at)),
        _ => String::new(),
    };
    format!("{come}.\n{land}.{by}")
}

fn sentence(words: &str) -> String {
    let mut letters = words.chars();
    match letters.next() {
        Some(first) => format!("{}{}.", first.to_uppercase(), letters.as_str()),
        None => String::new(),
    }
}

/// `texts | texts held|follow|usual [--lands in-place|again] | texts --lands in-place|again`.
pub async fn texts(client: &Client, args: &Args) -> Result<(), String> {
    let (come, land) = (args.word(1), args.value("lands"));
    if come.is_none() && land.is_none() {
        let found = client.get("/api/system/texts").await?;
        show(client, &found, || said(&found));
        return Ok(());
    }
    let now = client.post("/api/system/texts", json!({ "come": come, "land": land })).await?;
    show(client, &now, || {
        let mut lines = vec![said(&now)];
        for one in list(&now["not_taken"]) {
            lines.push(format!(
                "`{}` does not check as it now is, so `{}` goes on with the text it has:\n{}",
                text(one, "blueprint"),
                text(one, "key"),
                text(one, "error")
            ));
        }
        lines.join("\n")
    });
    Ok(())
}

/// How a project's own and the one that ships differ, in a line: the
/// project's own set against what ships (`differs` says what ships as
/// `was` and the project's own as `now`), as its summary sets the fork
/// against what it was forked from, by the version each says first.
fn against(differs: &Value) -> String {
    if differs["same"] == true {
        return "it says what the one that ships says".to_string();
    }
    let versions = match (differs["was"].as_str(), differs["now"].as_str()) {
        (Some(ships), Some(own)) if ships != own => format!(", version {ships} where this says {own}"),
        (Some(ships), Some(_)) => format!(", version {ships} as this is"),
        (Some(ships), None) => format!(", version {ships} where this says none"),
        (None, Some(own)) => format!(", which says no version where this says {own}"),
        (None, None) => String::new(),
    };
    let changed: Vec<&str> = list(&differs["changed"]).iter().filter_map(Value::as_str).collect();
    let changed = match changed.is_empty() {
        true => String::new(),
        false => format!(": {}", changed.join("; ")),
    };
    format!("it differs from the one that ships now{versions}{changed}")
}

/// What was kept when a fork was made, in a line; nothing for one an
/// earlier program made.
fn forked(origin: &Value) -> Option<String> {
    let name = origin["blueprint"].as_str()?;
    let version = origin["version"].as_str().map(|said| format!(" version {said}")).unwrap_or_default();
    let when = origin["at"].as_i64().map(|at| format!(", {}", ago(now_ms(), at))).unwrap_or_default();
    Some(format!("forked from `{name}`{version} by {}{when}", text(origin, "by_name")))
}

/// What the summary of a fork says of where it stands (`said` of the
/// daemon's answer), as a terminal prints a summary's parts, with the
/// command for what a page does by a press.
fn stands(said: &Value, name: &str, project: &str) -> String {
    let Ok(parts) = serde_json::from_value::<Vec<blueprint::summary::Part>>(said.clone()) else {
        return String::new();
    };
    if parts.is_empty() {
        return String::new();
    }
    let hints = crate::explain::hints(&parts, name, Some(project));
    let printed = blueprint::summary::plain(&blueprint::summary::Summary { name: String::new(), parts });
    // Without the line that names it, which is said above.
    let mut lines = vec![printed.trim_start_matches('\n').trim_end().to_string()];
    lines.extend(hints);
    format!("\n\n{}", lines.join("\n\n"))
}

/// `own <blueprint> --project P [--say | --back [--force]]`.
pub async fn own(client: &Client, args: &Args) -> Result<(), String> {
    let cli = brand::CLI;
    let name =
        args.need(1, "the blueprint that ships to make the project's own, such as `std/lane-round`")?;
    let project = crate::work::project_of(args)
        .ok_or_else(|| format!("say whose it is to be with `--project <project>`; `{cli} help own`"))?;
    let asked = json!({
        "blueprint": name, "say": args.has("say"), "back": args.has("back"), "force": args.has("force"),
    });
    let done = client.post(&format!("/api/projects/{}/own", encode(&project)), asked).await?;
    show(client, &done, || {
        if args.has("say") {
            let from = forked(&done["origin"]).map(|from| format!(", {from}")).unwrap_or_default();
            // Where what it was forked from was kept and is what ships still, it is the
            // fork that was changed: where, is said under this by what it was added to.
            let moved = list(&done["said"]).iter().any(|part| part["kind"] == "moved");
            let how = match done["differs"]["same"] != true && !from.is_empty() && !moved {
                true => {
                    "it was changed since it was forked, and the one that ships is as it was then".to_string()
                }
                false => against(&done["differs"]),
            };
            return format!(
                "`{}` is `{project}`'s own `{name}`{from}; {how}{}",
                text(&done, "path"),
                stands(&done["said"], name, &project)
            );
        }
        if args.has("back") {
            return format!(
                "`{}` is taken away: `{name}` is the one that ships again in `{project}`. What is running goes \
                 on with the text it has until it is applied (`{cli} apply ... --take`)",
                text(&done, "removed")
            );
        }
        let version =
            done["version"].as_str().map(|version| format!(" (version {version})")).unwrap_or_default();
        let held = match done["held"] == true {
            true => format!(
                "\nIt is the text what is running in `{project}` goes on with, which is not the one that ships \
                 now; {}.",
                against(&done["differs"])
            ),
            false => String::new(),
        };
        // Words a blueprint links have no summary to explain them by: `--say` says it all.
        let explained = match blueprint::parts::is_linked(name) {
            true => format!("{cli} own {name} --project {project} --say"),
            false => format!("{cli} explain {name} --project {project}"),
        };
        format!(
            "`{name}`{version} is now `{project}`'s own: `{}`{held}\nFrom here it is a file of the project's like \
             any other. What runs in `{project}` reads it in place of the one that ships, as whoever applies or \
             saves it vouches for it, and it no longer moves when a new version of this program brings \
             another: `{cli} own {name} --project {project} --say` says where the two then differ.\nIt is a fork \
             of `{name}`, and what it was forked from is kept: `{explained}` says that, and where it differs \
             from it.\nTo go back to the one that ships: `{cli} own {name} --project {project} --back`.",
            text(&done, "path")
        )
    });
    Ok(())
}
