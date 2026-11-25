//! What becomes of a card that somebody has in hand when the daemon is
//! stopped and started again.
//!
//! It is its stage's to say, in two cells of the stage's row. Unless the row
//! says otherwise the work goes on: a card an agent is working is begun
//! again by its lane, and a review in session goes on where it was. A row
//! that says `leave` has the card left where it is for a person, and the
//! channel told why; and after so many restarts in the middle of the same
//! work it is left whatever the row says. An agent whose program outlives
//! the daemon is taken up where it is, and is asked once.
//!
//! The stand-ins here say they were asked and wait to be let go, so that
//! the daemon can be stopped while one of them has the card.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add, card, change, given, shop};
use crate::lanes::{git, history, move_card, moved};

/// The holds the stand-ins wait at, each made here before anybody is
/// asked: so many are let go at one, whether they wait already or have
/// yet to come.
pub(crate) struct Held {
    at: PathBuf,
    work: PathBuf,
    look: PathBuf,
    /// Each start of a stand-in says a line at one of these, and how
    /// many have been read of each.
    began_work: Began,
    began_look: Began,
}

struct Began {
    lines: crate::Listening,
    heard: usize,
}

impl Held {
    fn pipe(at: &std::path::Path, name: &str) -> PathBuf {
        let hold = at.join(name);
        crate::hold(&hold);
        hold
    }

    fn began(at: &std::path::Path, name: &str) -> Began {
        Began { lines: crate::Listening::at(&at.join(name)), heard: 0 }
    }

    /// Lets whoever works go, however often it is asked from now.
    pub(crate) fn work(&mut self) {
        crate::pass(&self.work, 8);
    }

    /// And whoever reviews.
    pub(crate) fn look(&mut self) {
        crate::pass(&self.look, 8);
    }

    /// How many times the program of whoever works, or reviews, was started.
    pub(crate) fn asked(&self, who: &str) -> usize {
        std::fs::read_to_string(self.at.join(format!("asked-{who}"))).unwrap_or_default().lines().count()
    }

    /// Waits until it has been started so many times: a turn is said to
    /// have begun a moment before its program is there to be stopped. Each
    /// start says so where the test listens, which is waited at: nothing is
    /// looked for, and nothing rests on what the system will say of a file.
    pub(crate) async fn until_asked(&mut self, who: &str, times: usize) {
        let began = if who == "work" { &mut self.began_work } else { &mut self.began_look };
        crate::soon(&format!("whoever is to {who} to have been asked {times} times"), async {
            while began.heard < times {
                began.lines.next_line().await.unwrap();
                began.heard += 1;
            }
        })
        .await;
    }
}

/// A node whose worker and reviewer each say they were asked and wait to
/// be let go, and what they wait at.
pub(crate) async fn rig() -> (Rig, Held) {
    let mut at = None;
    let rig = Rig::with("api-restarted", |config, dir| {
        let waits = |who: &str, then: &[&str]| {
            let file = |name: &str| dir.join(format!("{name}-{who}")).to_string_lossy().into_owned();
            let (asked, began, hold) = (file("asked"), file("began"), file("hold"));
            let first = ["swallow", "append", &asked, "asked", "tell", &began, "began", "held", &hold, who];
            crate::standin::argv(&[&first[..], then].concat())
        };
        let writes = waits("work", &["append", "CHANGES.md", "changed", "say", "I changed a file."]);
        let looks = waits("look", &["say", "It does what the card asks.", "say", "DECISION: approve"]);
        config.adapters.push(("writer".into(), writes));
        config.adapters.push(("critic".into(), looks));
        at = Some(dir.to_path_buf());
    })
    .await;
    let at = at.unwrap();
    let held = Held {
        work: Held::pipe(&at, "hold-work"),
        look: Held::pipe(&at, "hold-look"),
        began_work: Held::began(&at, "began-work"),
        began_look: Held::began(&at, "began-look"),
        at,
    };
    (Rig::owned(rig).await, held)
}

