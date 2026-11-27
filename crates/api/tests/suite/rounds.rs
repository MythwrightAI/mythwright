//! A card that is sent back from review goes round again, and what a lane
//! leaves behind it when the card is done.
//!
//! Whoever reviews moves the card from review to doing and says what is
//! wrong. The lane that took the card takes it up again: the same agent in
//! the same conversation, worktree and branch, told what was said; and when
//! the agent has finished, the lane does what it did the first time.
//!
//! An agent that cannot tell what is wanted asks on the card instead, and
//! the card waits in doing until someone answers there.

use serde_json::{Value, json};

use crate::Rig;
use crate::lanes::{
    at_rest, finished, git, history, history_with, move_card, moved, rig, rig_whose_agent, said, shop, worked,
};

/// The turns the fleet's one worker has taken, oldest first.
async fn turns(rig: &Rig) -> Vec<Value> {
    let mut turns = rig.read("/api/agents/shop-worker-1/turns").await["turns"].as_array().unwrap().clone();
    turns.reverse();
    turns
}

/// Sends a card back from review and waits for it to have been worked
/// again and to have come to rest in review.
async fn sent_back(rig: &Rig, id: &str, note: Option<&str>) {
    let now = rig.mark();
    move_card(rig, id, "doing", note).await;
    let reviewed = moved(rig, id, "review", now).await;
    at_rest(rig, reviewed).await;
}

/// The runs of the stage that works cards that were started by `handler`.
async fn runs_of(rig: &Rig, handler: &str) -> usize {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let mut stage = None;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == "std/working") {
        let id = one["id"].as_str().unwrap().to_string();
        if rig.read(&format!("/api/instances/{id}")).await["instance"]["params"]["row"]["does"] == "work" {
            stage = Some(id);
        }
    }
    let stage = stage.expect("a stage that works cards");
    let runs = rig.read(&format!("/api/runs?instance={stage}&limit=200")).await;
    runs["runs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|run| run["flow"] == format!("on.{handler}").as_str())
        .count()
}

#[tokio::test]
async fn a_card_sent_back_with_a_note_goes_round_again_in_the_same_branch_and_conversation_and_its_agent_is_told_the_note()
 {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;
    let branch = format!("lane/{id}");
    assert_eq!(git(&remote, &["log", "--format=%s", &branch]), format!("{id}: Add a cart\nfirst"));

    sent_back(&rig, &id, Some("The total is wrong when the cart is empty.")).await;

    // The same agent, in the same conversation, in the same place.
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "{turns:?}");
    for turn in &turns {
        assert_eq!(turn["thread"], format!("lane:{id}").as_str(), "one conversation: {turn}");
        assert_eq!(turn["cwd"], format!(".lanes/{id}").as_str(), "one place: {turn}");
    }
    let (first, second) = (turns[0]["prompt"].as_str().unwrap(), turns[1]["prompt"].as_str().unwrap());
    assert!(first.starts_with("Please take this card.") && !first.contains("sent back"), "{first}");
    assert!(
        second.starts_with("This card was sent back, for you to take up again")
            && second.contains("What was said:\n\nThe total is wrong when the cart is empty.")
            && second.contains(&format!("Card {id}: Add a cart")),
        "{second}"
    );

    // And the lane did what it did the first time: the work is a second
    // commit on the same branch, the branch was sent again, and it said so.
    assert_eq!(
        git(&remote, &["log", "--format=%s", &branch]),
        format!("{id}: Add a cart\n{id}: Add a cart\nfirst")
    );
    assert_eq!(git(&dir, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 2);
    let told = said(&rig).await;
    assert_eq!(told.matches(&format!("{id}: its branch {branch} was sent to origin")).count(), 2, "{told}");
    assert_eq!(told.matches("I changed a file.").count(), 2, "the agent said so itself, each round: {told}");
}

