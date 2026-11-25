//! A lane in a project that is a repository: what its agent changed is kept
//! as a commit on the lane's own branch, and the branch is sent to the
//! repository's remote only when that was asked for.
//!
//! The remote here is another repository in the rig's own folder, and the
//! agent a stand-in that changes one file in the directory it is run in.
//!
//! What becomes of a card that is sent back from review is in `rounds`.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::Rig;

/// Git as a test asks it, in `dir`: the one place this suite starts it.
/// Who commits is what the repository there says and never what the
/// environment the suite is run from names (`kernel::git::WHO`): with an
/// author or a committer named there, what a test committed was by
/// somebody else, and a test of who committed said so.
pub(crate) fn asking(dir: &Path) -> std::process::Command {
    let mut git = std::process::Command::new("git");
    git.current_dir(dir);
    for name in kernel::git::WHO {
        git.env_remove(name);
    }
    git
}

/// Runs `git` in `dir` and gives what it printed; a failure is the test's.
///
/// A repository made here says how its lines end, as it says who commits:
/// a device's own git may be told to write them another way where files
/// are checked out (as it is installed on Windows, it is), and what a
/// test then reads of a file that was merged is not what was written.
pub(crate) fn git(dir: &Path, args: &[&str]) -> String {
    let ran = asking(dir).args(args).output().unwrap();
    assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
    if let ["init", .., made] = args
        && !args.contains(&"--bare")
    {
        let said = asking(&dir.join(made)).args(["config", "core.autocrlf", "false"]).status();
        assert!(said.unwrap().success());
    }
    String::from_utf8_lossy(&ran.stdout).trim().to_string()
}

/// A `shop` project that is a repository with one commit, a remote called
/// `origin` beside it, a board, and the fleet that ships ([`fleet`]) started
/// with `params`. Where the project is, and where the remote is.
pub(crate) async fn shop(rig: &Rig, params: Value) -> (PathBuf, PathBuf) {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = PathBuf::from(project["path"].as_str().unwrap());
    let remote = rig.dir.join("remote.git");
    git(&rig.dir, &["init", "--quiet", "--bare", remote.to_str().unwrap()]);
    git(&dir, &["init", "--quiet", "."]);
    // Who commits is said in the repository itself, so that the test does
    // not lean on whoever runs it having told git who they are.
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    git(&dir, &["remote", "add", "origin", remote.to_str().unwrap()]);
    rig.board("shop").await;
    fleet(rig, params).await;
    (dir, remote)
}

/// The fleet that ships, started in `shop` with `params`: one who works
/// (`shop-worker-1`, run by `writer`) and a person to review, unless
/// `params` says otherwise.
pub(crate) async fn fleet(rig: &Rig, params: Value) {
    // A person reviews, unless the test says who does.
    let mut said = json!({ "reviewers": 0 });
    for (name, value) in params.as_object().into_iter().flatten() {
        said[name] = value.clone();
    }
    // What the tables start with, and whatever else the test says of the
    // fleet itself, handed on as it was said.
    let mut start = crate::fleets::params(said.clone());
    for (name, value) in said.as_object().into_iter().flatten() {
        if !["workers", "reviewers", "seats", "needed", "rounds", "in_flight", "agents", "stages"]
            .contains(&name.as_str())
        {
            start[name] = value.clone();
        }
    }
    let fleet = json!({ "blueprint": "std/fleet", "project": "shop", "params": start, "wait": true });
    let applied = rig.send("/api/apply", fleet).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
}

/// Who works the cards of a fleet started by [`shop`] with nothing said.
pub(crate) const WORKER: &str = "shop-worker-1";

/// A node whose agents can be given `writer`: a program that adds a line to
/// a file in the directory it is run in, and says so. It knows nothing of
/// lanes: that the file ends up on a lane's branch is the lane's doing.
pub(crate) async fn rig() -> Rig {
    rig_whose_agent(&["swallow", "append", "CHANGES.md", "changed", "say", "I changed a file."]).await
}

/// The same, with an agent that does `does` when it is given work: what
/// the stand-in for a program is told (`crates/hosting/standin`).
pub(crate) async fn rig_whose_agent(does: &[&str]) -> Rig {
    Rig::owned(
        Rig::with("api-lane", move |config, _| {
            config.adapters.push(("writer".into(), crate::standin::argv(does)));
        })
        .await,
    )
    .await
}

