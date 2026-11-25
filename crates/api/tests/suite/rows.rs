//! A card is a row of its board's table, and nothing runs for one that
//! nobody is doing anything with.
//!
//! What looks after cards is one instance for a board and one for each
//! stage of a fleet. A card that is added, and a card that waits its turn
//! where cards are worked, starts nothing: what exists for one card is the
//! work in hand, for as long as it is in hand. What a card that waits says
//! of itself, and what a pool that is paused or given more places does for
//! it, is the same as when each card had something running for it.
//!
//! The agent here does not finish a card until the test lets it.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::fleets::{add, change, shop};
use crate::lanes::{history, move_card, moved};
use crate::{Rig, soon};

/// What a worker does first that waits for its card's gate: it reads what
/// it is asked, picks out which card that is of (`Card <id>: ...`), and
/// waits at a hold called after the card (`gate-<card>`) until the test
/// lets it go.
const AT_ITS_GATE: [&str; 7] = ["read", "pick", "Card ", ":", "held", "$DIR/gate-%:picked%", "%:picked%"];

/// What a worker does that changes a file and says so.
const CHANGES: [&str; 5] = ["append", "CHANGES.md", "changed", "say", "I changed a file."];

/// A node whose worker waits at its card's gate and then changes a file;
/// and the folder the gates are in.
async fn rig() -> (Rig, PathBuf) {
    rig_whose_worker(&[AT_ITS_GATE.as_slice(), &CHANGES].concat()).await
}

