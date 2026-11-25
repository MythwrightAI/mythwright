//! The fleet that ships (`std/fleet`): workers do cards, reviewers pass
//! each or send it back, what passes is merged and done. Who is in it and
//! what they do with a card are two tables of the project, and a row of
//! either can be changed while work is under way. Played with stand-ins
//! for the agents. Nothing the factory is built from knows a worker from
//! a reviewer: it is all that blueprint, `std/arrangement`, the stages
//! (`std/working`, `std/reviewing`),
//! `std/role`, `std/lane-round` and `std/review`.

use std::path::{Path, PathBuf};

use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::{git, history, move_card, moved, said};

/// A node with two programs to run agents on, and the folder they read
/// what to do from.
///
/// `writer` adds a line to a file where it is run and says so. `critic`
/// approves, unless a file `says-<its name>` there says otherwise, in
/// which case it says that. Where a test has made a hold `hold-<its
/// name>`, or failing that `hold`, a critic waits at it until it is let
/// go: a review is then in session for as long as the test wants. One
/// hold lets go everyone waiting at it, so reviewers that are to be let
/// go one at a time each have their own.
pub(crate) async fn rig() -> (Rig, PathBuf) {
    rig_told(|_| {}).await
}

/// The same, with something more said of the node.
pub(crate) async fn rig_told(more: impl FnOnce(&mut api::Config)) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-fleets", |config, dir| {
        let file = |name: &str| dir.join(name).to_string_lossy().into_owned();
        // Who it is, as its program is told: read by the program itself.
        let name = format!("%{}%", brand::env_var("SLOT"));
        let writes = ["swallow", "append", "CHANGES.md", "changed", "say", "I changed a file."];
        let holds = format!("{}|{}", file(&format!("hold-{name}")), file("hold"));
        let says = file(&format!("says-{name}"));
        let looks = ["swallow", "held", &holds, &name, "says", &says];
        let approves = ["say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&[&looks[..], &approves[..]].concat())));
        at = Some(dir.to_path_buf());
        more(config);
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// Has a reviewer say this, whenever it is asked from now on.
pub(crate) fn says(at: &Path, reviewer: &str, words: &str, decision: &str) {
    std::fs::write(at.join(format!("says-{reviewer}")), format!("{words}\nDECISION: {decision}\n")).unwrap();
}

/// A hold for the reviewers to wait at, and where it is.
fn hold(at: &Path) -> PathBuf {
    held(at, "hold")
}

/// A hold for one reviewer to wait at, and where it is.
fn hold_one(at: &Path, reviewer: &str) -> PathBuf {
    held(at, &format!("hold-{reviewer}"))
}

fn held(at: &Path, name: &str) -> PathBuf {
    let hold = at.join(name);
    crate::hold(&hold);
    hold
}

/// Lets whoever is waiting at the hold go on.
async fn let_go(hold: &Path) {
    crate::let_go("a reviewer to be waiting", hold).await;
}

/// What the fleet's two tables start with, for these choices: so many
/// who work and so many who review, each run by its stand-in, and how a
/// card is reviewed. One worker and one reviewer, where nothing is said.
pub(crate) fn params(more: Value) -> Value {
    let said = |name: &str, usual: Value| more.get(name).cloned().unwrap_or(usual);
    let agents = json!([
        { "title": "worker", "role": "worker", "count": said("workers", json!(1)), "adapter": "writer" },
        {
            "title": "reviewer", "role": "reviewer", "count": said("reviewers", json!(1)),
            "adapter": "critic", "permission": "read",
        },
    ]);
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        {
            "title": "review", "column": "review", "does": "review", "role": "reviewer",
            "seats": said("seats", json!(0)), "needed": said("needed", json!(0)),
            "pass": "done", "fail": "doing", "rounds": said("rounds", json!(2)), "merge": true,
            "in_flight": said("in_flight", json!("finish")),
        },
    ]);
    // A test that wants other rows says the whole table.
    json!({
        "project": "shop", "agents": said("agents", agents), "stages": said("stages", stages),
        "push": said("push", json!(false)), "key": said("key", json!("")),
    })
}

/// The fleet applied to `shop`, its tables starting as these choices say:
/// what the daemon answered.
pub(crate) async fn fleet(rig: &Rig, more: Value) -> Value {
    let request =
        json!({ "blueprint": "std/fleet", "project": "shop", "params": params(more), "wait": true });
    let applied = rig.send("/api/apply", request).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    applied
}

/// The row of one of the fleet's tables that is called this: its id.
pub(crate) async fn row(rig: &Rig, table: &str, title: &str) -> String {
    let rows = rig.read(&format!("/api/collections/shop/{table}/records")).await;
    let found = rows["records"].as_array().unwrap().iter().find(|row| row["data"]["title"] == title);
    found.unwrap_or_else(|| panic!("no row of {table} is called {title}: {rows}"))["id"]
        .as_str()
        .unwrap()
        .to_string()
}

