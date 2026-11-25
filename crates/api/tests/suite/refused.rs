//! What a person's first mistakes are answered with, where the answer is
//! the daemon's: an apply to a project that is not there, a permission a
//! text lists that whoever applies it does not hold, and a name that is
//! not there pointed at where it is written.

use reqwest::{Method, StatusCode};
use serde_json::Value;
use serde_json::json;

use crate::Rig;
use crate::held::{key, said};

const HELLO: &str = "\
blueprint: hello
steps:
  say: { do: log, with: { message: hello } }
";

#[tokio::test]
async fn an_apply_to_a_project_that_is_not_there_is_refused_and_starts_nothing() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let before = rig.read("/api/instances").await["instances"].as_array().unwrap().len();
    // A project's name misspelt: applied all the same, it ran under a
    // name nothing answers to.
    for asked in [
        json!({ "source": HELLO, "project": "shpo", "wait": true }),
        json!({ "blueprint": "std/docs", "project": "shpo" }),
    ] {
        let (status, refused) = rig.post("/api/apply", asked).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{refused}");
        assert_eq!(
            said(&refused),
            format!(
                "there is no project `shpo`, so nothing was applied; `{} project list` says which there are",
                brand::CLI
            )
        );
    }
    assert_eq!(rig.read("/api/instances").await["instances"].as_array().unwrap().len(), before);
    // The one that is there is applied to, and so is no project at all.
    for asked in [json!({ "source": HELLO, "project": "shop", "wait": true }), json!({ "source": HELLO })] {
        let applied = rig.send("/api/apply", asked).await;
        assert!(applied["instance"].is_string(), "{applied}");
    }
}

/// The warnings an apply answered with, each by its code.
fn warned(applied: &Value) -> Vec<(String, String)> {
    let text = |one: &Value, key: &str| one[key].as_str().unwrap_or_default().to_string();
    applied["warnings"]
        .as_array()
        .map(|all| all.iter().map(|one| (text(one, "code"), text(one, "message"))).collect())
        .unwrap_or_default()
}

#[tokio::test]
async fn a_permission_a_text_lists_and_whoever_applies_it_does_not_hold_is_said_as_it_is_applied() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let listed = |permissions: &str| {
        format!(
            "blueprint: listed\npermissions: {permissions}\nsteps:\n  say: {{ do: log, with: {{ message: hello }} }}\n"
        )
    };
    // Somebody who may apply, and reads the board; no shell, no agents.
    let narrow = key(&rig, &["blueprints", "projects:read", "board:read"]).await;
    let apply = |source: String, token: Option<String>| {
        let rig = &rig;
        async move {
            let asked = json!({ "source": source, "project": "shop", "wait": true });
            let token = token.unwrap_or_else(|| rig.token.clone());
            let (status, applied) = rig.call(Method::POST, "/api/apply", Some(&token), Some(asked)).await;
            assert_eq!(status, StatusCode::OK, "{applied}");
            applied
        }
    };
    // Listed and not held: it is applied, runs without, and says so.
    let applied = apply(listed("[shell, agents, board:read]"), Some(narrow.clone())).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    assert_eq!(
        warned(&applied),
        [(
            "unheld-permission".to_string(),
            "this lists `shell`, `agents` under `permissions`, which narrow does not hold: an instance has no \
             more than whoever applied it, so it runs without, and a step that needs it will fail"
                .to_string()
        )]
    );
    // And as it is checked, by whoever checks it, where the list is written.
    let asked = json!({ "source": listed("[shell, agents, board:read]"), "project": "shop" });
    let (status, checked) = rig.call(Method::POST, "/api/blueprints/check", Some(&narrow), Some(asked)).await;
    assert_eq!((status, &checked["ok"]), (StatusCode::OK, &json!(true)), "{checked}");
    let first = &checked["diagnostics"][0];
    assert_eq!(
        (&first["code"], &first["severity"], &first["span"]["start"]),
        (&json!("unheld-permission"), &json!("warning"), &json!({ "line": 2, "col": 1 })),
        "{checked}"
    );
    // An area listed whole by one who holds a part of it is not said: it
    // has what of the area they hold.
    assert_eq!(warned(&apply(listed("[board, blueprints]"), Some(narrow.clone())).await), []);
    // Nor is anything said to the owner, who holds what is listed.
    assert_eq!(warned(&apply(listed("[shell, agents, board:read]"), None).await), []);
}

#[tokio::test]
async fn a_name_that_is_not_there_is_pointed_at_where_it_is_written_by_the_check_and_by_an_apply() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let source = "\
blueprint: misspelt
params:
  name: { type: string, default: you }
steps:
  first:
    do: log
    with: { message: one }
  say:
    do: log
    with:
      message: \"hello ${params.nme}, after ${steps.frist.ok}\"
";
    let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
    let found: Vec<(u64, u64, &str)> = checked["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|one| one["severity"] == "error")
        .map(|one| {
            let at = &one["span"]["start"];
            (at["line"].as_u64().unwrap(), at["col"].as_u64().unwrap(), one["message"].as_str().unwrap())
        })
        .collect();
    assert_eq!(
        found,
        [
            (11, 25, "there is no param `nme`; did you mean `name`?"),
            (11, 46, "there is no step `frist`; did you mean `first`?"),
        ]
    );
    // An apply refuses it in the same words, with the same places.
    let (status, refused) = rig.post("/api/apply", json!({ "source": source, "project": "shop" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let why = said(&refused);
    assert!(why.contains("misspelt.bp:11:25: error: there is no param `nme`; did you mean `name`?"), "{why}");
    assert!(
        why.contains("misspelt.bp:11:46: error: there is no step `frist`; did you mean `first`?"),
        "{why}"
    );
}

#[tokio::test]
async fn a_handler_on_one_of_the_programs_own_events_misspelt_is_offered_the_one_that_was_meant() {
    let rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A word of the topic misspelt: no family of the program's own takes
    // it, so it is taken for something blueprints say to each other, and
    // nothing says it. What was meant is one of the program's events.
    let source = "\
blueprint: hears
on:
  moved:
    event: bord.item.moved
    do: log
    with: { message: moved }
";
    let checked = rig.send("/api/blueprints/check", json!({ "source": source, "project": "shop" })).await;
    let said: Vec<&str> =
        checked["diagnostics"].as_array().unwrap().iter().filter_map(|one| one["message"].as_str()).collect();
    assert_eq!(
        said,
        [
            "nothing that can be seen here says `bord.item.moved`: no blueprint emits it. If something outside \
          does, say so with `outside: true`; if not, this handler never runs. Did you mean `board.item.moved`?"
        ]
    );
}