/// Waits until a card has been moved to `column` after `after` in the
/// journal: where the journal had got to then.
pub(crate) async fn moved(rig: &Rig, id: &str, column: &'static str, after: u64) -> u64 {
    let card = id.to_string();
    rig.event("board.item.moved", move |event| {
        event.seq > after && event.data["id"] == card.as_str() && event.data["to"] == column
    })
    .await
    .seq
}

/// Moves a card as its owner does, saying `note` as it goes if anything.
pub(crate) async fn move_card(rig: &Rig, id: &str, column: &str, note: Option<&str>) {
    let (status, moved) = rig
        .call(
            reqwest::Method::PATCH,
            &format!("/api/board/items/{id}"),
            Some(&rig.token),
            Some(json!({ "status": column, "note": note })),
        )
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{moved}");
}

/// Finishes a card and waits for the stage that worked it to have tidied
/// its place away.
pub(crate) async fn finished(rig: &Rig, id: &str) {
    let now = rig.mark();
    move_card(rig, id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > now && event.data["flow"] == "on.tidied").await;
}

/// Adds a card, makes it ready, and waits until it has been worked and has
/// come to rest in review: the card's id.
pub(crate) async fn worked(rig: &Rig, title: &str) -> String {
    let card = rig.send("/api/board/shop/items", json!({ "title": title })).await;
    let id = card["id"].as_str().unwrap().to_string();
    move_card(rig, &id, "ready", None).await;
    let reviewed = moved(rig, &id, "review", 0).await;
    at_rest(rig, reviewed).await;
    id
}

/// Waits until the stage's run for a card that was handed on after `after`
/// in the journal is over: the card has come to rest, and what is done to
/// it next is taken as a person's doing it would be.
pub(crate) async fn at_rest(rig: &Rig, after: u64) {
    rig.event("run.finished", move |event| event.seq > after && event.data["flow"] == "on.go").await;
}