/// Adds a row to one of the fleet's tables, and waits for the fleet to
/// have taken it: what it then said of itself.
pub(crate) async fn add_row(rig: &Rig, table: &str, fields: Value) -> Value {
    let before = rig.mark();
    rig.send(&format!("/api/collections/shop/{table}/records"), json!({ "fields": fields })).await;
    crate::taken::by_fleet(rig, before).await
}

/// Takes a row of one of the fleet's tables away, and waits likewise.
pub(crate) async fn remove_row(rig: &Rig, table: &str, title: &str) {
    let id = row(rig, table, title).await;
    let before = rig.mark();
    let (status, answer) =
        rig.call(Method::DELETE, &format!("/api/records/{id}"), Some(&rig.token), None).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    crate::taken::by_fleet(rig, before).await;
}

/// Changes a row of one of the fleet's tables, as someone would in the
/// client, and waits for the fleet to have taken it.
pub(crate) async fn change(rig: &Rig, table: &str, title: &str, fields: Value) {
    let id = row(rig, table, title).await;
    let before = rig.mark();
    let (status, answer) = rig
        .call(
            Method::PATCH,
            &format!("/api/records/{id}"),
            Some(&rig.token),
            Some(json!({ "fields": fields })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    crate::taken::by_fleet(rig, before).await;
}

/// A `shop` project that is a repository, with a board and the fleet: the
/// project's folder.
pub(crate) async fn shop(rig: &Rig, more: Value) -> PathBuf {
    shop_with(rig, &["draft", "ready", "doing", "review", "done"], more).await
}

/// The same, on a board with these columns.
pub(crate) async fn shop_with(rig: &Rig, columns: &[&str], more: Value) -> PathBuf {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = PathBuf::from(project["path"].as_str().unwrap());
    git(&dir, &["init", "--quiet", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    let board = json!({
        "blueprint": "std/board", "project": "shop", "params": { "columns": columns }, "wait": true,
    });
    let applied = rig.send("/api/apply", board).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    fleet(rig, more).await;
    dir
}

pub(crate) async fn add(rig: &Rig, title: &str, after: &[&str]) -> String {
    let card = json!({ "title": title, "fields": { "after": after } });
    rig.send("/api/board/shop/items", card).await["id"].as_str().unwrap().to_string()
}

pub(crate) async fn card(rig: &Rig, id: &str) -> Value {
    let board = rig.read("/api/board/shop").await;
    board["items"].as_array().unwrap().iter().find(|card| card["id"] == id).cloned().unwrap()
}

/// What an agent was given, oldest first: each prompt.
pub(crate) async fn given(rig: &Rig, agent: &str) -> Vec<String> {
    let mut turns =
        rig.read(&format!("/api/agents/{agent}/turns")).await["turns"].as_array().unwrap().clone();
    turns.reverse();
    turns.iter().map(|turn| turn["prompt"].as_str().unwrap().to_string()).collect()
}

/// Each agent of a role that takes work, by name.
pub(crate) async fn active(rig: &Rig, role: &str) -> Vec<String> {
    let all = rig.read("/api/agents").await;
    let mut names: Vec<String> = all["agents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|agent| agent["role"] == role && agent["active"] == true)
        .map(|agent| agent["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn a_card_goes_between_a_worker_and_a_reviewer_and_passed_is_merged_and_done() {
    let (rig, at) = rig().await;
    let dir = shop(&rig, json!({})).await;
    // The roles are there, brought up by the fleet: nobody was added by hand.
    assert_eq!(
        (active(&rig, "worker").await, active(&rig, "reviewer").await),
        (vec!["shop-worker-1".to_string()], vec!["shop-reviewer-1".to_string()])
    );

    // A reviewer that finds something wrong, until it is told otherwise.
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let back = moved(&rig, &id, "doing", 0).await;
    let back = moved(&rig, &id, "doing", back).await;
    std::fs::remove_file(at.join("says-shop-reviewer-1")).unwrap();
    moved(&rig, &id, "done", back).await;

    // The worker did it, was told what the reviewer found, and did it again.
    let worked = given(&rig, "shop-worker-1").await;
    assert_eq!(worked.len(), 2, "{worked:?}");
    assert!(
        worked[1].starts_with("This card was sent back, for you to take up again")
            && worked[1].contains("shop-reviewer-1: The total is wrong."),
        "{}",
        worked[1]
    );
    // The reviewer looked twice, where the card was worked, and was told to change nothing.
    let looked = given(&rig, "shop-reviewer-1").await;
    assert_eq!(looked.len(), 2, "{looked:?}");
    assert!(
        looked[0].starts_with(&format!("Card {id}: Add a cart"))
            && looked[0].contains(&format!("on its own branch, lane/{id}"))
            && looked[0].contains("Change nothing yourself.")
            // It is shown what was changed, and need not be able to ask git.
            && looked[0].contains("What was changed, against the project's own branch:")
            && looked[0].contains("CHANGES.md | 1 +")
            && !looked[0].contains("You are one of"),
        "{}",
        looked[0]
    );

    // It is done, and its history says who said what.
    let said = history(
        &rig,
        &id,
        &format!(
            "conductor moved it from review to done, saying: approved by shop-reviewer-1; lane/{id} was merged"
        ),
    )
    .await;
    for line in [
        "conductor moved it from review to doing, saying: shop-reviewer-1: The total is wrong.",
        "shop-reviewer-1 approved it: It does what the card asks.",
    ] {
        assert!(said.contains(&line.to_string()), "{said:?}");
    }
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1);
    // And its work is in the project's own branch, as a merge that names the card.
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\nchanged\n");
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"));
    // The stage that worked it tidied up after it: the worktree is gone, and the branch with it.
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.tidied"
    })
    .await;
    assert!(!dir.join(".lanes").join(&id).exists());
    assert_eq!(git(&dir, &["branch", "--list", &format!("lane/{id}")]), "");
}

#[tokio::test]
async fn those_who_review_are_a_council_that_passes_what_most_approve_each_looking_for_itself() {
    let (rig, at) = rig().await;
    // Three who review, and nothing said of how many look at a card: all of them do.
    shop(&rig, json!({ "reviewers": 3 })).await;
    assert_eq!(active(&rig, "reviewer").await, ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"]);
    says(&at, "shop-reviewer-2", "I would have named it otherwise.", "send_back");

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let passed = rig.event("review.passed", |_| true).await;
    assert_eq!(
        (&passed.data["card"], &passed.data["approved"], &passed.data["seats"]),
        (&json!(id), &json!(2), &json!(3))
    );
    let mut by: Vec<&str> = passed.data["by"].as_str().unwrap().split(", ").collect();
    by.sort();
    assert_eq!(by, ["shop-reviewer-1", "shop-reviewer-3"]);
    moved(&rig, &id, "done", 0).await;
    // Each of the three was asked once at the most, each told it is one of three.
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"] {
        let looked = given(&rig, reviewer).await;
        assert!(looked.len() <= 1, "{reviewer}: {looked:?}");
        assert!(looked.iter().all(|prompt| prompt.contains("You are one of 3 looking at it")), "{looked:?}");
    }
    let said = history(
        &rig,
        &id,
        &format!(
            "conductor moved it from review to done, saying: approved by {}, 2 of 3; lane/{id} was merged",
            passed.data["by"].as_str().unwrap()
        ),
    )
    .await;
    assert!(
        said.iter().any(|line| line.ends_with("approved it: ") || line.contains("approved it")),
        "{said:?}"
    );
}

#[tokio::test]
async fn a_council_that_can_no_longer_pass_a_card_sends_it_back_with_what_each_said_is_wrong() {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "reviewers": 3, "rounds": 1 })).await;
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    says(&at, "shop-reviewer-3", "Nothing tests it.", "send_back");

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let failed = rig.event("review.failed", |_| true).await;
    assert_eq!((&failed.data["against"], &failed.data["seats"]), (&json!(2), &json!(3)));
    // Back with its worker, who is told what both said.
    let back = moved(&rig, &id, "doing", 0).await;
    let back = moved(&rig, &id, "doing", back).await;
    let again = moved(&rig, &id, "review", back).await;
    let worked = given(&rig, "shop-worker-1").await;
    assert!(
        worked[1].contains("shop-reviewer-1: The total is wrong.")
            && worked[1].contains("shop-reviewer-3: Nothing tests it."),
        "{}",
        worked[1]
    );
    // Sent back as often as is allowed, it is left for a person, and nobody is asked again.
    let told = rig
        .event("channel.message", move |event| {
            event.seq > again && event.data["body"].as_str().unwrap_or_default().contains("left for a person")
        })
        .await;
    assert!(told.data["body"].as_str().unwrap().starts_with(&format!("{id} is in review again")), "{told:?}");
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(rig.count_of("review.passed").await + rig.count_of("review.failed").await, 1);
}

#[tokio::test]
async fn a_card_that_waits_on_another_is_begun_when_that_one_is_done_and_from_its_work() {
    let (rig, at) = rig().await;
    let pipe = hold(&at);
    let dir = shop(&rig, json!({ "workers": 2 })).await;
    let first = add(&rig, "Lay the floor", &[]).await;
    let second = add(&rig, "Put the shelves up", &[first.as_str()]).await;
    // Both made ready, the second first: there are two workers, and it still waits.
    move_card(&rig, &second, "ready", None).await;
    let waiting = rig.event("card.waiting", |_| true).await;
    assert_eq!(
        waiting.data,
        json!({ "card": second, "project": "shop", "what": format!("after {first}, not done yet") })
    );
    assert_eq!(card(&rig, &second).await["more"]["waiting"], format!("after {first}, not done yet"));
    move_card(&rig, &first, "ready", None).await;
    // The first is worked and is in review, with a reviewer looking at it;
    // the second has not been begun, and has no place to work in yet.
    moved(&rig, &first, "review", 0).await;
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    assert_eq!(card(&rig, &second).await["status"], "ready");
    assert!(!dir.join(".lanes").join(&second).exists());
    assert_eq!(given(&rig, "shop-worker-1").await.len() + given(&rig, "shop-worker-2").await.len(), 1);

    // Passed, merged and done, the second is freed, and is worked from
    // what the first left: the first's line is there before its own.
    let_go(&pipe).await;
    moved(&rig, &first, "done", 0).await;
    moved(&rig, &second, "review", 0).await;
    assert_eq!(
        std::fs::read_to_string(dir.join(".lanes").join(&second).join("CHANGES.md")).unwrap(),
        "changed\nchanged\n"
    );
    let said = history(&rig, &second, "what it waited on is done").await;
    assert!(said.contains(&format!("it waits its turn: after {first}, not done yet")), "{said:?}");
    assert_eq!(card(&rig, &second).await["more"]["waiting"], "");
    let_go(&pipe).await;
    moved(&rig, &second, "done", 0).await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\nchanged\n");
}

#[tokio::test]
async fn with_fewer_to_review_a_council_in_session_finishes_as_it_began_and_the_next_card_is_the_new_way() {
    let (rig, at) = rig().await;
    let pipes =
        ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"].map(|reviewer| hold_one(&at, reviewer));
    shop(&rig, json!({ "workers": 2, "reviewers": 3 })).await;
    let first = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &first, "ready", None).await;
    // The council is in session: all three are looking.
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"] {
        rig.event("agent.turn.started", move |event| event.data["slot"] == reviewer).await;
    }

    // One reviewer from now on: the row that says how many is changed, as
    // someone would in the client. What is under way is finished as it was begun.
    change(&rig, "agents", "reviewer", json!({ "count": 1 })).await;
    let told = rig.event("review.arranged", |_| true).await;
    assert_eq!(
        told.data,
        json!({ "project": "shop", "stage": "review", "arranged": "std/review/1/0", "in_flight": "finish" })
    );
    // The two no longer counted are still there, finishing what they have
    // in hand, and are given nothing more.
    let agents = rig.read("/api/agents").await;
    let leaving: Vec<(&str, bool, bool)> = agents["agents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|agent| agent["role"] == "reviewer")
        .map(|agent| (agent["name"].as_str().unwrap(), agent["active"] == true, agent["leaving"] == true))
        .collect();
    assert_eq!(
        leaving,
        [("shop-reviewer-1", true, false), ("shop-reviewer-2", true, true), ("shop-reviewer-3", true, true)],
        "{agents}"
    );

    // The council reaches its decision as a council: two of three, the
    // two who are leaving among them.
    let_go(&pipes[1]).await;
    let_go(&pipes[2]).await;
    let passed = rig.event("review.passed", |_| true).await;
    assert_eq!(
        (&passed.data["card"], &passed.data["seats"], &passed.data["approved"]),
        (&json!(first), &json!(3), &json!(2))
    );
    moved(&rig, &first, "done", 0).await;
    // With nothing left in hand, the two are gone.
    for reviewer in ["shop-reviewer-2", "shop-reviewer-3"] {
        rig.event("agent.slot.stopped", move |event| event.data["slot"] == reviewer).await;
    }
    assert_eq!(active(&rig, "reviewer").await, ["shop-reviewer-1"]);

    // The next card to reach review is looked at by one, and passes on its word.
    let second = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &second, "ready", None).await;
    let asked = second.clone();
    let due = rig.event("review.due", move |event| event.data["card"] == asked.as_str()).await.seq;
    rig.event("agent.turn.started", move |event| event.seq > due && event.data["slot"] == "shop-reviewer-1")
        .await;
    let_go(&pipes[0]).await;
    let wanted = second.clone();
    let passed = rig.event("review.passed", move |event| event.data["card"] == wanted.as_str()).await;
    assert_eq!((&passed.data["seats"], &passed.data["by"]), (&json!(1), &json!("shop-reviewer-1")));
}

