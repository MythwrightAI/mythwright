//! A project's Git page, as the daemon hands it: the branch its folder is
//! on, its branches, the commits of one, what is changed and its remotes,
//! for whoever may read the project; being told that the repository may
//! have changed, a commit made by somebody's own git and the folder made a
//! repository among it; and the parts a package says it shows of a
//! repository it serves, read by the package's own program as whoever
//! asks holds what using it takes. And a repository followed once,
//! whether for its Git page, its branches (a pipeline's trigger) or both.
//!
//! The package is made up, for a service there is not, and its program is
//! the suite's stand-in, told what to print.

use std::time::Duration;

use reqwest::{Method, StatusCode};
use serde_json::{Value, json};

use crate::held::{key, said};
use crate::lanes::git;
use crate::{Rig, STANDIN};

/// A package that serves a repository whose remote is at `example.org`,
/// and shows two parts of it: one its program answers, with a row that
/// says more than is shown and one whose address is no page, and one its
/// program refuses as a program nobody signed in to would.
fn package() -> String {
    let rows = json!([
        { "number": 7, "title": "Cart", "by": "ada", "labels": ["shop", "ui"], "draft": true,
          "url": "https://example.org/acme/shop/pull/7", "secret": "not shown" },
        { "number": 8, "title": "Till", "by": { "login": "bo" }, "url": "javascript:alert(1)" },
    ]);
    // Written in the manifest as a value of its own, in quotes, so that
    // nothing of the JSON is read as the manifest's.
    let said = serde_json::to_string(&rows.to_string()).unwrap();
    format!(
        r##"
extension: example
title: Example Host
description: Not a real service.
program: {program}
actions:
  pulls:
    description: List the open pull requests.
    args: ["say", {said}]
    answer: json
    out:
      rows: ${{ answer }}
    repeatable: true
  checks:
    description: List what was checked.
    args: ["err", "To get started, sign in", "exit", "4"]
    fails:
      - when: ${{ code != 0 }}
        says: "The program is not signed in on this device."
    out:
      rows: ${{ answer }}
    repeatable: true
  quiet:
    description: List what is waiting, and answer nothing.
    args: ["exit", "0"]
    answer: json
    out:
      rows: ${{ answer }}
    repeatable: true
repository:
  serves: ${{ contains(remote.url, "example.org") }}
  parts:
    - name: pulls
      title: Pull requests
      action: pulls
      columns:
        - {{ field: number, title: "#" }}
        - {{ field: title, title: Title }}
        - {{ field: by, title: By }}
        - {{ field: labels }}
        - {{ field: draft, title: Draft }}
      link: url
    - name: checks
      title: Checks
      action: checks
      columns:
        - {{ field: name, title: Check }}
    - name: waiting
      title: Waiting
      action: quiet
      columns:
        - {{ field: name, title: What }}
"##,
        program = STANDIN
    )
}

async fn shop(rig: &Rig) -> std::path::PathBuf {
    let dir = rig.dir.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("README.md"), "# Shop\n").unwrap();
    rig.send("/api/projects", json!({ "name": "Shop", "path": dir })).await;
    dir
}

fn repository_of(projects: &Value) -> &Value {
    let shop = projects["projects"].as_array().unwrap().iter().find(|project| project["id"] == "shop");
    &shop.unwrap()["repository"]
}