/// A node whose worker does this, with `$DIR` the folder of the test's
/// own: what the stand-in for a program is told (`crates/hosting/standin`).
pub(crate) async fn rig_whose_worker(does: &[&str]) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-rows", |config, dir| {
        let does: Vec<String> =
            does.iter().map(|word| word.replace("$DIR", &dir.to_string_lossy())).collect();
        let does: Vec<&str> = does.iter().map(String::as_str).collect();
        config.adapters.push(("writer".into(), crate::standin::argv(&does)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

/// A card, and the hold its agent will wait at.
async fn card_with_a_gate(rig: &Rig, at: &Path, title: &str) -> String {
    let id = add(rig, title, &[]).await;
    let pipe = at.join(format!("gate-{id}"));
    crate::hold(&pipe);
    id
}

/// Lets the agent that has this card finish.
async fn let_go(at: &Path, id: &str) {
    let pipe = at.join(format!("gate-{id}"));
    crate::let_go("an agent to be waiting at the card's gate", &pipe).await;
}

/// What is alive in the project: each instance's blueprint and key, in
/// order. But for the small things a stage calls as it comes up (what is
/// wrong with its row, the look at its column, what was left in hand):
/// each is over in a moment, and may still be there as the fleet's apply
/// answers, which is not the fleet at rest.
pub(crate) async fn alive(rig: &Rig) -> Vec<(String, String)> {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    let passing = crate::PASSING;
    let mut alive: Vec<(String, String)> = all["instances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|one| (text(&one["blueprint"]), text(&one["key"])))
        .filter(|(blueprint, _)| !passing.contains(&blueprint.as_str()))
        .collect();
    alive.sort();
    alive
}

/// Where a card is, and what it says it waits for.
async fn stands(rig: &Rig, id: &str) -> (String, String) {
    let card = crate::fleets::card(rig, id).await;
    let text = |value: &Value| value.as_str().unwrap_or_default().to_string();
    (text(&card["status"]), text(&card["more"]["waiting"]))
}

/// Waits until a card has last said what it waits for, or that it waits
/// for nothing, after `after` in the journal.
async fn said_what_it_waits_for(rig: &Rig, id: &str, after: u64) {
    let card = id.to_string();
    rig.event("board.item.updated", move |event| {
        event.seq > after && event.data["id"] == card.as_str() && event.data["changed"] == json!(["waiting"])
    })
    .await;
}

/// The pool the stage that works cards asks: who holds a place in it and
/// who waits for one.
async fn pool(rig: &Rig) -> (Vec<String>, Vec<String>) {
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let mut found = None;
    for one in all["instances"].as_array().unwrap().iter().filter(|one| one["blueprint"] == "std/pool") {
        let one =
            rig.read(&format!("/api/instances/{}", one["id"].as_str().unwrap())).await["instance"].clone();
        if one["params"]["name"] == "shop/ready" {
            found = Some(one);
        }
    }
    let state = found.expect("the stage that works has a pool")["state"].clone();
    let names = |list: &Value| -> Vec<String> {
        list.as_array().unwrap().iter().map(|name| name.as_str().unwrap().to_string()).collect()
    };
    (names(&state["held"]), names(&state["waiting"]))
}

/// Those of `cards` that wait in the pool, in the order it keeps them.
///
/// How many wait in all is not what a fleet says of its cards. A card two
/// things found due (its stage's look at its column, and the saying of
/// its move) asked twice, and is kept among those waiting, for its next
/// turn, while it holds its place and until that turn has come and been
/// given back: so who holds a place is held to, and of those waiting the
/// cards that are to wait, in their order.
fn waiting_of(waiting: &[String], cards: &[String]) -> Vec<String> {
    let cards: Vec<String> = cards.iter().map(|id| format!("key:{id}")).collect();
    waiting.iter().filter(|name| cards.contains(name)).cloned().collect()
}

/// Waits until that pool holds no place and nobody waits for one.
///
/// A place is given back by the pool in a run of its own, which follows
/// whoever says it is given back, and a stage may ask and give back more
/// than once for one card (a card that was due as the stage first looked
/// at its column and again as it was told of the move). So what the pool
/// holds is looked at each time it says so, and not once after a run that
/// may not be the last: looked at once, on a slow machine, it still held
/// the place.
async fn pool_is_free(rig: &Rig) {
    let mut said = rig.follow_after(rig.mark(), "instance.state");
    soon("the stage's pool to hold no place and have nobody waiting", async {
        while pool(rig).await != (Vec::new(), Vec::new()) {
            said.next().await.unwrap();
        }
    })
    .await;
}

#[tokio::test]
async fn a_card_that_is_added_is_a_row_and_nothing_is_started_or_run_for_it() {
    let (rig, _at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let before = alive(&rig).await;

    let mut cards = Vec::new();
    for n in 1..=20 {
        cards.push(add(&rig, &format!("Card {n}"), &[]).await);
    }
    // Everything said of each as it was added has been written down.
    for id in &cards {
        history(&rig, id, "Ada added it to draft").await;
    }

    assert_eq!(alive(&rig).await, before, "nothing was started for a card");
    // Told by what set each run off, and not by how many runs there have
    // been: what a fleet's coming up sets off (a stage's look at its
    // column) may still be starting as the first card is added, and
    // counted, that was one run more on a machine with much else to do.
    let runs = rig.node().app.engine.recent_runs(500, None).await.unwrap();
    let for_a_card: Vec<String> = runs
        .iter()
        .filter(|run| {
            let set_off_by = run.event.to_string();
            cards.iter().any(|id| set_off_by.contains(&format!("\"{id}\"")))
        })
        .map(|run| format!("{} of {}, by {}", run.flow, run.instance, run.event))
        .collect();
    assert!(for_a_card.is_empty(), "and no handler of anything ran for one: {for_a_card:#?}");
    assert_eq!(rig.read("/api/board/shop").await["items"].as_array().unwrap().len(), 20);
}

#[tokio::test]
async fn cards_that_wait_their_turn_are_rows_that_say_so_and_only_the_one_in_hand_has_anything_running() {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let before = alive(&rig).await;

    // A dozen become ready, with one agent to work them.
    let mut cards = Vec::new();
    for n in 1..=12 {
        cards.push(card_with_a_gate(&rig, &at, &format!("Card {n}")).await);
    }
    for id in &cards {
        move_card(&rig, id, "ready", None).await;
    }
    let (first, rest) = (cards[0].clone(), &cards[1..]);
    let thread = format!("lane:{first}");
    rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await;
    for id in rest {
        said_what_it_waits_for(&rig, id, 0).await;
    }

    // The first is in hand, and its round, with the one thing of it that
    // is under way (the work) and its agent's turn in it, is all that was
    // started. Each of
    // the others is a row in ready that says it waits and what for, and a
    // name in the pool's list, in the order they became ready.
    let mut with_the_round = before.clone();
    with_the_round.push(("std/lane-round".to_string(), format!("round-{first}")));
    with_the_round.push(("std/work".to_string(), format!("segment-{first}-1")));
    with_the_round.push(("std/turn".to_string(), format!("turn-{first}")));
    with_the_round.sort();
    assert_eq!(alive(&rig).await, with_the_round);
    for id in rest {
        assert_eq!(
            stands(&rig, id).await,
            ("ready".to_string(), "one card is worked at a time here".to_string())
        );
    }
    let named = |ids: &[String]| -> Vec<String> { ids.iter().map(|id| format!("key:{id}")).collect() };
    let (held, waiting) = pool(&rig).await;
    assert_eq!((held, waiting_of(&waiting, rest)), (named(&cards[..1]), named(rest)));

    // Told to start nothing for now, each card that waits says why.
    let now = rig.mark();
    rig.send("/api/emit/pool.pause", json!({ "pool": "shop/ready", "why": "the bill is being looked at" }))
        .await;
    for id in rest {
        said_what_it_waits_for(&rig, id, now).await;
        assert_eq!(
            stands(&rig, id).await.1,
            "no work is being started here for now: the bill is being looked at"
        );
    }
    // What is in hand goes on, and nothing new begins when it is done.
    let_go(&at, &first).await;
    moved(&rig, &first, "review", now).await;
    rig.event("run.finished", move |event| {
        event.seq > now && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;
    assert_eq!(alive(&rig).await, before, "its round is over, and nothing is running for any card");
    let (held, waiting) = pool(&rig).await;
    assert_eq!((held, waiting_of(&waiting, rest)), (Vec::new(), named(rest)));

    // Going on, the one that asked first is taken, and says it waits for
    // nothing; the rest say what they wait for now.
    let now = rig.mark();
    rig.send("/api/emit/pool.resume", json!({ "pool": "shop/ready" })).await;
    moved(&rig, &cards[1], "doing", now).await;
    said_what_it_waits_for(&rig, &cards[1], now).await;
    assert_eq!(stands(&rig, &cards[1]).await, ("doing".to_string(), String::new()));
    for id in &cards[2..] {
        said_what_it_waits_for(&rig, id, now).await;
        assert_eq!(stands(&rig, id).await.1, "one card is worked at a time here");
    }

    // A second agent in the role is a second place: the next in line is
    // taken beside the one in hand, as soon as the row says so.
    let now = rig.mark();
    change(&rig, "agents", "worker", json!({ "count": 2 })).await;
    moved(&rig, &cards[2], "doing", now).await;
    let (held, waiting) = pool(&rig).await;
    assert_eq!((held, waiting_of(&waiting, &cards[3..])), (named(&cards[1..3]), named(&cards[3..])));
    assert_eq!(stands(&rig, &cards[3]).await.0, "ready");

    // Both go on to review. (Nothing more is begun meanwhile, so that the
    // test leaves no agent waiting at a gate.)
    let now = rig.mark();
    rig.send("/api/emit/pool.pause", json!({ "pool": "shop/ready", "why": "that is enough" })).await;
    rig.event("pool.paused", move |event| event.seq > now).await;
    let_go(&at, &cards[1]).await;
    let_go(&at, &cards[2]).await;
    moved(&rig, &cards[1], "review", now).await;
    moved(&rig, &cards[2], "review", now).await;
    assert_eq!(stands(&rig, &cards[3]).await.0, "ready");
}

#[tokio::test]
async fn where_the_board_keeps_nothing_of_whose_a_card_is_no_card_is_begun_and_the_fleet_says_what_to_do() {
    let (rig, _at) = rig().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // A board of the project's own, written before a stage kept anything on a card.
    let board = "blueprint: cards
permissions: [projects, board]
steps:
  board:
    do: collection.ensure
    with:
      name: board
      model: card
      fields:
        title: { type: text, required: true }
        status: { type: choice, of: [draft, ready, doing, review, done], required: true }
      views:
        board: { kind: board, by: status }
";
    let made = rig.send("/api/apply", json!({ "source": board, "project": "shop", "wait": true })).await;
    assert_eq!(made["finished"]["status"], "ok", "{made}");
    crate::fleets::fleet(&rig, json!({ "reviewers": 0 })).await;

    let added = rig.send("/api/board/shop/items", json!({ "title": "Add a cart" })).await;
    let id = added["id"].as_str().unwrap().to_string();
    move_card(&rig, &id, "ready", None).await;
    let told = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("was not begun")
        })
        .await;
    let body = told.data["body"].as_str().unwrap();
    assert!(
        body.starts_with(&format!("{id} was not begun: the project's board keeps no `taken` of its cards"))
            && body.contains("Apply `std/board` to the project again, or give a board of the project's own the fields `taken` (text) and `again` (list)"),
        "{body}"
    );
    // The card is where it was, nobody was asked, and its place is free.
    pool_is_free(&rig).await;
    assert_eq!(stands(&rig, &id).await.0, "ready");
    assert_eq!(rig.count_of("agent.turn.started").await, 0);
}

#[tokio::test]
async fn a_card_that_waits_on_another_is_still_begun_when_that_one_is_done_though_what_remembered_it_was_started_anew()
 {
    let (rig, at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let first = card_with_a_gate(&rig, &at, "Add a cart").await;
    let second = add(&rig, "Add a till", &[&first]).await;
    crate::hold(&at.join(format!("gate-{second}")));
    move_card(&rig, &second, "ready", None).await;
    said_what_it_waits_for(&rig, &second, 0).await;
    assert_eq!(stands(&rig, &second).await, ("ready".to_string(), format!("after {first}, not done yet")));

    // What looks after the board's cards is stopped, and the board applied
    // again starts another, which remembers nothing.
    let all = rig.read("/api/instances?live=true&project=shop").await;
    let of = |blueprint: &str| -> String {
        let one = all["instances"].as_array().unwrap().iter().find(|one| one["blueprint"] == blueprint);
        one.unwrap()["id"].as_str().unwrap().to_string()
    };
    rig.send(&format!("/api/instances/{}/stop", of("std/cards")), json!({})).await;
    let now = rig.mark();
    let again =
        rig.send(&format!("/api/instances/{}/apply", of("std/board")), json!({ "force": true })).await;
    assert_eq!(again["changed"], true, "{again}");
    // It looks for itself, as it starts, at the cards that say they wait
    // on others, and asks each again.
    rig.event("run.finished", move |event| {
        event.seq > now && event.data["blueprint"] == "std/cards" && event.data["flow"] == "on.due"
    })
    .await;

    // The first is finished by a person: the second is begun.
    let now = rig.mark();
    move_card(&rig, &first, "done", None).await;
    moved(&rig, &second, "doing", now).await;
    let_go(&at, &second).await;
    moved(&rig, &second, "review", now).await;
}

/// What an agent was asked about a card, oldest first.
async fn asked_of(rig: &Rig, agent: &str, id: &str) -> Vec<String> {
    let turns = rig.read(&format!("/api/agents/{agent}/turns")).await;
    let mut asked: Vec<String> = turns["turns"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|turn| turn["thread"] == format!("lane:{id}").as_str())
        .map(|turn| turn["prompt"].as_str().unwrap().to_string())
        .collect();
    asked.reverse();
    asked
}

#[tokio::test]
async fn work_that_could_not_be_committed_does_not_go_on_and_its_worker_is_given_it_again_and_told_what_git_said()
 {
    // A worker that, told its work could not be committed, puts right what was in the way.
    // Told that its work could not be committed, it puts that right
    // first; either way it changes a file.
    let puts_right = ["run", "(", "git", "config", "commit.gpgsign", "false", ")"];
    let told = ["read", "where", "could not be committed", "("];
    let (rig, _at) =
        rig_whose_worker(&[&told[..], &puts_right[..], &CHANGES[..], &[")"][..], &CHANGES[..]].concat())
            .await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    // Every commit is to be signed, with a program that is not there.
    crate::lanes::git(&dir, &["config", "commit.gpgsign", "true"]);
    crate::lanes::git(&dir, &["config", "gpg.program", "/nowhere/to/sign/with"]);

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    moved(&rig, &id, "review", 0).await;

    // Twice asked, in one conversation: the second time told why it was given the card again.
    let asked = asked_of(&rig, "shop-worker-1", &id).await;
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(asked[0].starts_with("Please take this card."), "{}", asked[0]);
    assert!(
        asked[1].starts_with("What you changed on this card could not be committed")
            && asked[1].contains("What git said:")
            && asked[1].contains("/nowhere/to/sign/with"),
        "{}",
        asked[1]
    );
    // The card says what happened to it, and its work is kept.
    let said = history(&rig, &id, "conductor moved it from doing to review").await;
    let unkept: Vec<&String> = said.iter().filter(|line| line.contains("could not be committed")).collect();
    assert_eq!(unkept.len(), 1, "{said:?}");
    assert!(
        unkept[0].starts_with("what was changed could not be committed: git exited with code"),
        "{said:?}"
    );
    assert_eq!(
        crate::lanes::git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]),
        format!("{id}: Add a cart")
    );
    let told = crate::lanes::said(&rig).await;
    assert!(told.contains("shop-worker-1 is given the card again, to put that right."), "{told}");
}

#[tokio::test]
async fn where_git_has_no_name_to_commit_as_the_card_is_left_for_a_person_at_once_and_nobody_is_asked_again()
{
    let (rig, _at) = rig_whose_worker(&[["swallow"].as_slice(), &CHANGES].concat()).await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    // Nobody to commit as, whatever the device says elsewhere.
    crate::lanes::git(&dir, &["config", "user.name", ""]);

    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "ready", None).await;
    let left = rig
        .event("channel.message", |event| {
            event.data["body"].as_str().unwrap_or_default().contains("could not be committed")
        })
        .await;
    let body = left.data["body"].as_str().unwrap();
    assert!(
        body.contains("Git does not know who to commit as, which no agent can put right")
            && body.ends_with("The card is left in doing for a person."),
        "{body}"
    );
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;
    assert_eq!(stands(&rig, &id).await.0, "doing");
    assert_eq!(asked_of(&rig, "shop-worker-1", &id).await.len(), 1, "asked once, and not again");
    // And its place is free.
    pool_is_free(&rig).await;
}

#[tokio::test]
async fn a_card_that_waits_its_turn_when_the_daemon_is_stopped_still_waits_when_it_is_back_and_is_given_its_place()
 {
    // The worker here waits at one hold each time it is asked.
    let waits = ["swallow", "held", "$DIR/hold", "worker"];
    let (mut rig, at) = rig_whose_worker(&[&waits[..], &CHANGES[..]].concat()).await;
    crate::hold(&at.join("hold"));
    shop(&rig, json!({ "reviewers": 0 })).await;
    let (first, second) = (add(&rig, "Add a cart", &[]).await, add(&rig, "Add a till", &[]).await);
    move_card(&rig, &first, "ready", None).await;
    let thread = format!("lane:{first}");
    rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await;
    move_card(&rig, &second, "ready", None).await;
    said_what_it_waits_for(&rig, &second, 0).await;
    let named = |id: &String| vec![format!("key:{id}")];
    let (held, waiting) = pool(&rig).await;
    assert_eq!((held, waiting_of(&waiting, std::slice::from_ref(&second))), (named(&first), named(&second)));

    rig.restart().await;

    // A name is no instance, and is not gone because the daemon was: the
    // card in hand has its place still, and the one that waited waits.
    let now = rig.mark();
    let (held, waiting) = pool(&rig).await;
    assert_eq!((held, waiting_of(&waiting, std::slice::from_ref(&second))), (named(&first), named(&second)));
    assert_eq!(
        stands(&rig, &second).await,
        ("ready".to_string(), "one card is worked at a time here".to_string())
    );
    // Whoever is asked from now is let go. The first goes on to review,
    // and the second is given the place and worked.
    crate::pass(&at.join("hold"), 16);
    moved(&rig, &first, "review", now).await;
    moved(&rig, &second, "doing", now).await;
    moved(&rig, &second, "review", now).await;
    pool_is_free(&rig).await;
}

#[tokio::test]
async fn a_card_whose_move_to_done_was_written_and_never_said_is_tidied_when_the_daemon_is_next_started() {
    let (mut rig, at) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let id = card_with_a_gate(&rig, &at, "Add a cart").await;
    move_card(&rig, &id, "ready", None).await;
    let_go(&at, &id).await;
    moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", |event| {
        event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go"
    })
    .await;
    assert!(dir.join(".lanes").join(&id).exists());

    // As when a daemon goes in the instant a person finishes a card: the
    // card's row says done, and nothing was ever said of its being moved.
    let studio = &rig.node().app.studio;
    let (board, mut record) = studio.collections.record(&id).await.unwrap();
    record.data.insert("status".into(), json!("done"));
    studio.collections.put(&board, record).await.unwrap();
    let before = rig.mark();

    rig.restart().await;

    // What was written and never said is said when the daemon is back,
    // late, and the stage that worked the card hears that it is finished
    // and tidies its place away.
    rig.event("run.finished", move |event| {
        event.seq > before && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.tidied"
    })
    .await;
    assert!(!dir.join(".lanes").join(&id).exists());
    let card = crate::fleets::card(&rig, &id).await;
    assert_eq!((&card["status"], card["more"]["taken"].as_str().unwrap_or_default()), (&json!("done"), ""));
    assert_eq!(
        rig.count_of("board.item.moved").await,
        4,
        "to ready, to doing and to review, and to done once"
    );
    let its = id.clone();
    let late = rig
        .event("board.item.moved", move |event| {
            event.data["id"] == its.as_str() && event.data["to"] == "done"
        })
        .await;
    assert_eq!((late.data["from"].clone(), late.data["late"].clone()), (json!("review"), json!(true)));
}