#[tokio::test]
async fn with_fewer_to_review_and_told_to_begin_again_a_review_in_session_is_stopped_and_begun_the_new_way() {
    let (rig, at) = rig().await;
    let pipe = hold(&at);
    shop(&rig, json!({ "reviewers": 3, "in_flight": "restart" })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // The council is in session: all three are looking.
    for reviewer in ["shop-reviewer-1", "shop-reviewer-2", "shop-reviewer-3"] {
        rig.event("agent.turn.started", move |event| event.data["slot"] == reviewer).await;
    }
    let before = rig.mark();

    // One reviewer from now on, where the stage says that what is under way begins again.
    change(&rig, "agents", "reviewer", json!({ "count": 1 })).await;
    // The council is stopped where it is: its three are no longer looking,
    // and the two no longer counted, having nothing left in hand, are gone.
    let stopped =
        rig.event("agent.turn.finished", move |event| event.seq > before && event.data["ok"] == false).await;
    assert_eq!(stopped.data["error"], "the turn was cancelled", "{stopped:?}");
    // Which is said as what it is: called off, and not the agent failing at it.
    assert_eq!(stopped.data["called_off"], true, "{stopped:?}");
    for reviewer in ["shop-reviewer-2", "shop-reviewer-3"] {
        rig.event("agent.slot.stopped", move |event| event.data["slot"] == reviewer).await;
    }
    assert_eq!(active(&rig, "reviewer").await, ["shop-reviewer-1"]);
    // The card is reviewed again, by one, and passes on that one's word.
    rig.event("agent.turn.started", move |event| {
        event.seq > before && event.data["slot"] == "shop-reviewer-1"
    })
    .await;
    let_go(&pipe).await;
    let passed = rig.event("review.passed", |_| true).await;
    assert_eq!(
        (&passed.data["card"], &passed.data["seats"], &passed.data["by"]),
        (&json!(id), &json!(1), &json!("shop-reviewer-1"))
    );
    moved(&rig, &id, "done", 0).await;
    let decided = rig.count_of("review.passed").await
        + rig.count_of("review.failed").await
        + rig.count_of("review.undecided").await;
    assert_eq!(decided, 1, "the council that was stopped decided nothing");
}

#[tokio::test]
async fn a_reviewer_that_neither_approves_nor_sends_back_has_decided_nothing_and_the_card_is_left_for_a_person()
 {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    std::fs::write(at.join("says-shop-reviewer-1"), "I am not sure what to make of this.\n").unwrap();
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;

    let undecided = rig.event("review.undecided", |_| true).await;
    assert_eq!(undecided.data["words"], "shop-reviewer-1: I am not sure what to make of this.");
    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("neither approved it nor sent it back")
        })
        .await;
    assert!(told.data["body"].as_str().unwrap().ends_with("It is left in review for a person."), "{told:?}");
    // Nothing was taken for approval, and nothing was sent back with no reason.
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(rig.count_of("card.approved").await, 0);
}