#[tokio::test]
async fn a_card_goes_round_as_often_as_it_is_sent_back_and_never_because_its_stage_moved_it() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;
    // The stage has moved the card twice itself by now, to doing and to review.
    assert_eq!(turns(&rig).await.len(), 1);

    sent_back(&rig, &id, Some("The total is wrong.")).await;
    sent_back(&rig, &id, Some("Still wrong: it ignores the discount.")).await;

    let turns = turns(&rig).await;
    let asked: Vec<&str> = turns.iter().map(|turn| turn["prompt"].as_str().unwrap()).collect();
    assert_eq!(asked.len(), 3, "{asked:?}");
    assert!(asked[1].contains("The total is wrong.") && !asked[1].contains("discount"), "{}", asked[1]);
    assert!(asked[2].contains("Still wrong: it ignores the discount."), "{}", asked[2]);
    assert_eq!(
        git(&dir, &["log", "--format=%s", &format!("lane/{id}")]),
        format!("{id}: Add a cart\n{id}: Add a cart\n{id}: Add a cart\nfirst")
    );

    // Once the card is done, all the stage ever began for it is there to
    // count: one for each time someone sent the card back, and none for
    // the six moves the stage and its rounds made themselves.
    let back = runs_of(&rig, "back").await;
    finished(&rig, &id).await;
    assert_eq!((back, runs_of(&rig, "back").await), (2, 2));
    assert_eq!(self::turns(&rig).await.len(), 3);
}

#[tokio::test]
async fn a_card_put_back_in_ready_is_taken_again_by_the_stage_that_worked_it_and_goes_round_as_one_sent_back_does()
 {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    // Dragged from review to ready, and not to doing: the stage hears a
    // card come to its column that it worked before.
    let now = rig.mark();
    move_card(&rig, &id, "ready", Some("The total is wrong.")).await;
    moved(&rig, &id, "review", now).await;

    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "one more round, and one only");
    let second = turns[1]["prompt"].as_str().unwrap();
    assert!(
        second.starts_with("This card was sent back, for you to take up again")
            && second.contains("The total is wrong."),
        "{second}"
    );
    assert_eq!(turns[0]["thread"], turns[1]["thread"], "in the conversation it was worked in");
    assert_eq!(
        git(&dir, &["log", "--format=%s", &format!("lane/{id}")]),
        format!("{id}: Add a cart\n{id}: Add a cart\nfirst"),
        "and on the branch it was worked on"
    );
    assert_eq!(runs_of(&rig, "back").await, 0, "it was not sent back to doing, and is not taken for that");
}

#[tokio::test]
async fn a_card_sent_back_with_nothing_said_tells_its_agent_to_look_at_the_card_and_what_went_before() {
    let rig = rig().await;
    shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    sent_back(&rig, &id, None).await;

    let turns = turns(&rig).await;
    let second = turns[1]["prompt"].as_str().unwrap();
    assert!(
        second.starts_with(
            "This card was sent back with nothing said. Look at the card, and at what you did and said \
             before, to find what is wrong, and put it right."
        ),
        "{second}"
    );
    assert!(!second.contains("What was said"), "{second}");
}

#[tokio::test]
async fn a_cards_history_says_who_sent_it_back_and_why_and_each_round_of_work() {
    let rig = rig().await;
    shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;
    sent_back(&rig, &id, Some("The total is wrong.")).await;
    sent_back(&rig, &id, None).await;

    // The lines are written in the order things happened, so the last of
    // them being there is all of them being there: back in review a third
    // time, having been sent back as often as its stage allows, it is left
    // for a person, which is said a moment after it came there.
    let left = "it was left for a person, having been sent back 2 times";
    let said = history_with(&rig, &id, left, 1).await;
    // What is said of it waiting for a person to review it, nobody being
    // there to, is no part of a round's work (`waited.rs`).
    let journey: Vec<&str> = said
        .iter()
        .map(String::as_str)
        .filter(|line| {
            !line.contains("its own blueprint") && !line.ends_with("it waits for a person to review it.")
        })
        .collect();
    let branch = format!("its branch lane/{id} was sent to origin");
    assert_eq!(
        journey,
        [
            "Ada added it to draft",
            "Ada moved it from draft to ready",
            "conductor moved it from ready to doing",
            "shop-worker-1 worked on it",
            branch.as_str(),
            "conductor moved it from doing to review",
            "Ada moved it from review to doing, saying: The total is wrong.",
            "shop-worker-1 worked on it",
            branch.as_str(),
            "conductor moved it from doing to review",
            // Nothing said, nothing put in anybody's mouth.
            "Ada moved it from review to doing",
            "shop-worker-1 worked on it",
            branch.as_str(),
            "conductor moved it from doing to review",
            left,
        ],
        "{said:?}"
    );
}