/// A card made ready, and the moment its worker has it: the card, and
/// where the journal had got to.
pub(crate) async fn in_hand(rig: &Rig, held: &mut Held) -> (String, u64) {
    let id = add(rig, "Add a cart", &[]).await;
    move_card(rig, &id, "ready", None).await;
    let thread = format!("lane:{id}");
    let asked =
        rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await.seq;
    held.until_asked("work", 1).await;
    (id, asked)
}

/// The same, worked, and the moment its reviewer has it.
pub(crate) async fn in_review(rig: &Rig, held: &mut Held) -> (String, u64) {
    let (id, _) = in_hand(rig, held).await;
    held.work();
    moved(rig, &id, "review", 0).await;
    let asked = rig.event("agent.turn.started", |event| event.data["slot"] == "shop-reviewer-1").await.seq;
    held.until_asked("look", 1).await;
    (id, asked)
}

/// What the channel was told after `seq` that holds `words`.
pub(crate) async fn told(rig: &Rig, seq: u64, words: &'static str) -> String {
    let said = rig
        .event("channel.message", move |event| {
            event.seq > seq && event.data["body"].as_str().unwrap_or_default().contains(words)
        })
        .await;
    said.data["body"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn work_in_hand_when_the_daemon_is_started_again_goes_on_and_is_one_turn_of_its_agent() {
    let (mut rig, mut held) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    rig.restart().await;

    // Nobody moves the card or says anything. Its round was over when the
    // daemon stopped; its lane begins it again, and is given the turn the
    // agent already had, which the daemon takes again: one turn, not two.
    held.work();
    moved(&rig, &id, "review", asked).await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]), format!("{id}: Add a cart"));
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 1);
    // Its program is not one that outlives the daemon, so it was started
    // again for that turn.
    assert_eq!(held.asked("work"), 2);
    // And the card's history says what happened to it, for whoever reads it later.
    history(
        &rig,
        &id,
        "the daemon was stopped while shop-worker-1 had it, and shop-worker-1 went on with it",
    )
    .await;
}

#[tokio::test]
async fn an_agent_whose_program_outlives_the_daemon_is_taken_up_where_it_is_and_asked_once() {
    let (mut rig, mut held) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "agents", "worker", json!({ "outlives": "on" })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    rig.restart().await;

    held.work();
    moved(&rig, &id, "review", asked).await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]), format!("{id}: Add a cart"));
    assert_eq!((held.asked("work"), given(&rig, "shop-worker-1").await.len()), (1, 1));
}

#[tokio::test]
async fn an_answer_read_by_a_daemon_that_went_before_it_kept_that_the_turn_ended_is_the_next_ones_and_its_agent_is_asked_once()
 {
    let (mut rig, mut held) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "agents", "worker", json!({ "outlives": "on" })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    // The daemon goes once its worker's program has answered and the
    // answer has been read, before it has kept that the turn ended: what
    // the program printed is passed on, and nothing after it is kept.
    // That was a daemon stopping while starting the program, on Windows,
    // where that takes long enough for the program to answer meanwhile:
    // what read the answer took the program's files away with it.
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "agent.output");
    held.work();
    journal.shut().await;
    rig.stop().await;
    rig.start().await;

    // The turn is in hand still, and what its program left is there to
    // be taken up: its answer is taken, and the program is not started again.
    moved(&rig, &id, "review", asked).await;
    assert_eq!(git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]), format!("{id}: Add a cart"));
    assert_eq!((held.asked("work"), given(&rig, "shop-worker-1").await.len()), (1, 1));
    let thread = format!("lane:{id}");
    let taken = rig
        .event("agent.turn.started", move |event| {
            event.data["thread"] == thread.as_str() && event.data["again"] == 1
        })
        .await;
    assert_eq!(taken.data["taken_up"], true, "{}", taken.data);
}