#[tokio::test]
async fn a_branch_that_no_longer_merges_goes_back_to_its_worker_and_the_projects_branch_is_as_it_was() {
    let (rig, at) = rig().await;
    let pipe = hold(&at);
    let dir = shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // While a reviewer looks at it, the same file is changed in the project's own branch.
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    std::fs::write(dir.join("CHANGES.md"), "something else\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "meanwhile"]);
    let_go(&pipe).await;

    // Approved, it does not merge: the merge is called off, and the card
    // goes back to whoever worked it, who is told why.
    let back = rig
        .event("board.item.moved", |event| event.data["from"] == "review" && event.data["to"] == "doing")
        .await;
    let note = back.data["note"].as_str().unwrap();
    assert!(
        note.starts_with(&format!("It was approved, and its branch lane/{id} no longer merges"))
            && note.contains("Merge the project's branch into yours"),
        "{note}"
    );
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "meanwhile");
    assert_eq!(
        git(&dir, &["status", "--porcelain", "--untracked-files=no"]),
        "",
        "no merge was left under way"
    );
    let again = moved(&rig, &id, "review", back.seq).await;
    let worked = given(&rig, "shop-worker-1").await;
    assert!(worked[1].contains("no longer merges into the project's branch"), "{}", worked[1]);
    // The stand-in worker did not put it right, so it is the same again; what matters is that nothing was forced.
    let_go(&pipe).await;
    rig.event("board.item.moved", move |event| event.seq > again && event.data["from"] == "review").await;
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "something else\n");
}