#[tokio::test]
async fn a_lane_says_of_its_card_that_its_branch_reached_the_remote_and_says_nothing_when_it_did_not() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;

    let pushed = rig.event("card.pushed", |_| true).await;
    assert_eq!(
        pushed.data,
        json!({
            "card": id, "project": "shop", "branch": format!("lane/{id}"), "remote": "origin",
            "title": "Add a cart",
        })
    );

    // A branch that did not go is not said to have gone. By the time the
    // card is in review its round has said all it was going to.
    git(&dir, &["remote", "set-url", "origin", rig.dir.join("nowhere.git").to_str().unwrap()]);
    let second = worked(&rig, "Add a till").await;
    let events = rig.read("/api/events/recent?topics=card.pushed&limit=10").await;
    let cards: Vec<&str> = events["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["data"]["card"].as_str().unwrap())
        .collect();
    assert_eq!(cards, [id.as_str()], "nothing was said of {second}");
}

#[tokio::test]
async fn where_a_cards_branch_was_proposed_for_merging_is_kept_in_its_history() {
    let rig = rig().await;
    shop(&rig, json!({})).await;
    let card = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = card["id"].as_str().unwrap();

    // Said by whatever proposes the branch, which is a package's business:
    // here, a blueprint that only says it.
    let source = format!(
        "blueprint: proposer\nsteps:\n  say: {{ do: emit, with: {{ topic: card.proposed, card: {id}, project: shop, \
         branch: \"lane/{id}\", url: \"https://hosting.example/shop/pull/1\" }} }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": source, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    let line = "a pull request was opened for it: https://hosting.example/shop/pull/1";
    let said = history_with(&rig, id, line, 1).await;
    assert_eq!(said.iter().filter(|one| *one == line).count(), 1, "{said:?}");
}

#[tokio::test]
async fn what_a_lane_commits_is_the_agents_and_was_committed_by_whoever_git_is_configured_as() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    assert_eq!(
        git(&dir, &["log", "-1", "--format=%an <%ae> / %cn <%ce>", &format!("lane/{id}")]),
        "shop-worker-1 <shop-worker-1@agents.invalid> / Ada <ada@example.com>"
    );
}

#[tokio::test]
async fn a_commit_the_agent_made_itself_is_left_as_it_is() {
    // An agent that commits its own work, as whoever it finds git set up
    // as in the project: with no wall it is shown the environment the
    // daemon has, which here is the one the suite is run from, and it
    // goes by nobody that names.
    let mut commits = vec!["swallow", "append", "CHANGES.md", "changed", "run", "(", "git", "add", "-A", ")"];
    for named in kernel::git::WHO {
        commits.extend(["unset", named]);
    }
    commits.extend([
        "run",
        "(",
        "git",
        "commit",
        "--quiet",
        "-m",
        "By my own hand",
        ")",
        "say",
        "I committed it.",
    ]);
    let rig = rig_whose_agent(&commits).await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    assert_eq!(
        git(&dir, &["log", "--format=%s by %an <%ae>", &format!("lane/{id}")]),
        "By my own hand by Ada <ada@example.com>\nfirst by Ada <ada@example.com>",
        "nothing was left for the lane to commit, and it rewrote nothing"
    );
}