#[tokio::test]
async fn work_a_stage_says_to_leave_is_left_in_doing_for_a_person_and_the_channel_says_why() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "stages", "work", json!({ "after_restart": "leave" })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    rig.restart().await;

    let said = told(&rig, asked, "left as it was").await;
    assert!(
        said.contains(
            "the daemon was stopped while shop-worker-1 had this in hand, and it was asked for to be left as it \
             was then and not begun again"
        ),
        "{said}"
    );
    assert_eq!(card(&rig, &id).await["status"], "doing");
    assert_eq!(held.asked("work"), 1, "and its agent was not asked again");

    // Made ready again by a person, it is worked as any card is.
    let again = rig.mark();
    move_card(&rig, &id, "ready", None).await;
    held.work();
    moved(&rig, &id, "review", again).await;
}

#[tokio::test]
async fn work_is_left_for_a_person_after_as_many_restarts_as_its_stage_says_whatever_else_it_says() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "stages", "work", json!({ "restarts": 1 })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    // Once it goes on: its lane begins the round again, and its agent is
    // asked again. The round is waited for, and not only the agent: the
    // daemon takes the turn up again by itself as it starts, a moment
    // before the lane has begun the round that will hear what comes of it,
    // and a daemon stopped in that moment is another thing than this.
    rig.restart().await;
    rig.event("instance.started", move |event| {
        event.seq > asked && event.data["blueprint"] == "std/lane-round"
    })
    .await;
    let thread = format!("lane:{id}");
    let again = rig
        .event("agent.turn.started", move |event| {
            event.seq > asked && event.data["thread"] == thread.as_str()
        })
        .await
        .seq;
    held.until_asked("work", 2).await;
    // The second time it is left.
    rig.restart().await;

    let said = told(&rig, again, "It is left as it was for a person").await;
    assert!(said.contains("2 times now, and it is begun again after 1 such stops at most"), "{said}");
    assert_eq!(card(&rig, &id).await["status"], "doing");
}

#[tokio::test]
async fn a_review_in_session_when_the_daemon_is_started_again_goes_on_and_its_card_is_passed() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({})).await;
    let (id, asked) = in_review(&rig, &mut held).await;

    rig.restart().await;

    // The seat that was waiting for its reviewer goes on waiting, and the
    // reviewer is given what it had: nobody decides again who looks.
    held.look();
    moved(&rig, &id, "done", asked).await;
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), 1);
    history(
        &rig,
        &id,
        "the daemon was stopped while shop-reviewer-1 had it, and shop-reviewer-1 went on with it",
    )
    .await;
}

#[tokio::test]
async fn a_review_a_stage_says_to_leave_is_said_to_be_undecided_and_its_card_is_left_for_a_person() {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({})).await;
    change(&rig, "stages", "review", json!({ "after_restart": "leave" })).await;
    let (id, asked) = in_review(&rig, &mut held).await;

    rig.restart().await;

    // Whoever was looking had it in hand, and its stage says that is left:
    // nobody has decided, and that is said where a person looks, with why.
    // Nothing is passed or sent back on a guess.
    let body = told(&rig, asked, "left as it was then and not begun again").await;
    assert!(
        body.starts_with(&format!("{id}: those who looked at it neither approved it nor sent it back")),
        "{body}"
    );
    assert!(body.ends_with("It is left in review for a person."), "{body}");
    assert_eq!(card(&rig, &id).await["status"], "review");
    assert_eq!(held.asked("look"), 1, "and its reviewer was not asked again");
}