#[tokio::test]
async fn a_merge_somebody_has_under_way_in_the_projects_folder_is_left_to_them_and_the_card_waits() {
    let (rig, at) = rig().await;
    let pipe = hold(&at);
    let dir = shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // While a reviewer looks at it, somebody merges a branch of their own
    // in the project's folder, and that stops on a conflict they have yet
    // to put right.
    rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await;
    let here = git(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let wrote = |words: &str, said: &str| {
        std::fs::write(dir.join("NOTES.md"), words).unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "--quiet", "-m", said]);
    };
    git(&dir, &["checkout", "--quiet", "-b", "theirs"]);
    wrote("one way\n", "theirs");
    git(&dir, &["checkout", "--quiet", &here]);
    wrote("another way\n", "ours");
    let stopped = crate::lanes::asking(&dir).args(["merge", "theirs"]).output().unwrap();
    assert!(!stopped.status.success(), "their merge was to stop on a conflict");
    let under_way = git(&dir, &["rev-parse", "--verify", "MERGE_HEAD"]);
    let_go(&pipe).await;

    // Approved, the card's branch is not merged over theirs, and theirs is
    // not called off to make room: it is said, and the card stays.
    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("could not be merged into the project's")
        })
        .await;
    let body = told.data["body"].as_str().unwrap();
    assert!(body.contains("a merge is already under way in the project's folder"), "{body}");
    assert!(
        body.ends_with("It stays in review; when what is in the way is put right, merge it and move it on.")
    );
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(
        git(&dir, &["rev-parse", "--verify", "MERGE_HEAD"]),
        under_way,
        "their merge is as they left it"
    );
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), "ours");
}