#[tokio::test]
async fn a_project_that_is_a_repository_is_said_to_be_one_with_its_branch_commits_changes_and_remotes() {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    let from = rig.mark();

    // No repository yet: said so, and nothing of one handed out.
    assert_eq!(repository_of(&rig.read("/api/projects").await), &json!(false));
    let none = rig.read("/api/projects/shop/git").await;
    assert_eq!(none["repository"], json!(false), "{none}");
    assert!(none["said"].as_str().unwrap().contains("no git repository"), "{none}");

    // Made one by somebody's own git, which is heard. What is only passed
    // on is heard by whoever follows as it is said, so the test follows first.
    let mut heard = rig.follow_after(from, "project.git.changed");
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    let made = tokio::time::timeout(Duration::from_secs(30), heard.next()).await;
    assert_eq!(made.expect("the folder made a repository is heard").unwrap().data["project"], "shop");
    assert_eq!(repository_of(&rig.read("/api/projects").await), &json!(true));

    // Nothing committed: on its branch, with no commit.
    let empty = rig.read("/api/projects/shop/git").await;
    assert_eq!((&empty["head"]["branch"], &empty["commits"]), (&json!("main"), &json!([])), "{empty}");

    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "The first"]);
    git(&dir, &["branch", "cart"]);
    git(&dir, &["checkout", "--quiet", "cart"]);
    std::fs::write(dir.join("cart.rs"), "fn main() {}\n").unwrap();
    git(&dir, &["add", "cart.rs"]);
    git(&dir, &["commit", "--quiet", "-m", "A cart"]);
    git(&dir, &["checkout", "--quiet", "main"]);
    std::fs::write(dir.join("README.md"), "# Shop, changed\n").unwrap();
    std::fs::write(dir.join("notes.txt"), "to do\n").unwrap();
    git(&dir, &["remote", "add", "origin", "https://ada:secret@example.org/acme/shop.git"]);

    let seen = rig.read("/api/projects/shop/git").await;
    assert_eq!(seen["head"]["branch"], "main", "{seen}");
    assert_eq!(seen["of"], "HEAD");
    let subjects = |seen: &Value| -> Vec<String> {
        seen["commits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|one| one["subject"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(subjects(&seen), ["The first"]);
    assert_eq!(seen["commits"][0]["author"], "Ada");
    let branches: Vec<&str> =
        seen["branches"].as_array().unwrap().iter().map(|one| one["name"].as_str().unwrap()).collect();
    assert_eq!(branches, ["cart", "main"]);
    let changes: Vec<(&str, &str, &str)> = seen["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| {
            (
                one["path"].as_str().unwrap(),
                one["staged"].as_str().unwrap(),
                one["unstaged"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(changes, [("README.md", "", "modified"), ("notes.txt", "", "untracked")]);
    assert_eq!(
        seen["remotes"],
        json!([{ "name": "origin", "fetch": "https://example.org/acme/shop.git" }]),
        "no secret written into an address is handed on"
    );
    assert!(!seen.to_string().contains("secret@"), "{seen}");

    // Another branch's commits, asked for whole as it is listed; one that is not there refused, so that nothing handed in reaches git as anything else.
    let cart = rig.read("/api/projects/shop/git?branch=refs/heads/cart").await;
    assert_eq!(
        (cart["of"].as_str(), subjects(&cart)),
        (Some("refs/heads/cart"), vec!["A cart".to_string(), "The first".to_string()])
    );
    for asked in ["refs/heads/nothing", "--output=x", "cart"] {
        let (status, refused) = rig.get(&format!("/api/projects/shop/git?branch={asked}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{asked}: {refused}");
        assert!(said(&refused).contains("has no branch"), "{refused}");
    }

    // A commit is heard as the system says the repository changed.
    let from = rig.mark();
    let mut heard = rig.follow_after(from, "project.git.changed");
    git(&dir, &["commit", "--quiet", "-am", "Changed"]);
    let changed = tokio::time::timeout(Duration::from_secs(30), heard.next()).await;
    assert_eq!(changed.expect("a commit is heard").unwrap().data["project"], "shop");
    assert_eq!(subjects(&rig.read("/api/projects/shop/git").await)[0], "Changed");

    // For whoever may read the project, and nobody else.
    let narrow = key(&rig, &["board:read"]).await;
    let (status, refused) = rig.call(Method::GET, "/api/projects/shop/git", Some(&narrow), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    let (status, _) = rig.get("/api/projects/nothing/git").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_package_that_serves_a_repositorys_remote_shows_its_parts_as_whoever_asks_holds_what_using_it_takes()
 {
    let rig = Rig::new().await;
    let dir = shop(&rig).await;
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    rig.send("/api/extensions", json!({ "source": package() })).await;

    // A remote it does not serve: nothing of it offered.
    git(&dir, &["remote", "add", "origin", "https://elsewhere.net/acme/shop.git"]);
    assert_eq!(rig.read("/api/projects/shop/git").await["shows"], json!([]));

    git(&dir, &["remote", "add", "hosted", "https://example.org/acme/shop.git"]);
    let seen = rig.read("/api/projects/shop/git").await;
    let shows = &seen["shows"];
    assert_eq!(shows.as_array().unwrap().len(), 1, "{seen}");
    assert_eq!((&shows[0]["package"], &shows[0]["title"]), (&json!("example"), &json!("Example Host")));
    let parts: Vec<&str> =
        shows[0]["parts"].as_array().unwrap().iter().map(|part| part["name"].as_str().unwrap()).collect();
    assert_eq!(parts, ["pulls", "checks", "waiting"]);
    assert_eq!(
        shows[0]["parts"][0]["columns"][3],
        json!({ "field": "labels", "title": "labels" }),
        "a heading left out is the field's name"
    );
    assert!(shows[0]["refused"].is_null());

    // Read: each row as the columns say and no more, a list joined, an address only where it opens a page.
    let read = rig.read("/api/projects/shop/git/example/pulls").await;
    assert_eq!(
        read["rows"],
        json!([
            { "number": 7, "title": "Cart", "by": "ada", "labels": "shop, ui", "draft": "yes", "url": "https://example.org/acme/shop/pull/7" },
            { "number": 8, "title": "Till", "by": "{\"login\":\"bo\"}", "labels": "", "draft": "" },
        ]),
        "{read}"
    );
    assert_eq!(read["more"], 0);

    // What the program says of itself is said in the package's own sentence.
    let (status, failed) = rig.get("/api/projects/shop/git/example/checks").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{failed}");
    assert_eq!(said(&failed), "The program is not signed in on this device.");
    // A program that answered nothing has not said there is nothing: the part says so, never as no rows.
    let (status, quiet) = rig.get("/api/projects/shop/git/example/waiting").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{quiet}");
    assert!(said(&quiet).contains("answered nothing for Waiting"), "{quiet}");
    let (status, _) = rig.get("/api/projects/shop/git/example/nothing").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Somebody who may read the project and not use the package is told why, and its program is not run for them.
    let narrow = key(&rig, &["projects:read"]).await;
    let (_, seen) = rig.call(Method::GET, "/api/projects/shop/git", Some(&narrow), None).await;
    assert!(seen["shows"][0]["refused"].as_str().unwrap().contains("extensions:example"), "{seen}");
    let (status, refused) =
        rig.call(Method::GET, "/api/projects/shop/git/example/pulls", Some(&narrow), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
}

#[tokio::test]
async fn a_package_that_says_it_shows_a_repository_wrongly_is_refused_saying_where() {
    let rig = Rig::new().await;
    let source = format!(
        "extension: wrong\nprogram: {STANDIN}\nactions:\n  needs:\n    description: Needs a number.\n    with:\n      \
         n: {{ type: int, required: true, description: \"A number.\" }}\n    args: [\"say\", \"[]\"]\n    out:\n      \
         said: ${{ answer }}\nrepository:\n  serves: ${{ contains(daemon.url, \"x\") }}\n  parts:\n    - name: one\n      \
         title: One\n      action: needs\n      columns: []\n    - name: two\n      title: Two\n      action: missing\n      \
         columns: [{{ field: x }}]\n"
    );
    let (status, refused) = rig.post("/api/extensions", json!({ "source": source })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let said = said(&refused);
    for wrong in [
        "`repository.serves` reads only `remote.name` and `remote.url`",
        "`repository.parts[0].action` names `needs`, which needs `n`",
        "`repository.parts[0].action` names `needs`, which gives out no `rows`",
        "`repository.parts[0].columns` is needed",
        "`repository.parts[1].action` names `missing`, which is not an action of this package",
    ] {
        assert!(said.contains(wrong), "{wrong}: {said}");
    }
}

#[tokio::test]
async fn a_projects_repository_is_followed_once_for_its_git_page_and_its_branches_together() {
    let rig = Rig::new().await;
    let dir = rig.dir.join("shop");
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "--quiet", "-b", "main", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "first"]);
    crate::pipelines::shop(&rig, Some(&dir)).await;
    // Its branches followed, for a row that runs a pipeline as one moves.
    let before = rig.mark();
    crate::pipelines::row(&rig, json!({ "pipeline": "pipelines/gates", "on": "branch", "branch": "main" }))
        .await;
    let gone_by = |event: &kernel::Event| {
        event.seq > before && event.data["blueprint"] == "std/pipeline" && event.data["flow"] == "on.changed"
    };
    rig.event("run.finished", gone_by).await;
    // And its Git page looked at: what follows it begins again, saying it may have changed.
    let from = rig.mark();
    let mut heard = rig.follow_after(from, "project.git.changed");
    rig.read("/api/projects/shop/git").await;
    let began = tokio::time::timeout(Duration::from_secs(30), heard.next()).await;
    assert_eq!(began.expect("following it again is heard").unwrap().data["project"], "shop");
    let followers = rig.node().app.hosting.watch().followers(&dir.join(".git"));
    assert_eq!(followers, 1, "one follower for the Git page and the branches together");

    // And it says both: a commit is a change of the page and a branch that moved.
    let mark = rig.mark();
    let mut heard = rig.follow_after(mark, "project.git.changed");
    git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "second"]);
    let commit = git(&dir, &["rev-parse", "HEAD"]);
    rig.event("project.branch.moved", |event| event.seq > mark && event.data["to"] == commit.as_str()).await;
    let changed = tokio::time::timeout(Duration::from_secs(30), heard.next()).await;
    assert_eq!(changed.expect("a commit is heard").unwrap().data["project"], "shop");
    assert_eq!(rig.node().app.hosting.watch().followers(&dir.join(".git")), 1);
}