/// What was said in the project's channel, one body a line.
pub(crate) async fn said(rig: &Rig) -> String {
    let messages = rig.read("/api/channels/shop/messages").await;
    let bodies: Vec<&str> = messages["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|message| message["body"].as_str().unwrap())
        .collect();
    bodies.join("\n")
}

/// The history of a card once it has `line` in it: waited for by the
/// events that say a line was written, not by looking again.
pub(crate) async fn history(rig: &Rig, id: &str, line: &str) -> Vec<String> {
    history_with(rig, id, line, 1).await
}

/// The same, once it has `line` in it `times` over.
pub(crate) async fn history_with(rig: &Rig, id: &str, line: &str, times: usize) -> Vec<String> {
    let mut written = rig.follow("collection.history");
    crate::soon(&format!("`{line}` in the history of {id}, {times} times"), async {
        loop {
            let history = rig.read(&format!("/api/records/{id}/history")).await;
            let said: Vec<String> = history["history"]
                .as_array()
                .unwrap()
                .iter()
                .map(|one| one["said"].as_str().unwrap().to_string())
                .collect();
            if said.iter().filter(|one| *one == line).count() >= times {
                return said;
            }
            written.next().await.unwrap();
        }
    })
    .await
}

#[tokio::test]
async fn a_cards_history_says_its_lane_moved_it_and_which_agent_worked_on_it_and_not_the_person_who_started_the_fleet()
 {
    let rig = rig().await;
    shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    let said = history(&rig, &id, "conductor moved it from doing to review").await;
    let moves: Vec<&str> = said
        .iter()
        .map(String::as_str)
        .filter(|line| line.contains("moved it") || line.contains("worked"))
        .collect();
    assert_eq!(
        moves,
        [
            // The person made it ready; everything after is the lane's doing,
            // and its round's, though it was done with that person's authority.
            "Ada moved it from draft to ready",
            "conductor moved it from ready to doing",
            "shop-worker-1 worked on it",
            "conductor moved it from doing to review",
        ],
        "{said:?}"
    );
    // The event still says whose authority it was done with, and where.
    let moved = rig.event("board.item.moved", |event| event.data["to"] == "review").await;
    assert_eq!(moved.data["through"], "std/lane-round");
    assert!(
        moved.source.actor.as_deref().is_some_and(|actor| actor.starts_with("user:")),
        "{:?}",
        moved.source
    );
    assert!(moved.source.instance.is_some(), "{:?}", moved.source);
}

#[tokio::test]
async fn every_line_a_blueprint_wrote_in_a_cards_history_is_said_by_the_bot_and_a_persons_own_by_the_person()
{
    let rig = rig().await;
    shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;
    // What the fleet said for itself, with no action that changes a
    // record: which agent worked on it, and that it waits for a person,
    // which is said a moment after the move to review is.
    let topic = |line: &Value| line["topic"].as_str().unwrap_or_default().to_string();
    let mut written = rig.follow("collection.history");
    let lines = crate::soon("what the fleet said of the card in its history", async {
        loop {
            let history = rig.read(&format!("/api/records/{id}/history")).await;
            let lines = history["history"].as_array().unwrap().clone();
            if ["card.worked", "card.waits"].iter().all(|said| lines.iter().any(|line| topic(line) == *said))
            {
                return lines;
            }
            written.next().await.unwrap();
        }
    })
    .await;
    for line in &lines {
        let through = line["data"]["through"].as_str().unwrap_or_default();
        if line["said"].as_str().unwrap().starts_with("Ada ") {
            assert_eq!(line["who"], "Ada", "{line:#}");
            assert_eq!(through, "", "a person's own doing is said by no blueprint: {line:#}");
        } else {
            assert_eq!(line["who"], "conductor", "{line:#}");
            assert!(through.starts_with("std/"), "said with the blueprint that did it: {line:#}");
        }
    }
    let worked = lines.iter().find(|line| topic(line) == "card.worked").unwrap();
    assert_eq!(worked["said"], "shop-worker-1 worked on it");
    assert_eq!(worked["data"]["through"], "std/work");
}

#[tokio::test]
async fn a_lane_asked_to_push_keeps_what_its_agent_changed_as_a_commit_and_sends_its_branch_to_the_remote() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;
    let branch = format!("lane/{id}");

    // The work is a commit on the lane's branch, named for the card, and
    // the remote has it; the project's own branch is where it was.
    assert_eq!(git(&remote, &["log", "--format=%s", &branch]), format!("{id}: Add a cart\nfirst"));
    assert_eq!(git(&remote, &["show", "--format=", "--name-only", &branch]), "CHANGES.md");
    assert_eq!(git(&dir, &["log", "--format=%s", "HEAD"]), "first");
    let told = said(&rig).await;
    assert!(told.contains(&format!("{id}: its branch {branch} was sent to origin")), "{told}");
    // The lanes are not the project's files, and the repository does not
    // take them for changes.
    assert!(!git(&dir, &["status", "--porcelain"]).contains(".lanes"));
}

#[tokio::test]
async fn a_lane_that_was_not_asked_to_push_keeps_the_work_and_sends_nothing_anywhere() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    assert_eq!(git(&dir, &["log", "--format=%s", &format!("lane/{id}")]), format!("{id}: Add a cart\nfirst"));
    assert_eq!(git(&remote, &["branch", "--list"]), "", "nothing reached the remote");
    let told = said(&rig).await;
    assert!(!told.contains("sent to"), "{told}");
}