#[tokio::test]
async fn a_role_made_of_several_rows_is_one_council_and_a_row_added_while_it_runs_joins_it() {
    let (rig, at) = rig().await;
    shop(&rig, json!({})).await;
    // A second who reviews, as a row of its own; the fleet then says what it is, and the change it took.
    let mut arranged =
        add_row(&rig, "agents", json!({ "title": "second", "role": "reviewer", "adapter": "critic" })).await;
    assert!(arranged["asked"].take().as_str().is_some_and(|asked| asked.starts_with("row ")), "{arranged}");
    assert_eq!(arranged, json!({ "project": "shop", "agents": 3, "stages": 2, "asked": null }));
    assert_eq!(active(&rig, "reviewer").await, ["shop-reviewer-1", "shop-second-1"]);

    // Both look at a card, each for itself, and one of two is not enough.
    // Which of two who look at once speaks first is nobody's to say, and
    // a review is decided as soon as it can be: so the second waits at a
    // pipe of its own until the first has answered, and is the one whose
    // word decides it. Of two that have answered by the time the review
    // looks, the one of the lower seat is counted first.
    says(&at, "shop-second-1", "Nothing tests it.", "send_back");
    let second = hold_one(&at, "shop-second-1");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    rig.event("agent.turn.finished", |event| event.data["slot"] == "shop-reviewer-1").await;
    let_go(&second).await;
    let failed = rig.event("review.failed", |_| true).await;
    assert_eq!(
        (&failed.data["by"], &failed.data["approved"], &failed.data["seats"]),
        (&json!("shop-second-1"), &json!(1), &json!(2))
    );
    for reviewer in ["shop-reviewer-1", "shop-second-1"] {
        let looked = given(&rig, reviewer).await;
        assert!(looked[0].contains("You are one of 2 looking at it"), "{reviewer}: {looked:?}");
    }
}

#[tokio::test]
async fn a_stage_added_to_the_line_has_its_role_look_at_each_card_before_it_is_done() {
    let (rig, at) = rig().await;
    // A board with a column between review and done, and a fleet whose
    // tables say what happens there: those who test look at what passed review.
    let agents = json!([
        { "title": "worker", "role": "worker", "adapter": "writer" },
        { "title": "reviewer", "role": "reviewer", "adapter": "critic", "permission": "read" },
        { "title": "tester", "adapter": "critic", "permission": "read" },
    ]);
    let stages = json!([
        { "title": "work", "column": "ready", "does": "work", "role": "worker", "pass": "review" },
        { "title": "review", "does": "review", "role": "reviewer", "pass": "test" },
        { "title": "test", "role": "tester", "pass": "done", "merge": true },
    ]);
    let columns = ["draft", "ready", "doing", "review", "test", "done"];
    let dir = shop_with(&rig, &columns, json!({ "agents": agents, "stages": stages })).await;
    // A row that says no role is of the one it is called after, and no count is one.
    assert_eq!(active(&rig, "tester").await, ["shop-tester-1"]);

    says(&at, "shop-tester-1", "It does not start.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // Worked, passed in review, and sent back from test to whoever worked it.
    let tested = moved(&rig, &id, "test", 0).await;
    let back = moved(&rig, &id, "doing", tested).await;
    std::fs::remove_file(at.join("says-shop-tester-1")).unwrap();
    moved(&rig, &id, "done", back).await;

    let worked = given(&rig, "shop-worker-1").await;
    assert!(worked[1].contains("shop-tester-1: It does not start."), "{}", worked[1]);
    let said = history(
        &rig,
        &id,
        &format!(
            "conductor moved it from test to done, saying: approved by shop-tester-1; lane/{id} was merged"
        ),
    )
    .await;
    for line in [
        "conductor moved it from review to test, saying: approved by shop-reviewer-1",
        "conductor moved it from test to doing, saying: shop-tester-1: It does not start.",
    ] {
        assert!(said.contains(&line.to_string()), "{said:?}");
    }
    // Sent back from test is sent back: the card went round once.
    assert_eq!(card(&rig, &id).await["more"]["rounds"], 1);
    // Merged where the stage that says so passed it, and not before.
    assert_eq!(git(&dir, &["log", "-1", "--format=%s"]), format!("{id}: Add a cart"));
}

#[tokio::test]
async fn with_nobody_to_review_a_card_waits_for_a_person_until_a_row_says_there_is_somebody() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    assert!(active(&rig, "reviewer").await.is_empty());
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    assert_eq!(rig.count_of("review.passed").await, 0);

    // The row is given a count, as someone would in the client: the card
    // that was waiting is theirs to look at, and nobody moved it.
    change(&rig, "agents", "reviewer", json!({ "count": 1 })).await;
    moved(&rig, &id, "done", 0).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);

    // The row taken away, its agent is gone when it has nothing in hand,
    // and the next card waits for a person again.
    remove_row(&rig, "agents", "reviewer").await;
    rig.event("agent.slot.stopped", |event| event.data["slot"] == "shop-reviewer-1").await;
    assert!(active(&rig, "reviewer").await.is_empty());
    let next = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &next, "ready", None).await;
    moved(&rig, &next, "review", 0).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    assert_eq!(card(&rig, &next).await["status"], "review");
}