#[tokio::test]
async fn a_task_is_done_in_the_directory_it_names_and_one_outside_the_project_is_refused() {
    let rig = rig_whose_agent(&["swallow", "say", "%:cwd%"]).await;
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = std::path::PathBuf::from(project["path"].as_str().unwrap());
    std::fs::create_dir_all(dir.join("parts/till")).unwrap();
    rig.send("/api/agents", json!({ "name": "builder", "adapter": "writer", "project": "shop" })).await;
    let task = |cwd: &str| {
        let source = format!(
            "blueprint: errand\nsteps:\n  go:\n    do: agent.task\n    with: {{ slot: builder, prompt: Where are you?, \
             cwd: \"{cwd}\" }}\noutputs:\n  where: ${{ steps.go.out.reply }}\n"
        );
        json!({ "source": source, "project": "shop", "wait": true, "key": format!("errand-{}", cwd.len()) })
    };

    let there = rig.send("/api/apply", task("parts/till")).await;
    let said = there["finished"]["outputs"]["where"].as_str().unwrap();
    assert_eq!(std::path::Path::new(said), kernel::whole::whole(&dir.join("parts/till")).unwrap(), "{there}");
    // Left out, it is the project's own directory, as it always was.
    let here = rig.send("/api/apply", task("")).await;
    let said = here["finished"]["outputs"]["where"].as_str().unwrap();
    assert_eq!(std::path::Path::new(said), kernel::whole::whole(&dir).unwrap(), "{here}");

    for (cwd, why) in [
        ("../elsewhere", "`../elsewhere` would leave it"),
        ("/tmp", "`/tmp` would leave it"),
        ("parts/counter", "the project `shop` has no directory `parts/counter` to work in"),
    ] {
        let refused = rig.send("/api/apply", task(cwd)).await;
        assert_eq!(refused["finished"]["status"], "failed", "{refused}");
        assert!(refused["finished"]["error"].as_str().unwrap().contains(why), "{refused}");
    }
    let turns = rig.read("/api/agents/builder/turns").await;
    assert_eq!(
        turns["turns"].as_array().unwrap().len(),
        2,
        "one that was refused was never given to the agent"
    );
}

#[tokio::test]
async fn a_done_cards_lane_removes_its_worktree_and_its_branch_when_the_work_is_on_the_remote() {
    let rig = rig().await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;
    let branch = format!("lane/{id}");
    assert!(dir.join(".lanes").join(&id).is_dir());

    finished(&rig, &id).await;

    assert!(!dir.join(".lanes").join(&id).exists(), "the worktree is gone");
    assert_eq!(git(&dir, &["branch", "--list", &branch]), "", "and the branch, here");
    // Nothing is done to the remote: the branch there is as it was.
    assert_eq!(git(&remote, &["log", "--format=%s", &branch]), format!("{id}: Add a cart\nfirst"));
    let told = said(&rig).await;
    assert!(!told.contains("was kept"), "{told}");
}

#[tokio::test]
async fn a_done_cards_lane_removes_its_branch_when_the_work_is_in_the_projects_own_branch() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;
    let branch = format!("lane/{id}");
    // Whoever reviewed it took the work into the project's own branch.
    git(&dir, &["merge", "--quiet", &branch]);

    finished(&rig, &id).await;

    assert_eq!(git(&dir, &["branch", "--list", &branch]), "");
    assert_eq!(git(&dir, &["log", "--format=%s", "HEAD"]), format!("{id}: Add a cart\nfirst"));
}

#[tokio::test]
async fn a_done_cards_lane_keeps_a_branch_whose_work_is_nowhere_else_and_says_why() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({})).await;
    // Passed by hand with its branch left as it is, and not merged.
    crate::fleets::change(&rig, "stages", "review", json!({ "by_hand": "leave" })).await;
    let id = worked(&rig, "Add a cart").await;
    let branch = format!("lane/{id}");

    finished(&rig, &id).await;

    assert!(!dir.join(".lanes").join(&id).exists(), "the worktree held nothing the branch does not");
    assert_eq!(git(&dir, &["log", "--format=%s", &branch]), format!("{id}: Add a cart\nfirst"));
    let told = said(&rig).await;
    assert!(
        told.contains(&format!(
            "{id}: its branch {branch} was kept, because it has work that is neither in the project's own \
             branch nor on origin"
        )),
        "{told}"
    );
}