#[tokio::test]
async fn a_lane_says_when_its_branch_could_not_be_sent_and_work_that_cannot_be_committed_is_given_back_to_its_worker_and_then_left_for_a_person()
 {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;

    // A remote that is not there: the work is kept, and the lane says the
    // branch did not go and why.
    git(&dir, &["remote", "set-url", "origin", rig.dir.join("nowhere.git").to_str().unwrap()]);
    let first = worked(&rig, "Add a cart").await;
    assert_eq!(
        git(&dir, &["log", "--format=%s", "-1", &format!("lane/{first}")]),
        format!("{first}: Add a cart")
    );
    let told = said(&rig).await;
    assert!(told.contains(&format!("{first}: its branch was not sent to origin: ")), "{told}");

    // A repository in which the commit cannot be made. (It is to sign every
    // commit, with a program that is not there. A hook that refused would
    // not do: the fleet's git runs none.) The card does not go on with
    // nothing committed: its worker is given it again, in the conversation
    // it had, and told what git said. Nothing here puts it right, so when
    // that has been done as often as a round allows, the card is left
    // where it is for a person, and why is said.
    git(&dir, &["remote", "set-url", "origin", remote.to_str().unwrap()]);
    git(&dir, &["config", "commit.gpgsign", "true"]);
    git(&dir, &["config", "gpg.program", "/nowhere/to/sign/with"]);
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a till" })).await;
    let second = card["id"].as_str().unwrap().to_string();
    move_card(&rig, &second, "ready", None).await;
    let left = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("It is left in doing for a person")
        })
        .await;
    let told = said(&rig).await;
    assert!(
        told.contains(&format!(
            "{second}: what was changed could not be committed, so it has not gone on and nothing was sent anywhere; it is still in .lanes/{second}"
        )),
        "{told}"
    );
    assert_eq!(
        told.matches("shop-worker-1 is given the card again, to put that right.").count(),
        2,
        "{told}"
    );
    let last = left.data["body"].as_str().unwrap();
    assert!(
        last.ends_with("It is left in doing for a person, having been given back 2 times for it."),
        "{last}"
    );

    let turns = rig.read("/api/agents/shop-worker-1/turns").await;
    let mut asked: Vec<&str> = turns["turns"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|turn| turn["thread"] == format!("lane:{second}").as_str())
        .map(|turn| turn["prompt"].as_str().unwrap())
        .collect();
    asked.reverse();
    assert_eq!(asked.len(), 3, "once, and twice more: {asked:?}");
    assert!(asked[0].starts_with("Please take this card."), "{}", asked[0]);
    for again in &asked[1..] {
        assert!(
            again.starts_with("What you changed on this card could not be committed")
                && again.contains("What git said:")
                && again.contains("/nowhere/to/sign/with"),
            "{again}"
        );
    }
    let board = rig.read("/api/board/shop").await;
    let card = board["items"].as_array().unwrap().iter().find(|card| card["id"] == second.as_str()).unwrap();
    assert_eq!(card["status"], "doing");
    assert_eq!(git(&remote, &["branch", "--list"]), "", "nothing reached the remote");
}

#[tokio::test]
async fn what_is_written_on_a_card_is_committed_as_it_was_written_and_none_of_it_is_run() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;
    // Everything a shell would take for something to do, in a title.
    let title =
        "Add `touch RAN-1` $(touch RAN-2); touch RAN-3 && \"touch RAN-4\" | tee RAN-5 > RAN-6 --amend";
    let id = worked(&rig, title).await;
    let branch = format!("lane/{id}");

    // The commit is named for the card, letter for letter, and is the
    // agent's; the remote has it.
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", &branch]), format!("{id}: {title}"));
    assert_eq!(
        git(&dir, &["log", "--format=%an <%ae>", "-1", &branch]),
        "shop-worker-1 <shop-worker-1@agents.invalid>"
    );
    assert_eq!(git(&remote, &["log", "--format=%s", "-1", &branch]), format!("{id}: {title}"));
    // Nothing in it was run: not where the lane works, and not in the project.
    for place in [dir.clone(), dir.join(".lanes").join(&id)] {
        let ran: Vec<String> = std::fs::read_dir(&place)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("RAN"))
            .collect();
        assert!(ran.is_empty(), "{ran:?} in {}", place.display());
    }
    assert_eq!(
        git(&dir, &["show", "--format=", "--name-only", &branch]),
        "CHANGES.md",
        "only the agent's work"
    );
}

#[tokio::test]
async fn a_lane_in_a_repository_with_no_history_to_branch_from_works_in_the_project_itself() {
    let rig = rig().await;
    // A repository, and nothing committed in it yet: there is nothing for
    // a lane's branch to start from.
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = PathBuf::from(project["path"].as_str().unwrap());
    git(&dir, &["init", "--quiet", "."]);
    rig.board("shop").await;
    fleet(&rig, json!({ "push": true })).await;

    let id = worked(&rig, "Add a cart").await;

    // The agent worked where the project is, and no place was made for it.
    assert_eq!(std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(), "changed\n");
    assert!(!dir.join(".lanes").exists(), "no worktree, and nothing said of one to the repository");
    let turns = rig.read("/api/agents/shop-worker-1/turns").await;
    let cwd = &turns["turns"][0]["cwd"];
    assert!(cwd.is_null() || cwd == "", "it was given the project itself to work in: {turns}");
    // Nothing was committed for it, which is not a failure; and with no
    // branch of its own there was none to send, which is said.
    let told = said(&rig).await;
    assert!(!told.contains("could not be committed"), "{told}");
    assert!(
        told.contains(&format!("{id}: its branch was not sent to origin: "))
            && told.contains("no worktree of its own, so no branch to send"),
        "{told}"
    );

    // Done, there is nothing of the card's own to tidy away, and nothing is said.
    let now = rig.mark();
    move_card(&rig, &id, "done", None).await;
    rig.event("run.finished", move |event| event.seq > now && event.data["flow"] == "on.tidied").await;
    let told = said(&rig).await;
    assert!(!told.contains("tidied") && !told.contains("kept"), "{told}");
    assert_eq!(
        std::fs::read_to_string(dir.join("CHANGES.md")).unwrap(),
        "changed\n",
        "the work is where it was"
    );
}