#[tokio::test]
async fn a_card_a_person_finishes_after_a_restart_stays_done_and_its_agent_is_asked_nothing_more() {
    // Its agent's program went on without the daemon. A person then moves
    // the card to done themselves: nobody is waiting for the agent any
    // more, and what it goes on to say moves the card nowhere.
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    change(&rig, "agents", "worker", json!({ "outlives": "on" })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;

    rig.restart().await;
    let before = rig.mark();
    move_card(&rig, &id, "done", None).await;

    // Its stage has heard of the move, and nothing is in hand for the card
    // any more. Whether its round had been asked for again by then or
    // not, the turn comes to an end: stopped with whoever waited for it,
    // or let finish with nobody to hear.
    rig.event("run.finished", move |event| {
        event.seq > before && event.data["blueprint"] == "std/working" && event.data["flow"] == "on.tidied"
    })
    .await;
    held.work();
    rig.event("agent.turn.finished", move |event| event.seq > asked && event.data["slot"] == "shop-worker-1")
        .await;
    assert_eq!(card(&rig, &id).await["status"], "done");
    assert_eq!((held.asked("work"), given(&rig, "shop-worker-1").await.len()), (1, 1));
}

/// Lets go whoever was asked, before the daemon stopped or since: more
/// than there can be askings.
fn let_go(hold: &std::path::Path) {
    crate::pass(hold, 32);
}

/// A node whose stand-ins of one kind wait to be let go each time one is
/// asked (`writer`, who works a card, or `critic`, who reviews one), and
/// the one hold they all wait at: for several cards in hand at once.
async fn rig_where_waits(who: &'static str) -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-restarted", |config, dir| {
        let hold = dir.join("hold").to_string_lossy().into_owned();
        // Whoever is not the one that waits is told to wait where no
        // hold is ever made, and goes on.
        let does = |name: &str, then: &[&str]| {
            let wait = ["held", if name == who { hold.as_str() } else { "nowhere" }, name];
            crate::standin::argv(&[&["swallow"][..], &wait[..], then].concat())
        };
        let writes = does("writer", &["append", "CHANGES.md", "changed", "say", "I changed a file."]);
        let looks = does("critic", &["say", "It does what the card asks.", "say", "DECISION: approve"]);
        config.adapters.push(("writer".into(), writes));
        config.adapters.push(("critic".into(), looks));
        at = Some(dir.to_path_buf());
    })
    .await;
    let hold = Held::pipe(&at.unwrap(), "hold");
    (Rig::owned(rig).await, hold)
}

#[tokio::test]
async fn several_cards_in_hand_when_the_daemon_is_started_again_all_go_on() {
    // One alone went on and two of three did not: each lane looks, as it
    // goes on, at the run of the round it had called, and whichever looked
    // before that run was said to be over waited on it for ever.
    let (mut rig, hold) = rig_where_waits("writer").await;
    let dir = crate::fleets::shop(&rig, json!({ "workers": 3, "reviewers": 0 })).await;
    let mut cards = Vec::new();
    for title in ["Add a cart", "Add a till", "Add a shelf"] {
        let id = crate::fleets::add(&rig, title, &[]).await;
        move_card(&rig, &id, "ready", None).await;
        cards.push((id, title));
    }
    // Each has its agent, in the middle of the work.
    let mut asked = 0;
    for (id, _) in &cards {
        let thread = format!("lane:{id}");
        let turn =
            rig.event("agent.turn.started", move |event| event.data["thread"] == thread.as_str()).await;
        asked = asked.max(turn.seq);
    }

    rig.restart().await;
    let_go(&hold);

    for (id, title) in &cards {
        moved(&rig, id, "review", asked).await;
        assert_eq!(git(&dir, &["log", "--format=%s", "-1", &format!("lane/{id}")]), format!("{id}: {title}"));
    }
}

/// A node whose workers each leave a file of their own, so that several
/// cards merge whichever is merged first, and whose reviewer approves when
/// asked: for cards that are held somewhere other than at an agent. The
/// folder what holds them is in.
async fn rig_of_many() -> (Rig, PathBuf) {
    let mut at = None;
    let rig = Rig::with("api-restarted", |config, dir| {
        let writes = ["swallow", "write", "CHANGES-%:pid%.md", "changed", "say", "I changed a file."];
        let looks = ["swallow", "say", "It does what the card asks.", "say", "DECISION: approve"];
        config.adapters.push(("writer".into(), crate::standin::argv(&writes)));
        config.adapters.push(("critic".into(), crate::standin::argv(&looks)));
        at = Some(dir.to_path_buf());
    })
    .await;
    (Rig::owned(rig).await, at.unwrap())
}