#[tokio::test]
async fn a_card_said_to_be_due_twice_whose_agent_asks_something_is_asked_once() {
    due_twice_and_asked_once(true).await;
}

/// The order that seldom comes of itself where there are processors to
/// spare, made to come: the card is said to be due a second time once its
/// round has taken it.
#[tokio::test]
async fn a_card_said_to_be_due_again_once_its_round_has_taken_it_is_asked_once_all_the_same() {
    due_twice_and_asked_once(false).await;
}

/// A card said to be due twice, in one moment (`together`) or the second
/// time once its round has taken it, whose agent asks something.
async fn due_twice_and_asked_once(together: bool) {
    // The worker waits to be let go, and then asks instead of working.
    let asks = ["say", "Which total: with tax, or without?", "say", "DECISION: ask"];
    let (rig, at) = rig_whose_worker(&[AT_ITS_GATE.as_slice(), &asks].concat()).await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let id = card_with_a_gate(&rig, &at, "Add a cart").await;
    // In the column with nothing said of it, as a card is that was there
    // before its stage looked, or was put there as the daemon went down.
    let studio = &rig.node().app.studio;
    let (board, mut record) = studio.collections.record(&id).await.unwrap();
    record.data.insert("status".into(), json!("ready"));
    studio.collections.put(&board, record).await.unwrap();
    // And said to be due twice, as it is when the stage's look at its
    // column and the saying of the move both find it.
    let due = format!(
        "{{ do: emit, with: {{ topic: stage.due, card: {id}, project: shop, stage: ready, note: \"\" }} }}"
    );
    let say = |steps: String| {
        let text = format!("blueprint: due-twice\nsteps:\n{steps}");
        json!({ "project": "shop", "source": text, "wait": true })
    };
    if together {
        let said = rig.send("/api/apply", say(format!("  one: {due}\n  two: {due}\n"))).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
    } else {
        let said = rig.send("/api/apply", say(format!("  one: {due}\n"))).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
        moved(&rig, &id, "doing", 0).await;
        let said = rig.send("/api/apply", say(format!("  one: {due}\n"))).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
    }

    // The stage looks at the card once for each time it was said to be
    // due. Which of two things comes first is not the test's to say: a
    // look, or the round an earlier look led to taking the card. Looked
    // at while it is still in its column, the card's turn is asked for
    // again, and waits behind the one it has. Looked at after its round
    // has taken it, it is due nothing and nothing is asked. And how often
    // it is said to be due is not the test's alone: the stage's own look
    // at its column, as it comes up, may find the card there too. This
    // waited for a second asking whatever came first, and where the round
    // was the quicker (once in a gate held to four processors) it never
    // came.
    //
    // So it waits until the card's round has taken it and every saying
    // there has been was looked at. From then a look finds the card
    // taken, and how often its turn was asked for is settled.
    moved(&rig, &id, "doing", 0).await;
    let due = looked_at_each_time(&rig, &id).await;
    let asked = turns_asked(&rig, &id).await;
    eprintln!("ORDER: said to be due {due} times, {asked} of them before its round had taken it");
    assert!((1..=due).contains(&asked), "asked for {asked} times, said to be due {due} times");
    assert!(together || asked < due, "said to be due once it was taken, and its turn was asked for");
    let thread = format!("lane:{id}");
    rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await;

    // Its agent has it, and what was asked beyond the first waits behind
    // the turn it has, once the pool has taken each asking in.
    let mut taken_in = rig.follow_after(rig.mark(), "run.finished");
    soon("the pool to have taken in each time the card's turn was asked for", async {
        while runs_over(&rig, "std/pool", "on.asked").await < asked {
            taken_in.next().await.unwrap();
        }
    })
    .await;
    // Asked again while it waits already, it waits the once: what it
    // waits for is its next turn, however often that was asked for.
    let turns = asked.min(2);
    let key = format!("key:{id}");
    let (held, waiting) = pool(&rig).await;
    assert_eq!(held, std::slice::from_ref(&key), "it holds the one place");
    assert_eq!(waiting, vec![key.clone(); turns - 1], "and waits for its next, where one was asked for");

    // The agent asks something, and the card waits in doing for whoever
    // answers. Its next turn, where one was asked for, comes when the
    // round has given the first back, finds a card that is owed no
    // round, and is given back. Whatever the order was, nobody is asked
    // again before anybody has answered: that is what this holds.
    let_go(&at, &id).await;
    let its = id.clone();
    rig.event("card.asked", move |event| event.data["card"] == its.as_str()).await;
    pool_is_free(&rig).await;
    assert_eq!(
        rig.count_of("pool.granted").await,
        turns,
        "the turn it had, and its next where one was asked for"
    );
    assert_eq!(rig.count_of("card.asked").await, 1, "the card asked once");
    assert_eq!(crate::fleets::given(&rig, "shop-worker-1").await.len(), 1);
    assert_eq!(stands(&rig, &id).await.0, "doing");
}