#[tokio::test]
async fn a_stage_that_names_a_column_the_board_has_not_got_says_so_where_the_fleet_reports() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // A row with nothing but what it is called: its column is that, and the board has none.
    add_row(&rig, "stages", json!({ "title": "qa" })).await;
    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("names a column the board has not got")
        })
        .await;
    let body = told.data["body"].as_str().unwrap();
    assert!(
        body.starts_with("The stage qa names a column")
            && body.contains("draft, ready, doing, review, done")
            && body.contains("its own, qa"),
        "{body}"
    );
    // Its role is what it is called after too, and no row of agents has that one.
    rig.event("channel.message", |event| {
        event.data["body"]
            .as_str()
            .unwrap_or_default()
            .starts_with("The stage qa is for the role qa, and no row")
    })
    .await;
    // A row that was right and is put wrong says so as well, when it is.
    change(&rig, "stages", "review", json!({ "pass": "nowhere" })).await;
    rig.event("channel.message", |event| {
        let body = event.data["body"].as_str().unwrap_or_default();
        body.starts_with("The stage review names a column")
            && body.contains("where a card goes on to, nowhere")
    })
    .await;
}

#[tokio::test]
async fn a_stage_given_another_column_has_the_cards_that_arrive_there_from_then_on() {
    let (rig, _at) = rig().await;
    let columns = ["draft", "ready", "doing", "review", "check", "done"];
    shop_with(&rig, &columns, json!({})).await;
    // Those who review are to look at cards in another column, and worked cards are to go there.
    change(&rig, "stages", "review", json!({ "column": "check" })).await;
    change(&rig, "stages", "work", json!({ "pass": "check" })).await;

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let checked = moved(&rig, &id, "check", 0).await;
    moved(&rig, &id, "done", checked).await;
    let said = history(
        &rig,
        &id,
        &format!(
            "conductor moved it from check to done, saying: approved by shop-reviewer-1; lane/{id} was merged"
        ),
    )
    .await;
    assert!(!said.iter().any(|line| line.contains("to review")), "{said:?}");
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
}

#[tokio::test]
async fn two_projects_with_fleets_each_have_agents_of_their_own() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    rig.send("/api/projects", json!({ "name": "Depot" })).await;
    rig.board("depot").await;
    let mut theirs = params(json!({}));
    theirs["project"] = json!("depot");
    let request = json!({ "blueprint": "std/fleet", "project": "depot", "params": theirs, "wait": true });
    assert_eq!(rig.send("/api/apply", request).await["finished"]["status"], "ok");

    // An agent's name is its own in the whole factory, so each is called after its project.
    let all = rig.read("/api/agents").await;
    let mut whose: Vec<(String, String)> = all["agents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|agent| agent["active"] == true)
        .map(|agent| {
            (agent["name"].as_str().unwrap().to_string(), agent["project"].as_str().unwrap().to_string())
        })
        .collect();
    whose.sort();
    let named = |name: &str, project: &str| (name.to_string(), project.to_string());
    assert_eq!(
        whose,
        [
            named("depot-reviewer-1", "depot"),
            named("depot-worker-1", "depot"),
            named("shop-reviewer-1", "shop"),
            named("shop-worker-1", "shop"),
        ]
    );
    // And a card of one project is worked and reviewed by that project's own.
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
    assert_eq!(
        (given(&rig, "shop-worker-1").await.len(), given(&rig, "shop-reviewer-1").await.len()),
        (1, 1)
    );
    assert_eq!(
        (given(&rig, "depot-worker-1").await.len(), given(&rig, "depot-reviewer-1").await.len()),
        (0, 0)
    );
}

#[tokio::test]
async fn a_cell_that_was_emptied_says_no_more_than_one_never_filled() {
    // A cell of words emptied in the client is kept as empty text, a
    // number taken out is nothing, and so is a cell never filled: a row is
    // read the same whichever it is.
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    change(&rig, "agents", "worker", json!({ "count": 3 })).await;
    assert_eq!(active(&rig, "worker").await.len(), 3);
    // Emptied, the row says no count, which is one agent; and no role,
    // which is the one it is called after.
    change(&rig, "agents", "worker", json!({ "count": null, "role": "" })).await;
    assert_eq!(active(&rig, "worker").await, ["shop-worker-1"]);
    change(&rig, "stages", "review", json!({ "seats": null, "rounds": null, "fail": "", "picks": "" })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "done", 0).await;
}

#[tokio::test]
async fn a_row_called_what_another_is_brings_up_nobody_and_says_so() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    let arranged = add_row(&rig, "agents", json!({ "title": "reviewer", "role": "reviewer" })).await;
    // Counted as what it is: one who works, one who reviews.
    assert_eq!(arranged["agents"], 2, "{arranged}");
    assert_eq!(active(&rig, "reviewer").await, ["shop-reviewer-1"]);
    rig.event("channel.message", |event| {
        event.data["body"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Two rows of the fleet's agents are called reviewer.")
    })
    .await;
}