#[tokio::test]
async fn several_cards_each_in_a_command_of_their_projects_when_the_daemon_is_started_again_are_checked_and_none_is_sent_back()
 {
    // A command a row of `segments` names is run for each card before it
    // is looked at. Stopped in the middle, it was only interrupted: it is
    // run again, and what an interrupted one came to is nobody's verdict.
    let (mut rig, at) = rig_of_many().await;
    let hold = Held::pipe(&at, "hold-check");
    let mut began = Held::began(&at, "began-check");
    let dir = shop(&rig, json!({ "workers": 3 })).await;
    // The command is a line for the system's shell, which names a
    // program: it says it was asked and that it began, and waits.
    let file = |name: &str| at.join(name).to_string_lossy().into_owned();
    let (asked, begins, waits) = (file("asked-check"), file("began-check"), file("hold-check"));
    let command =
        crate::standin::line(&["append", &asked, "asked", "tell", &begins, "began", "held", &waits, "check"]);
    let row = json!({
        "title": "Tests", "stage": "review", "order": 5, "does": "std/run", "command": command,
        "record_to": "test-runs",
    });
    crate::fleets::add_row(&rig, "segments", row).await;
    let mut cards = Vec::new();
    for title in ["Add a cart", "Add a till", "Add a shelf"] {
        let id = add(&rig, title, &[]).await;
        move_card(&rig, &id, "ready", None).await;
        cards.push(id);
    }
    // Each was worked, and its command is under way.
    crate::soon("the command of each card to have begun", async {
        while began.heard < cards.len() {
            began.lines.next_line().await.unwrap();
            began.heard += 1;
        }
    })
    .await;
    let stopped = rig.mark();

    rig.restart().await;
    let_go(&hold);

    for id in &cards {
        moved(&rig, id, "done", stopped).await;
    }
    let kept = async |topic: &str| {
        let query = journal::EventQuery { topics: topic.into(), limit: 10_000, ..Default::default() };
        rig.said(query).await
    };
    // What was done before anybody looked at each card came, once, to
    // its review: none was sent back, and none was left for a person.
    let came = kept("stage.came").await;
    for id in &cards {
        let its: Vec<&str> = came
            .iter()
            .filter(|event| event.data["card"] == id.as_str())
            .map(|event| event.data["came"].as_str().unwrap())
            .collect();
        assert_eq!(its, ["review"], "{id}");
        assert_eq!(
            card(&rig, id).await["more"]["rounds"].as_u64().unwrap_or(0),
            0,
            "{id} was never sent back"
        );
    }
    let moves = kept("board.item.moved").await;
    assert_eq!(
        moves.iter().filter(|event| event.data["to"] == "doing").count(),
        cards.len(),
        "each taken once"
    );
    assert!(kept("card.left").await.is_empty());
    // Each was looked at once, and its work merged.
    assert_eq!(given(&rig, "shop-reviewer-1").await.len(), cards.len());
    let merged = std::fs::read_dir(&dir).unwrap().flatten();
    let merged = merged.filter(|file| file.file_name().to_string_lossy().starts_with("CHANGES-")).count();
    assert_eq!(merged, cards.len());
    // The command was begun for each card, and again for each one that
    // was in the middle of it: never more than twice for a card.
    let asked = std::fs::read_to_string(at.join("asked-check")).unwrap().lines().count();
    assert!((cards.len()..=2 * cards.len()).contains(&asked), "the command was begun {asked} times");
    // And each run of it that ended is kept, once: one cut off by the
    // stop ended nowhere, and is in no row.
    let runs = rig.read("/api/collections/shop/test-runs/records").await;
    let runs = runs["records"].as_array().unwrap();
    for id in &cards {
        let its: Vec<&Value> = runs.iter().filter(|run| run["data"]["card"] == id.as_str()).collect();
        assert_eq!(its.len(), 1, "{id}: {runs:?}");
        assert_eq!(its[0]["data"]["result"], "passed", "{id}");
    }
}