/// How many times a card's turn has been asked for.
async fn turns_asked(rig: &Rig, id: &str) -> usize {
    said_of(rig, "pool.asked", "as", id).await
}

/// How many events on `topic` there have been whose `field` is `value`.
pub(crate) async fn said_of(rig: &Rig, topic: &str, field: &str, value: &str) -> usize {
    let query = journal::EventQuery { topics: topic.to_string(), limit: 10_000, ..Default::default() };
    let said = rig.said(query).await;
    said.iter().filter(|event| event.data[field] == value).count()
}

/// How many runs of a handler of a blueprint are over.
async fn runs_over(rig: &Rig, blueprint: &str, flow: &str) -> usize {
    let query =
        journal::EventQuery { topics: "run.finished".to_string(), limit: 10_000, ..Default::default() };
    let over = rig.said(query).await;
    over.iter().filter(|event| event.data["blueprint"] == blueprint && event.data["flow"] == flow).count()
}

/// Waits until a card has been looked at by its stage once for each time
/// it has been said to be due, and says how many times that is. The
/// looks are counted first and the sayings after: where the two are the
/// same, every saying there was by the first reading had been looked at,
/// and there was none between the two.
async fn looked_at_each_time(rig: &Rig, id: &str) -> usize {
    let mut over = rig.follow_after(rig.mark(), "run.finished");
    soon("the card to have been looked at each time it was said to be due", async {
        loop {
            let looked = runs_over(rig, "std/working", "on.begun").await;
            let due = said_of(rig, "stage.due", "card", id).await;
            if due >= 2 && looked == due {
                return due;
            }
            over.next().await.unwrap();
        }
    })
    .await
}