#[tokio::test]
async fn a_stage_that_sends_cards_back_to_where_they_are_worked_from_still_stops_at_its_rounds() {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "rounds": 1 })).await;
    // Sent back, a card is to wait its turn again and not go straight to whoever worked it.
    change(&rig, "stages", "review", json!({ "fail": "ready" })).await;
    says(&at, "shop-reviewer-1", "The total is wrong.", "send_back");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let first = moved(&rig, &id, "review", 0).await;
    let back = moved(&rig, &id, "ready", first).await;
    let again = moved(&rig, &id, "review", back).await;
    // Sent back as often as is allowed, it is left for a person: the two do not go round for ever.
    rig.event("channel.message", move |event| {
        event.seq > again && event.data["body"].as_str().unwrap_or_default().contains("left for a person")
    })
    .await;
    assert_eq!(rig.count_of("review.failed").await, 1);
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 2);
}

#[tokio::test]
async fn a_stage_says_how_many_look_at_a_card_where_that_is_fewer_than_there_are() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 3 })).await;
    // Three who review, and one of them to a card: said in the stage's row while the fleet runs.
    change(&rig, "stages", "review", json!({ "seats": 1 })).await;
    let told = rig.event("review.arranged", |_| true).await;
    assert_eq!((&told.data["stage"], &told.data["arranged"]), (&json!("review"), &json!("std/review/1/0")));
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let passed = rig.event("review.passed", |_| true).await;
    assert_eq!(
        (&passed.data["card"], &passed.data["seats"], &passed.data["approved"]),
        (&json!(id), &json!(1), &json!(1))
    );
    moved(&rig, &id, "done", 0).await;
}

#[tokio::test]
async fn a_stage_with_nowhere_to_send_what_passes_leaves_it_approved_and_one_taken_away_asks_nobody() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({})).await;
    // What passes is to stay where it is, for a person to finish.
    change(&rig, "stages", "review", json!({ "pass": "" })).await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let approved = rig.event("card.approved", |_| true).await;
    assert_eq!((&approved.data["card"], &approved.data["by"]), (&json!(id), &json!("shop-reviewer-1")));
    let said = history(&rig, &id, "shop-reviewer-1 approved it: It does what the card asks.").await;
    assert!(!said.iter().any(|line| line.contains("to done")), "{said:?}");
    assert_eq!(card(&rig, &id).await["status"], "review");

    // The stage's row taken away, it is stopped, which is said; and the next card is nobody's to review.
    remove_row(&rig, "stages", "review").await;
    rig.event("channel.message", |event| {
        event.data["body"].as_str().unwrap_or_default().starts_with("A stage of the fleet was taken away")
    })
    .await;
    let next = add(&rig, "Add a till", &[]).await;
    move_card(&rig, &next, "ready", None).await;
    moved(&rig, &next, "review", 0).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
}

#[tokio::test]
async fn with_nobody_to_work_a_card_stays_where_it_is_until_a_row_says_there_is_somebody() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "workers": 0 })).await;
    rig.event("channel.message", |event| {
        event.data["body"].as_str().unwrap_or_default().starts_with("Nobody in the fleet has the role worker")
    })
    .await;
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    // The fleet has taken the move and done nothing with it: the card is not begun.
    change(&rig, "stages", "review", json!({ "rounds": 3 })).await;
    assert_eq!(card(&rig, &id).await["status"], "ready");

    // Somebody to work it, said in the row: the card that waited is taken up, and nobody moved it.
    change(&rig, "agents", "worker", json!({ "count": 1 })).await;
    moved(&rig, &id, "done", 0).await;
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
}

#[tokio::test]
async fn what_the_fleet_is_told_of_the_remote_reaches_the_lanes_of_its_stage() {
    let (rig, at) = rig().await;
    let dir = shop(&rig, json!({ "push": true })).await;
    git(&at, &["init", "--bare", "--quiet", "remote.git"]);
    git(&dir, &["remote", "add", "origin", at.join("remote.git").to_str().unwrap()]);
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    // The card's branch was sent, as the fleet was told when it was applied.
    let sent =
        git(&at.join("remote.git"), &["rev-parse", "--verify", "--quiet", &format!("refs/heads/lane/{id}")]);
    assert_eq!(sent.len(), 40, "{sent}");
}

#[tokio::test]
async fn the_key_the_fleet_is_told_reaches_where_a_branch_is_sent_and_nothing_goes_as_somebody_else() {
    let (rig, at) = rig().await;
    let dir = shop(&rig, json!({ "push": true, "key": "shop-bot" })).await;
    git(&at, &["init", "--bare", "--quiet", "remote.git"]);
    git(&dir, &["remote", "add", "origin", at.join("remote.git").to_str().unwrap()]);
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;
    // The remote is a folder, which asks nobody for a key. The lane was
    // told the fleet's, and so does not send the branch as whoever runs
    // the daemon, which is how it would otherwise have gone.
    let told = said(&rig).await;
    assert!(
        told.contains(
            "origin is not reached over https, and the key shop-bot is only given to a remote that is"
        ),
        "{told}"
    );
    assert_eq!(git(&at.join("remote.git"), &["branch", "--list"]), "", "nothing reached the remote");
}