#[tokio::test]
async fn a_done_cards_lane_keeps_a_worktree_that_holds_what_was_never_committed_and_says_why() {
    let rig = rig().await;
    let (dir, _) = shop(&rig, json!({ "push": true })).await;
    let id = worked(&rig, "Add a cart").await;
    // Somebody went on working there after the lane had committed.
    std::fs::write(dir.join(".lanes").join(&id).join("NOTES.md"), "not committed anywhere\n").unwrap();

    finished(&rig, &id).await;

    let kept = dir.join(".lanes").join(&id);
    assert_eq!(std::fs::read_to_string(kept.join("NOTES.md")).unwrap(), "not committed anywhere\n");
    assert_eq!(
        git(&dir, &["branch", "--list", "--format=%(refname:short)", &format!("lane/{id}")]),
        format!("lane/{id}")
    );
    let told = said(&rig).await;
    assert!(
        told.contains(&format!(
            "{id}: its worktree .lanes/{id} and its branch lane/{id} were kept, because the worktree could \
             not be removed without losing something: "
        )),
        "{told}"
    );
    // And what git said of it, which is why.
    assert!(told.contains("contains modified or untracked files"), "{told}");
}

/// An agent that asks before it will work: it does nothing to the files
/// until it is told its question was answered, and then does the work.
const ASKS_FIRST: &[&str] = &[
    "read",
    "where",
    "it has been answered",
    "(",
    "append",
    "CHANGES.md",
    "changed",
    "say",
    "I changed a file.",
    "say",
    "DECISION: done",
    ")",
    "say",
    "Which total: with tax, or without?",
    "say",
    "DECISION: ask",
];

/// One that does the work it is given, and asks only when a card comes
/// back to it with nothing said.
const ASKS_WHY: &[&str] = &[
    "read",
    "where",
    "sent back with nothing said",
    "(",
    "say",
    "What is wrong with it?",
    "say",
    "DECISION: ask",
    ")",
    "append",
    "CHANGES.md",
    "changed",
    "say",
    "I changed a file.",
    "say",
    "DECISION: done",
];

/// A card as the board has it now.
async fn card(rig: &Rig, id: &str) -> Value {
    let board = rig.read("/api/board/shop").await;
    board["items"].as_array().unwrap().iter().find(|card| card["id"] == id).cloned().unwrap()
}

/// What is asked on a card and not yet answered, as the board shows it:
/// nothing, for a card that is not waiting.
async fn question(rig: &Rig, id: &str) -> Option<String> {
    card(rig, id).await["more"]["question"].as_str().filter(|asked| !asked.is_empty()).map(str::to_string)
}

/// Says something on a card, as its owner.
async fn say(rig: &Rig, id: &str, body: &str) {
    rig.send(&format!("/api/records/{id}/comments"), json!({ "body": body })).await;
}

/// Waits until a card's question was last changed after `after`: put on
/// the card or taken off it, by what looks after the board's cards.
async fn questioned(rig: &Rig, id: &str, after: u64) {
    let card = id.to_string();
    rig.event("board.item.updated", move |event| {
        event.seq > after && event.data["id"] == card.as_str() && event.data["changed"] == json!(["question"])
    })
    .await;
}

/// A card whose agent has asked something on it instead of working, and
/// which is waiting where it is: the card's id.
async fn asked(rig: &Rig) -> String {
    let added = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = added["id"].as_str().unwrap().to_string();
    move_card(rig, &id, "ready", None).await;
    questioned(rig, &id, 0).await;
    id
}