/// The reviews there have been of a card, each by its key.
async fn reviews_of(rig: &Rig, id: &str) -> Vec<String> {
    let all = rig.read("/api/instances?project=shop&all=true").await;
    let named = format!("review-{id}-");
    let mut keys: Vec<String> = all["instances"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|one| one["key"].as_str().filter(|key| key.starts_with(&named)).map(str::to_string))
        .collect();
    keys.sort();
    keys
}

#[tokio::test]
async fn a_card_said_to_be_due_a_review_twice_has_one_review_and_its_reviewer_is_asked_once() {
    let (rig, at) = crate::fleets::rig().await;
    shop(&rig, json!({})).await;
    // Whoever reviews waits to be let go, so that the review is in session.
    let pipe = at.join("hold");
    crate::hold(&pipe);
    let id = add(&rig, "Add a cart", &[]).await;
    // In the reviewing column with nothing said of it, as a card is that
    // was there before its stage looked, or was put there as the daemon
    // went down.
    let studio = &rig.node().app.studio;
    let (board, mut record) = studio.collections.record(&id).await.unwrap();
    record.data.insert("status".into(), json!("review"));
    studio.collections.put(&board, record).await.unwrap();
    // And said to be due twice, as it is when the stage's look at its
    // column and the saying of the move both find it.
    let before = rig.mark();
    let due =
        format!("{{ do: emit, with: {{ topic: review.due, card: {id}, project: shop, stage: review }} }}");
    let twice = format!("blueprint: due-twice\nsteps:\n  one: {due}\n  two: {due}\n");
    let said = rig.send("/api/apply", json!({ "project": "shop", "source": twice, "wait": true })).await;
    assert_eq!(said["finished"]["status"], "ok", "{said}");

    // The stage has heard both, and one review of the card was begun.
    let heard = std::sync::atomic::AtomicUsize::new(0);
    rig.event("run.finished", move |event| {
        let due = event.seq > before
            && event.data["blueprint"] == "std/reviewing"
            && event.data["flow"] == "on.due";
        due && heard.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= 1
    })
    .await;
    assert_eq!(reviews_of(&rig, &id).await.len(), 1, "one review of the card");

    // Let go, its reviewer approves it, and was asked once.
    crate::let_go("a reviewer to be waiting", &pipe).await;
    moved(&rig, &id, "done", before).await;
    assert_eq!(crate::fleets::given(&rig, "shop-reviewer-1").await.len(), 1);
}

