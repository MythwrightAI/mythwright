//! What the program writes out of itself (`api::documented`): which pages
//! there are, held to a kept list (`contract/docs.txt`) as the other
//! contract files are; the action that writes them into a project; and
//! what every blueprint that ships says of what it may be told, which a
//! page of each says again.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The pages a kept list says there are.
fn pages_of(listed: &str) -> BTreeSet<String> {
    listed.lines().filter_map(|line| line.strip_prefix("page ")).map(str::to_string).collect()
}

/// The head of a kept list: what it says of every page.
fn head_of(listed: &str) -> Option<String> {
    listed.lines().find_map(|line| line.strip_prefix("head ")).map(str::to_string)
}

/// The gate. What the program writes out now is listed as the kept file
/// lists it, and compared: a page added is written down by asking
/// (`WRITE_DOCS=1`), and one taken away, which somebody may link to, or a
/// head that says otherwise, only by saying why (`BREAK_DOCS="why"`), which
/// is kept in `docs-broken.txt`.
#[tokio::test]
async fn the_docs_kept_with_the_code_are_the_pages_this_program_writes_out() {
    let rig = Rig::fresh().await;
    let pages = api::documented::pages(rig.node().app.engine.registry());
    let now = manual::pages::listed(&pages);
    let file = repo().join("contract").join("docs.txt");
    let kept = std::fs::read_to_string(&file).unwrap_or_default();
    let (was, is) = (pages_of(&kept), pages_of(&now));
    let mut broken: Vec<String> =
        was.difference(&is).map(|page| format!("page {page} is no longer written")).collect();
    if !kept.is_empty() && head_of(&kept) != head_of(&now) {
        broken.push(format!(
            "every page's head said `{}` and says `{}`",
            head_of(&kept).unwrap_or_default(),
            head_of(&now).unwrap_or_default()
        ));
    }
    let added: Vec<&String> = is.difference(&was).collect();
    match (std::env::var("BREAK_DOCS"), std::env::var_os("WRITE_DOCS")) {
        (Ok(why), _) => {
            let now_ms =
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
            let day = api::contract::day(now_ms as i64);
            let mut kept_broken = std::fs::read_to_string(repo().join("contract/docs-broken.txt")).unwrap_or_else(|_| {
                "# Every page the program writes out of itself that was taken away, or a head that came to say\n\
                 # something else: the day, what, and why. A test adds to this when asked\n\
                 # (BREAK_DOCS=\"why\" cargo test -p api the_docs); nothing is taken off it.\n"
                    .to_string()
            });
            for line in &broken {
                kept_broken.push_str(&format!("{day}\t{line}\t{why}\n"));
            }
            std::fs::write(repo().join("contract/docs-broken.txt"), kept_broken).unwrap();
            std::fs::write(&file, now).unwrap();
        }
        (_, Some(_)) => {
            assert!(
                broken.is_empty(),
                "{}\nthat breaks what a reader may link to: BREAK_DOCS=\"why\" cargo test -p api the_docs",
                broken.join("\n")
            );
            std::fs::write(&file, now).unwrap();
        }
        _ => {
            assert!(
                broken.is_empty(),
                "the pages the program writes out are not those kept in contract/docs.txt:\n{}\nsomebody may link to it: where it is meant, BREAK_DOCS=\"why\" cargo test -p api the_docs",
                broken.join("\n")
            );
            assert!(
                added.is_empty() && kept == now,
                "pages the program writes out that contract/docs.txt does not list:\n{}\nwrite them down with WRITE_DOCS=1 cargo test -p api the_docs",
                added.iter().map(|page| format!("page {page}")).collect::<Vec<_>>().join("\n")
            );
        }
    }
}

/// What the action the pages are written with says of what it is: every
/// action and event there is has its page, and the index links each.
#[tokio::test]
async fn every_action_event_and_blueprint_that_ships_has_a_page_the_index_links() {
    let rig = Rig::fresh().await;
    let registry = rig.node().app.engine.registry();
    let pages = api::documented::pages(registry);
    let index = &pages["index.md"];
    let head = format!("---\npublic: true\nversion: {}\n---\n", manual::VERSION);
    for (page, text) in &pages {
        assert!(text.starts_with(&head), "{page} says it is public and the version first");
        if page != "index.md" {
            assert!(index.contains(&format!("]({page})")), "the index links {page}");
        }
    }
    let mut actions = blueprint::intrinsics();
    actions.extend(registry.specs().specs().iter().cloned());
    for action in &actions {
        let page = &pages[&manual::pages::action_page(&action.name)];
        assert!(page.contains(&action.summary), "{} says what it does", action.name);
        if let Some(example) = &action.example {
            assert!(page.contains(example.trim_end()), "{} shows its example", action.name);
        }
    }
    for event in registry.events().specs() {
        assert!(pages[&manual::pages::event_page(&event.topic)].contains(&event.summary), "{}", event.topic);
    }
    for shape in blueprint::vocabulary::SHAPES {
        let page = &pages[&manual::pages::keyword_page(shape.name)];
        for field in shape.fields {
            assert!(page.contains(field.help), "{}: {} says what it is for", shape.name, field.name);
        }
    }
    for page in ["blueprints/std/board.md", "blueprints/std/fleet.md", "releases.md", "guides/blueprints.md"]
    {
        assert!(pages.contains_key(page), "{page}");
    }
    // The summary of each is in plain words: none failed to read.
    assert!(!pages.values().any(|text| text.contains("does not read as a blueprint here")));
}