#[tokio::test]
async fn an_agent_that_cannot_tell_what_is_wanted_asks_on_the_card_and_is_given_it_again_with_the_answer() {
    let rig = rig_whose_agent(ASKS_FIRST).await;
    let (dir, remote) = shop(&rig, json!({ "push": true })).await;
    let id = asked(&rig).await;
    let branch = format!("lane/{id}");

    // It asked instead of working: the card waits where it is with the
    // question on it, and nothing was kept, sent or handed on for review.
    let waiting = card(&rig, &id).await;
    assert_eq!(
        (waiting["status"].as_str(), waiting["more"]["question"].as_str(), &waiting["labels"]),
        (Some("doing"), Some("Which total: with tax, or without?"), &json!([])),
        "{waiting}"
    );
    assert_eq!(git(&dir, &["log", "--format=%s", &branch]), "first", "nothing was committed");
    assert_eq!(git(&remote, &["branch", "--list", &branch]), "", "and nothing was sent");
    let asking = "shop-worker-1 asked: Which total: with tax, or without?";
    let journey = history(&rig, &id, asking).await;
    assert!(
        !journey.iter().any(|line| line.contains("worked on it") || line.contains("to review")),
        "{journey:?}"
    );
    // That the question was put on the card is not said a second time.
    assert!(!journey.iter().any(|line| line.contains("changed its question")), "{journey:?}");
    // The question is the agent's own, in the channel as on the card, and
    // the line it chose on is in neither.
    let messages = rig.read("/api/channels/shop/messages").await;
    let last = messages["messages"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(
        (last["author"].as_str(), last["body"].as_str()),
        (Some("agent:shop-worker-1"), Some("Which total: with tax, or without?"))
    );
    assert!(!said(&rig).await.contains("was sent to origin"));

    // Answered on the card, it goes round again, told the answer.
    let now = rig.mark();
    say(&rig, &id, "Without tax.").await;
    moved(&rig, &id, "review", now).await;
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2);
    let second = turns[1]["prompt"].as_str().unwrap();
    assert!(
        second.starts_with("You asked something on this card, and it has been answered.")
            && second.contains("The answer:\n\nWithout tax."),
        "{second}"
    );
    assert_eq!(turns[0]["thread"], turns[1]["thread"], "in the conversation it asked in");
    let done = card(&rig, &id).await;
    assert_eq!(done["status"], "review");
    assert_eq!(question(&rig, &id).await, None, "the question is off the card");
    assert_eq!(git(&remote, &["log", "--format=%s", &branch]), format!("{id}: Add a cart\nfirst"));
    let journey = history(&rig, &id, "conductor moved it from doing to review").await;
    let at = |line: &str| {
        journey.iter().position(|one| one == line).unwrap_or_else(|| panic!("{line}: {journey:?}"))
    };
    assert!(
        at(asking) < at("Ada said: Without tax.")
            && at("Ada said: Without tax.") < at("that answered what shop-worker-1 asked")
            && at("that answered what shop-worker-1 asked") < at("shop-worker-1 worked on it")
    );
}

#[tokio::test]
async fn an_answer_that_is_marked_public_or_said_with_a_key_answers_as_any_other_does() {
    let rig = rig_whose_agent(ASKS_FIRST).await;
    shop(&rig, json!({})).await;
    let id = asked(&rig).await;
    // Who else is told of what is said on a card, and that it was a key
    // of somebody's that said it, are nothing to the card: what anybody
    // but the asker says while it asks is the answer.
    let made = rig.send("/api/credentials", json!({ "name": "notes", "scopes": ["board"] })).await;
    let key = made["token"].as_str().unwrap().to_string();
    let now = rig.mark();
    let words = json!({ "body": "Without tax.", "public": true });
    let (status, on) = rig
        .call(reqwest::Method::POST, &format!("/api/records/{id}/comments"), Some(&key), Some(words))
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{on}");
    let answered = rig.event("card.answered", move |event| event.seq > now).await;
    assert_eq!(
        answered.data,
        json!({ "card": id, "project": "shop", "asker": "shop-worker-1", "body": "Without tax." })
    );
    moved(&rig, &id, "review", now).await;
    assert_eq!(question(&rig, &id).await, None, "the question is off the card");
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "{turns:?}");
    assert!(turns[1]["prompt"].as_str().unwrap().contains("The answer:\n\nWithout tax."), "{turns:?}");
    let journey = history(&rig, &id, "conductor moved it from doing to review").await;
    let said = "Ada (with the key notes) said: Without tax. (public)".to_string();
    assert!(journey.contains(&said), "{journey:?}");
    assert_eq!(rig.count_of("card.answered").await, 1);
}