#[tokio::test]
async fn a_card_taken_out_of_review_while_it_is_looked_at_and_put_straight_back_is_given_a_new_review() {
    let (rig, at) = crate::fleets::rig().await;
    shop(&rig, json!({})).await;
    let pipe = at.join("hold");
    crate::hold(&pipe);
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "review", None).await;
    let seat = format!("review:{id}:");
    let looking =
        |event: &kernel::Event| event.data["thread"].as_str().is_some_and(|of| of.starts_with(&seat));
    let thread = rig.event("agent.turn.started", looking).await.seq;

    // Out and straight back: the review it had ends itself, and its
    // stage, which heard it leave before it heard it come, begins another.
    move_card(&rig, &id, "draft", None).await;
    move_card(&rig, &id, "review", None).await;
    let back = moved(&rig, &id, "review", thread).await;
    rig.event("run.finished", move |event| {
        event.seq > back && event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.due"
    })
    .await;
    assert_eq!(reviews_of(&rig, &id).await.len(), 2, "the one it had, and the one begun as it came back");

    crate::let_go("a reviewer to be waiting", &pipe).await;
    moved(&rig, &id, "done", back).await;
}

/// A way of reviewing of the project's own: it passes the card as it is.
pub(crate) const LOOKED_OVER: &str = r#"blueprint: looked-over
description: Pass one card as it is.
permissions: [board]