#[test]
fn every_param_of_a_blueprint_that_ships_says_what_it_is() {
    use conductor::Library;
    let mut texts: Vec<(String, String)> = Vec::new();
    let library = conductor::standard();
    for name in library.names(None) {
        if !conductor::library::is_linked(&name) {
            texts.push((name.clone(), library.load(&name, None).unwrap()));
        }
    }
    // And what every package that ships brings.
    let mut pending = vec![repo().join("extensions")];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if brand::is_blueprint_path(&path.to_string_lossy()) {
                texts.push((path.display().to_string(), std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    assert!(texts.len() > 60, "{} blueprints ship", texts.len());
    let mut unsaid = Vec::new();
    for (name, text) in &texts {
        let checked = blueprint::check(text, &blueprint::NoCatalog);
        let read = checked.blueprint.unwrap_or_else(|| panic!("{name} reads"));
        for param in read.params {
            if param.description.as_deref().is_none_or(|said| said.trim().is_empty()) {
                unsaid.push(format!("{name}: `{}`", param.name));
            }
        }
    }
    assert!(
        unsaid.is_empty(),
        "params of blueprints that ship that say nothing of what they are:\n{}",
        unsaid.join("\n")
    );
}

/// Applies a blueprint of one step that writes the pages into `with`'s
/// folder of `shop`, as `token`.
async fn export(rig: &Rig, token: &str, to: &str) -> (StatusCode, Value) {
    let source = format!(
        "blueprint: reference\npermissions: [projects]\nsteps:\n  written:\n    do: docs.export\n    with: {{ to: \"{to}\" }}\noutputs:\n  written: ${{ steps.written.out }}\n"
    );
    let asked = json!({ "source": source, "project": "shop", "wait": true, "force": true });
    rig.call(Method::POST, "/api/apply", Some(token), Some(asked)).await
}

#[tokio::test]
async fn docs_export_writes_the_pages_into_a_folder_of_the_project_and_replaces_only_what_it_wrote() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = rig.node().app.studio.projects.dir_of("shop").unwrap();

    let (status, applied) = export(&rig, &rig.token, "docs/reference").await;
    assert_eq!((status, &applied["finished"]["status"]), (StatusCode::OK, &json!("ok")), "{applied}");
    let written = &applied["finished"]["outputs"]["written"];
    assert_eq!(written["changed"], true, "{written}");
    assert_eq!(written["version"], manual::VERSION);
    assert_eq!(written["to"], "docs/reference");
    let made = api::documented::pages(rig.node().app.engine.registry());
    assert_eq!(written["pages"], made.len());
    for (page, text) in &made {
        assert_eq!(&std::fs::read_to_string(dir.join("docs/reference").join(page)).unwrap(), text, "{page}");
    }

    // Again, with nothing changed: nothing is written.
    let (_, again) = export(&rig, &rig.token, "docs/reference").await;
    assert_eq!(again["finished"]["outputs"]["written"]["changed"], false, "{again}");

    // A folder that holds what no export wrote is left as it is.
    std::fs::write(dir.join("docs/reference/mine.md"), "mine").unwrap();
    let (_, refused) = export(&rig, &rig.token, "docs/reference").await;
    assert_eq!(refused["finished"]["status"], "failed", "{refused}");
    assert!(refused.to_string().contains("which no export put there"), "{refused}");
    assert_eq!(std::fs::read_to_string(dir.join("docs/reference/mine.md")).unwrap(), "mine");

    // Nowhere but a folder of the project.
    for elsewhere in ["../outside", "", "@blueprints/reference"] {
        let (_, refused) = export(&rig, &rig.token, elsewhere).await;
        assert_eq!(refused["finished"]["status"], "failed", "`{elsewhere}`: {refused}");
    }
    assert!(!dir.join("../outside").exists());

    // Writing the project's files is what it takes, and no more.
    let made = rig
        .send("/api/credentials", json!({ "name": "reader", "scopes": ["blueprints", "projects:read"] }))
        .await;
    let reader = made["token"].as_str().unwrap().to_string();
    let (status, answer) = export(&rig, &reader, "docs/other").await;
    assert!(status != StatusCode::OK || answer["finished"]["status"] == "failed", "{answer}");
    assert!(answer.to_string().contains("projects:write"), "{answer}");
    assert!(!dir.join("docs/other").exists());
    let made =
        rig.send("/api/credentials", json!({ "name": "writer", "scopes": ["blueprints", "projects"] })).await;
    let writer = made["token"].as_str().unwrap().to_string();
    let (_, answer) = export(&rig, &writer, "docs/other").await;
    assert_eq!(answer["finished"]["status"], "ok", "{answer}");
    assert!(dir.join("docs/other/index.md").is_file());
}