#[tokio::test]
async fn what_whoever_asked_says_on_the_card_and_what_a_blueprint_says_there_answer_nothing() {
    let rig = rig_whose_agent(ASKS_FIRST).await;
    shop(&rig, json!({})).await;
    let id = asked(&rig).await;
    let now = rig.mark();

    // The agent that asked says more on its card, with its own credential.
    let worker = std::fs::read_to_string(rig.dir.join("data/slots/shop-worker-1/token")).unwrap();
    let more = json!({ "body": "I mean the total on the receipt." });
    let (status, answer) = rig
        .call(reqwest::Method::POST, &format!("/api/records/{id}/comments"), Some(worker.trim()), Some(more))
        .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    // And a blueprint says something there, for whoever applied it.
    let notes = format!(
        "blueprint: notes\npermissions: [board]\nsteps:\n  note:\n    do: record.comment\n    with: {{ id: {id}, body: \"Still waiting on an answer.\" }}\n"
    );
    let applied = rig.send("/api/apply", json!({ "source": notes, "project": "shop", "wait": true })).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");

    // Both are said, in the card's history (what the blueprint says, as the factory's bot), and the card goes on waiting.
    let journey = history(&rig, &id, "conductor said: Still waiting on an answer.").await;
    assert!(
        journey.contains(&"shop-worker-1 said: I mean the total on the receipt.".to_string()),
        "{journey:?}"
    );
    // Each was looked at by what looks after the cards, and neither taken
    // for the answer: a person's, said after them, is the first that is.
    say(&rig, &id, "Without tax.").await;
    let answered = rig.event("card.answered", move |event| event.seq > now).await;
    assert_eq!(
        answered.data,
        json!({ "card": id, "project": "shop", "asker": "shop-worker-1", "body": "Without tax." })
    );
    moved(&rig, &id, "review", now).await;
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "one round for the question, one for the answer: {turns:?}");
    assert_eq!(rig.count_of("card.answered").await, 1);
}

#[tokio::test]
async fn what_is_said_the_instant_a_question_is_asked_either_answers_it_or_leaves_the_card_asking() {
    let rig = rig_whose_agent(ASKS_FIRST).await;
    shop(&rig, json!({})).await;
    let added = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = added["id"].as_str().unwrap().to_string();
    move_card(&rig, &id, "ready", None).await;

    // Said the moment the round says its agent asked: before the question
    // is on the card, as no person could, and while the round that asked
    // has still to finish.
    let asked = rig.event("card.asked", |_| true).await.seq;
    say(&rig, &id, "Without tax.").await;
    // What looks after the cards has by now looked at both.
    questioned(&rig, &id, asked).await;
    // What the person said, and not the question said on the card for whoever is told (`std/told`).
    let said = rig
        .event("record.commented", move |event| event.seq > asked && event.data["through"].is_null())
        .await
        .seq;
    rig.event("run.finished", move |event| {
        event.seq > said && event.data["blueprint"] == "std/cards" && event.data["flow"] == "on.said"
    })
    .await;

    // Whichever it took first, the card is not left with its question
    // gone and nobody at work on it: either what was said was the answer,
    // and the card goes round again with it, or the question is still on
    // the card for the next thing said to answer.
    if question(&rig, &id).await.is_some() {
        assert_eq!(rig.count_of("card.answered").await, 0);
        say(&rig, &id, "Without tax.").await;
    }
    moved(&rig, &id, "review", asked).await;
    assert_eq!(rig.count_of("card.answered").await, 1);
    assert_eq!(question(&rig, &id).await, None);
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "{turns:?}");
    assert!(turns[1]["prompt"].as_str().unwrap().contains("The answer:\n\nWithout tax."), "{}", turns[1]);
}

#[tokio::test]
async fn what_is_said_on_a_card_nobody_asked_anything_on_starts_no_round() {
    let rig = rig().await;
    shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    say(&rig, &id, "Looks good so far.").await;
    history(&rig, &id, "Ada said: Looks good so far.").await;

    // Once the card is done, all the stage ever began for it is there to count.
    finished(&rig, &id).await;
    assert_eq!(runs_of(&rig, "answered").await, 0);
    assert_eq!(turns(&rig).await.len(), 1);
}