params:
  project: { type: string }
  item: { type: string }
  stage: { type: string, default: review }
  role: { type: string, default: reviewer }
  picks: { type: string, default: first }
  seats: { type: int, default: 1 }
  needed: { type: int, default: 0 }
  pool: { type: string, default: "" }
  arranged: { type: string, default: "" }

steps:
  passed:
    do: emit
    with:
      topic: review.passed
      card: "${params.item}"
      project: "${params.project}"
      by: nobody
      words: looked over
      approved: 1
      seats: 1
"#;

#[tokio::test]
async fn a_review_that_could_not_be_begun_is_not_kept_as_under_way_and_the_card_is_reviewed_when_it_can_be() {
    let (rig, _at) = crate::fleets::rig().await;
    // Cards are reviewed by a blueprint of the project's own, which is
    // not there yet.
    shop(&rig, json!({})).await;
    let mut params = crate::fleets::params(json!({}));
    params["review"] = json!("looked-over");
    let fleet = json!({ "blueprint": "std/fleet", "project": "shop", "params": params, "wait": true });
    let applied = rig.send("/api/apply", fleet).await;
    assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "review", None).await;
    let failed = rig
        .event("run.failed", |event| {
            event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.due"
        })
        .await;
    assert!(failed.data["reason"].as_str().unwrap_or_default().contains("looked-over"), "{}", failed.data);

    // Written, and the card put in review again, it is reviewed: the
    // review that never began was not kept as one under way.
    let saved = json!({
        "path": format!("@blueprints/{}", brand::blueprint_file("looked-over")), "content": LOOKED_OVER,
    });
    let (status, answer) =
        rig.call(reqwest::Method::PUT, "/api/projects/shop/file", Some(&rig.token), Some(saved)).await;
    assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    move_card(&rig, &id, "draft", None).await;
    move_card(&rig, &id, "review", None).await;
    moved(&rig, &id, "done", failed.seq).await;
}

#[tokio::test]
async fn a_review_that_is_stopped_with_nothing_said_is_no_longer_under_way_and_its_card_is_reviewed_again_when_it_is_due()
 {
    let (rig, at) = crate::fleets::rig().await;
    shop(&rig, json!({})).await;
    let pipe = at.join("hold");
    crate::hold(&pipe);
    let id = add(&rig, "Add a cart", &[]).await;
    move_card(&rig, &id, "review", None).await;
    let seat = format!("review:{id}:");
    let looking =
        |event: &kernel::Event| event.data["thread"].as_str().is_some_and(|of| of.starts_with(&seat));
    rig.event("agent.turn.started", looking).await;

    // Somebody stops the review while it is in session. It says nothing
    // of how it came out, and the card stays where it is.
    let all = rig.read("/api/instances?project=shop&all=true").await;
    let key = reviews_of(&rig, &id).await.remove(0);
    let review = all["instances"].as_array().unwrap().iter().find(|one| one["key"] == key.as_str()).unwrap();
    let stopped = rig.mark();
    rig.send(&format!("/api/instances/{}/stop", review["id"].as_str().unwrap()), json!({})).await;
    let its = review["id"].as_str().unwrap().to_string();
    rig.event("instance.finished", move |event| event.source.instance.as_deref() == Some(its.as_str())).await;
    // The stage has taken that in when the handler that hears a review
    // of its own end has run: it takes no turn by the card, so a card
    // said to be due in the moment before it has is still one with a
    // review under way, and is given no other.
    rig.event("run.finished", move |event| {
        event.seq > stopped && event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.ended"
    })
    .await;

    // Said to be due again, as it is when a role that had nobody has
    // somebody, it is given a new review: the one that was stopped is not
    // kept as under way.
    let due =
        format!("{{ do: emit, with: {{ topic: review.due, card: {id}, project: shop, stage: review }} }}");
    let once = format!("blueprint: due-again\nsteps:\n  one: {due}\n");
    let said = rig.send("/api/apply", json!({ "project": "shop", "source": once, "wait": true })).await;
    assert_eq!(said["finished"]["status"], "ok", "{said}");
    rig.event("run.finished", move |event| {
        event.seq > stopped && event.data["blueprint"] == "std/reviewing" && event.data["flow"] == "on.due"
    })
    .await;
    assert_eq!(reviews_of(&rig, &id).await.len(), 2, "the one that was stopped, and one begun now");
    crate::let_go("a reviewer to be waiting", &pipe).await;
    moved(&rig, &id, "done", stopped).await;
}