#[tokio::test]
async fn a_card_sent_back_while_its_round_is_begun_again_after_a_restart_is_not_handed_on_again_and_its_worker_is_told_why()
 {
    let (mut rig, mut held) = rig().await;
    shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, _) = in_hand(&rig, &mut held).await;

    // The daemon stops as the round hands the card on, before the round
    // has ended: the move to review is the last thing the journal keeps.
    let journal = rig.node().app.journal.clone();
    let its = id.clone();
    journal.shut_after(move |event| {
        event.topic == "board.item.moved" && event.data["id"] == its.as_str() && event.data["to"] == "review"
    });
    held.work();
    journal.shut().await;
    let kept_in = rig.kept_in("shop");
    rig.stop().await;

    // Somebody sends the card back and says why before the round begun
    // again is over. This stands for that order: the daemon stopped
    // after the round's move to review and before the round ended,
    // started, and the card sent back with words while the round begun
    // again is still going. It is written here as the move writes it,
    // into the row, while the daemon is away: what the round goes by is
    // the row, and the row is the same whichever of them came first, so
    // nothing is left to which is quicker when the daemon is back.
    // Written in the file that keeps the card.
    let db = journal::Db::open(&kept_in).unwrap();
    let card = id.clone();
    let rows = db
        .call(move |connection| {
            connection.execute(
                "UPDATE records SET
                   data = json_set(data, '$.status', 'doing', '$.moved',
                     json_array('review', 'user', 'The total is wrong.',
                       CAST(CAST(json_extract(data, '$.moved[3]') AS INTEGER) + 1 AS TEXT))),
                   updated = CAST(strftime('%s', 'now') AS INTEGER) * 1000 + 999
                 WHERE id = ?1",
                [card],
            )
        })
        .await
        .unwrap();
    assert_eq!(rows, 1);
    db.close();
    db.closed().await;
    rig.start().await;

    // The round begun again finds a card somebody has moved since it was
    // told of it, and hands nothing on. The sending back is said, late,
    // and heard as any is: the worker is asked a second time, with the words.
    let its = id.clone();
    let late = rig
        .event("board.item.moved", move |event| {
            event.data["id"] == its.as_str() && event.data["to"] == "doing" && event.data["late"] == true
        })
        .await;
    assert_eq!(late.data["note"], "The total is wrong.");
    let thread = format!("lane:{id}");
    let after = late.seq;
    rig.event("agent.turn.started", move |event| {
        event.seq > after && event.data["thread"] == thread.as_str()
    })
    .await;
    held.work();
    moved(&rig, &id, "review", after).await;
    let told = given(&rig, "shop-worker-1").await;
    assert_eq!(told.len(), 2, "{told:?}");
    assert!(told[1].contains("The total is wrong."), "{}", told[1]);
    // And it was handed on twice in all: once by each round that worked it.
    let all = journal::EventQuery { topics: "board.item.moved".into(), limit: 1000, ..Default::default() };
    let said = rig.said(all).await;
    let handed_on =
        said.iter().filter(|event| event.data["id"] == id.as_str() && event.data["to"] == "review");
    assert_eq!(handed_on.count(), 2);
}