#[tokio::test]
async fn nothing_is_asked_of_git_for_a_card_or_in_a_place_that_would_be_outside_the_project() {
    // A lane's folder and its branch are called after its card, and where
    // it is worked is said from the top of the project: neither is a way
    // to somewhere else on the device.
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    let elsewhere = rig.dir.join("elsewhere");
    git(&rig.dir, &["init", "--quiet", elsewhere.to_str().unwrap()]);
    std::fs::write(elsewhere.join("NOTES.md"), "somebody else's\n").unwrap();
    let asked = |with: &str| {
        let source = format!(
            "blueprint: errand\npermissions: [shell]\nsteps:\n  go:\n    do: place.keep\n    with: {with}\n"
        );
        rig.send("/api/apply", json!({ "project": "shop", "source": source, "wait": true }))
    };
    let outside = asked("{ item: shop-1, dir: ../elsewhere, by: shop-worker-1 }").await;
    let why = outside["finished"]["error"].as_str().unwrap();
    assert!(
        why.contains("`dir` is a folder within the project") && why.contains("would be outside it"),
        "{why}"
    );
    let whole = asked(&format!("{{ item: shop-1, dir: {}, by: shop-worker-1 }}", elsewhere.display())).await;
    assert!(whole["finished"]["error"].as_str().unwrap().contains("would be outside it"), "{whole}");
    for card in ["../elsewhere", "-D", "a/b"] {
        let named = asked(&format!("{{ item: \"{card}\", dir: . }}")).await;
        let why = named["finished"]["error"].as_str().unwrap();
        assert!(
            why.contains("`item` is what the work is known by")
                && why.contains("cannot be the name of either"),
            "{why}"
        );
    }
    // Nothing was put forward or committed there, or in the project.
    assert_eq!(git(&elsewhere, &["status", "--porcelain"]), "?? NOTES.md");
    assert_eq!(git(&dir, &["log", "--format=%s"]), "first");
}

#[tokio::test]
async fn a_card_whose_place_cannot_be_made_is_not_worked_in_the_projects_own_folder_and_the_channel_says_why()
{
    let (rig, _at) = crate::fleets::rig().await;
    let dir = crate::fleets::shop(&rig, json!({ "reviewers": 0 })).await;
    let id = crate::fleets::add(&rig, "Add a cart", &[]).await;
    // Something is where the card's worktree would go.
    std::fs::create_dir_all(dir.join(".lanes")).unwrap();
    std::fs::write(dir.join(".lanes").join(&id), "in the way\n").unwrap();
    move_card(&rig, &id, "ready", None).await;

    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("was not worked")
        })
        .await;
    let body = told.data["body"].as_str().unwrap();
    assert!(
        body.contains(&format!("{id} was not worked: its place .lanes/{id} could not be made")),
        "{body}"
    );
    assert!(body.contains("already exists"), "what git said of it: {body}");
    assert!(body.contains("Put that right and move the card to ready again"), "{body}");
    // Nobody was asked to work where the project's own files are, and
    // nothing was left among them.
    assert!(crate::fleets::given(&rig, "shop-worker-1").await.is_empty());
    assert!(!dir.join("CHANGES.md").exists());

    // Put right and made ready again, it is worked in a place of its own.
    std::fs::remove_file(dir.join(".lanes").join(&id)).unwrap();
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", told.seq).await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]), format!("{id}: Add a cart"));
    assert!(!dir.join("CHANGES.md").exists());
}