#[tokio::test]
async fn a_card_sent_back_as_often_as_is_allowed_and_said_to_be_due_twice_is_said_to_be_left_for_a_person_once()
 {
    let (rig, _at) = crate::fleets::rig().await;
    shop(&rig, json!({})).await;
    let id = add(&rig, "Add a cart", &[]).await;
    // In the reviewing column with nothing said of it, having been sent
    // back as often as its stage allows.
    let studio = &rig.node().app.studio;
    let (board, mut record) = studio.collections.record(&id).await.unwrap();
    record.data.insert("status".into(), json!("review"));
    record.data.insert("rounds".into(), json!(2));
    studio.collections.put(&board, record).await.unwrap();
    let before = rig.mark();
    let due =
        format!("{{ do: emit, with: {{ topic: review.due, card: {id}, project: shop, stage: review }} }}");
    let twice = format!("blueprint: due-twice\nsteps:\n  one: {due}\n  two: {due}\n");
    let said = rig.send("/api/apply", json!({ "project": "shop", "source": twice, "wait": true })).await;
    assert_eq!(said["finished"]["status"], "ok", "{said}");
    let heard = std::sync::atomic::AtomicUsize::new(0);
    rig.event("run.finished", move |event| {
        let due = event.seq > before
            && event.data["blueprint"] == "std/reviewing"
            && event.data["flow"] == "on.due";
        due && heard.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= 1
    })
    .await;

    // Nobody is asked, and that it is left for a person is said once.
    let all = rig.read("/api/channels/shop/messages?limit=200").await;
    let left: Vec<&str> = all["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|message| message["body"].as_str())
        .filter(|body| body.contains("it is left for a person to look at"))
        .collect();
    assert_eq!(left.len(), 1, "{left:?}");
    assert!(reviews_of(&rig, &id).await.is_empty());
    // And as an event of the card's, once, with how often it had been
    // sent back: what counts what comes of cards goes by that.
    let all = journal::EventQuery { topics: "card.left".into(), limit: 100, ..Default::default() };
    let said = rig.said(all).await;
    let of: Vec<_> = said.iter().map(|event| event.data.clone()).collect();
    assert_eq!(of, [json!({ "card": id, "project": "shop", "rounds": 2 })]);
    history(&rig, &id, "it was left for a person, having been sent back 2 times").await;
}

#[tokio::test]
async fn a_turn_that_comes_for_a_card_sent_back_before_anything_has_heard_of_it_has_its_worker_told_why_once()
{
    let (rig, at) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let id = card_with_a_gate(&rig, &at, "Add a cart").await;
    move_card(&rig, &id, "ready", None).await;
    let_go(&at, &id).await;
    let handed_on = moved(&rig, &id, "review", 0).await;
    rig.event("run.finished", move |event| event.seq > handed_on && event.data["flow"] == "on.go").await;
    pool_is_free(&rig).await;

    // Somebody sent it back and said why, and nothing heard of it: the
    // saying of the move was not kept, as a journal from before may hold
    // where a daemon went between writing a move and saying it. The row
    // is written as the move wrote it.
    crate::late::moved_saying_and_not_said(&rig, &id, "doing", "The total is wrong.").await;

    // A turn comes for the card all the same, as a second does for a
    // card that was said to be due twice. What it comes to is said by
    // the card's row: its worker is asked once more, and told why.
    let turn = |called: &str| {
        format!(
            "blueprint: {called}\nsteps:\n  one:\n    do: emit\n    with: {{ topic: pool.granted, pool: shop/ready, \
             to: \"key:{id}\", as: {id}, waited: false }}\n"
        )
    };
    let before = rig.mark();
    let said =
        rig.send("/api/apply", json!({ "project": "shop", "source": turn("a-turn"), "wait": true })).await;
    assert_eq!(said["finished"]["status"], "ok", "{said}");
    let_go(&at, &id).await;
    let again = moved(&rig, &id, "review", before).await;
    let told = crate::fleets::given(&rig, "shop-worker-1").await;
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(told[1].contains("The total is wrong."), "{}", told[1]);

    // Sent back the same way once more, and two turns come: the first
    // has its worker told, and the second finds a round already told of
    // that move, and nobody is asked again.
    crate::late::moved_saying_and_not_said(&rig, &id, "doing", "Still wrong.").await;
    for called in ["another-turn", "and-another"] {
        let said =
            rig.send("/api/apply", json!({ "project": "shop", "source": turn(called), "wait": true })).await;
        assert_eq!(said["finished"]["status"], "ok", "{said}");
    }
    let_go(&at, &id).await;
    let last = moved(&rig, &id, "review", again).await;
    let heard = std::sync::atomic::AtomicUsize::new(0);
    rig.event("run.finished", move |event| {
        let go =
            event.seq > again && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.go";
        go && heard.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= 1
    })
    .await;
    let _ = last;
    let told = crate::fleets::given(&rig, "shop-worker-1").await;
    assert_eq!(told.len(), 3, "{told:?}");
    assert!(told[2].contains("Still wrong."), "{}", told[2]);
}