#[tokio::test]
async fn a_card_sent_back_with_nothing_said_whose_agent_asks_why_is_not_handed_on_for_review_again() {
    let rig = rig_whose_agent(ASKS_WHY).await;
    let (dir, _) = shop(&rig, json!({})).await;
    let id = worked(&rig, "Add a cart").await;

    let now = rig.mark();
    move_card(&rig, &id, "doing", None).await;
    questioned(&rig, &id, now).await;
    let waiting = card(&rig, &id).await;
    assert_eq!(
        (waiting["status"].as_str(), waiting["more"]["question"].as_str()),
        (Some("doing"), Some("What is wrong with it?"))
    );
    history(&rig, &id, "shop-worker-1 asked: What is wrong with it?").await;
    assert_eq!(
        git(&dir, &["log", "--format=%s", &format!("lane/{id}")]),
        format!("{id}: Add a cart\nfirst"),
        "the round that asked changed nothing"
    );

    // Put back in ready by a person instead of answered on the card: that
    // is an answer too. What looks after the cards takes the question
    // off, and what was said as it was moved is what the agent is told.
    let now = rig.mark();
    move_card(&rig, &id, "ready", Some("The total is wrong.")).await;
    questioned(&rig, &id, now).await;
    // Said with what happened to the card, should this not hold: it has
    // failed once on a slow machine and what led to it was not kept.
    let (_, happened) = rig.get(&format!("/api/records/{id}/history")).await;
    assert_eq!(question(&rig, &id).await, None, "{happened}");
    moved(&rig, &id, "review", now).await;
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 3);
    assert!(turns[2]["prompt"].as_str().unwrap().contains("The total is wrong."), "{}", turns[2]["prompt"]);
    assert_eq!(rig.count_of("card.answered").await, 0, "nothing was said on the card");
}

#[tokio::test]
async fn what_is_said_on_a_card_that_asked_as_the_daemon_goes_is_its_answer_when_the_daemon_is_back() {
    let mut rig = rig_whose_agent(ASKS_FIRST).await;
    shop(&rig, json!({})).await;
    let id = asked(&rig).await;

    // Somebody answers, and the daemon goes in that instant: what was said
    // is the last thing the journal keeps, and nothing has heard it. What
    // is said on a record is kept as its saying and no other way, so
    // there is nothing of it that could be written and never said.
    let journal = rig.node().app.journal.clone();
    // What the person says, and not the question said on the card for whoever is told (`std/told`).
    journal.shut_after(|event| event.topic == "record.commented" && event.data["through"].is_null());
    // Whoever asks anything of a journal that has shut may never be answered.
    tokio::select! {
        _ = say(&rig, &id, "Without tax.") => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;
    rig.restart().await;

    // What looks after the board's cards takes it up where the journal
    // has it: it is the answer, and the card goes round again told it, once.
    moved(&rig, &id, "review", 0).await;
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "{turns:?}");
    let second = turns[1]["prompt"].as_str().unwrap();
    assert!(second.contains("The answer:\n\nWithout tax."), "{second}");
    assert_eq!(question(&rig, &id).await, None, "the question is off the card");
}

#[tokio::test]
async fn a_question_answered_as_the_daemon_goes_before_its_round_is_over_is_asked_once_and_stays_answered() {
    let mut rig = rig_whose_agent(ASKS_FIRST).await;
    shop(&rig, json!({})).await;
    let added = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = added["id"].as_str().unwrap().to_string();
    move_card(&rig, &id, "ready", None).await;

    // Its agent asks, and somebody answers at once: the round that asked
    // is still putting its things away, and the daemon goes with the
    // answer the last thing kept. The round is not over, so it is begun
    // again when the daemon is back.
    let its = id.clone();
    rig.event("card.asked", move |event| event.data["card"] == its.as_str()).await;
    let journal = rig.node().app.journal.clone();
    // What the person says, and not the question said on the card for whoever is told (`std/told`).
    journal.shut_after(|event| event.topic == "record.commented" && event.data["through"].is_null());
    // Whoever asks anything of a journal that has shut may never be answered.
    tokio::select! {
        _ = say(&rig, &id, "Without tax.") => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;
    rig.restart().await;

    // The answer is taken, and the card goes round again told it. The
    // round begun again does not ask the question a second time: said
    // again after it was answered, it was back on the card for good.
    let handed_on = moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", move |event| event.seq > handed_on && event.data["flow"] == "on.go").await;
    let turns = turns(&rig).await;
    assert_eq!(turns.len(), 2, "{turns:?}");
    assert!(
        turns[1]["prompt"].as_str().unwrap().contains("The answer:\n\nWithout tax."),
        "{}",
        turns[1]["prompt"]
    );
    assert_eq!(rig.count_of("card.asked").await, 1, "it was asked once");
    assert_eq!(question(&rig, &id).await, None, "the question is off the card");
}