#[tokio::test]
async fn work_begun_again_after_a_restart_is_called_off_where_its_card_was_moved_away_before_its_round_came_to_it()
 {
    let (mut rig, mut held) = rig().await;
    let dir = shop(&rig, json!({ "reviewers": 0 })).await;
    let (id, asked) = in_hand(&rig, &mut held).await;
    let turn = rig.event("agent.turn.started", move |event| event.seq == asked).await.data["turn"].clone();
    let kept_in = rig.kept_in("shop");
    rig.stop().await;

    // Somebody moves the card out of doing before the round asked for
    // again comes to its agent's turn, which the daemon begins again as
    // it starts. This stands for that order: written here as the move
    // writes it, into the row, while the daemon is away, so that the
    // round begun again finds the card moved whichever comes first when
    // the daemon is back, and never asks for the turn.
    let db = journal::Db::open(&kept_in).unwrap();
    let its = id.clone();
    let rows = db
        .call(move |connection| {
            connection.execute(
                "UPDATE records SET
                   data = json_set(data, '$.status', 'draft', '$.moved',
                     json_array('doing', 'user', 'Not yet.',
                       CAST(CAST(json_extract(data, '$.moved[3]') AS INTEGER) + 1 AS TEXT))),
                   updated = CAST(strftime('%s', 'now') AS INTEGER) * 1000 + 999
                 WHERE id = ?1",
                [its],
            )
        })
        .await
        .unwrap();
    assert_eq!(rows, 1);
    db.close();
    db.closed().await;
    let back = rig.mark();
    rig.start().await;

    // The turn begun again is nobody's: it is called off, and its program
    // ended, with nobody having let its agent go on.
    let ended = rig
        .event("agent.turn.finished", move |event| event.seq > back && event.data["slot"] == "shop-worker-1")
        .await;
    assert_eq!(
        (ended.data["turn"].clone(), ended.data["called_off"].clone()),
        (turn, json!(true)),
        "{}",
        ended.data
    );
    assert_eq!(card(&rig, &id).await["status"], "draft");

    // Put back, the card is worked by a round of its own, which is not
    // behind it: its agent is asked once more, and only that round's
    // program writes in the card's place.
    move_card(&rig, &id, "ready", Some("Now.")).await;
    let thread = format!("lane:{id}");
    let after = ended.seq;
    rig.event("agent.turn.started", move |event| {
        event.seq > after && event.data["thread"] == thread.as_str()
    })
    .await;
    held.work();
    moved(&rig, &id, "review", after).await;
    assert_eq!(given(&rig, "shop-worker-1").await.len(), 2);
    let changes = git(&dir, &["show", &format!("lane/{id}:CHANGES.md")]);
    assert_eq!(changes.lines().filter(|line| line.contains("changed")).count(), 1, "{changes}");
}

#[tokio::test]
async fn an_instance_stopped_as_the_daemon_went_is_said_to_be_stopped_and_to_be_over_as_one_thing_and_once() {
    let mut rig = Rig::new().await;
    rig.send("/api/projects", json!({ "name": "Shop" })).await;
    // Something that stays, listening.
    let stays = "blueprint: stays\non:\n  heard:\n    event: shop.nothing\n    do: noop\n";
    let applied = rig.send("/api/apply", json!({ "source": stays, "project": "shop", "wait": true })).await;
    let id = applied["instance"].as_str().unwrap().to_string();

    // It is stopped, and the daemon goes as it is said to be stopped.
    // That it is over is what a pool hears for the place an instance
    // held, and whatever started it for what it had in hand: written as
    // stopped and said to be, and to be over, are one write, so a daemon
    // that went there kept all of it.
    let journal = rig.node().app.journal.clone();
    journal.shut_after(|event| event.topic == "instance.stopped");
    // Whoever asks anything of a journal that has shut may never be answered.
    let stop = format!("/api/instances/{id}/stop");
    tokio::select! {
        _ = rig.send(&stop, json!({})) => {}
        _ = journal.shut() => {}
    }
    journal.shut().await;
    rig.restart().await;

    let its = id.clone();
    let over = rig
        .event("instance.finished", move |event| event.source.instance.as_deref() == Some(its.as_str()))
        .await;
    assert_eq!((over.data["status"].clone(), over.data["key"].is_string()), (json!("stopped"), true));

    // And is said once, however often the daemon is started.
    rig.restart().await;
    let after = over.seq;
    rig.event("system.started", move |event| event.seq > after).await;
    let all = journal::EventQuery { topics: "instance.finished".into(), limit: 1000, ..Default::default() };
    let said = rig.said(all).await;
    assert_eq!(said.iter().filter(|event| event.source.instance.as_deref() == Some(id.as_str())).count(), 1);
}
