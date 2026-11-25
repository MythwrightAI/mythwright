//! What a fleet and a rollout come to when the daemon is stopped and
//! started in the middle of them: each scenario here is played whole, and
//! again with the daemon stopped at each place it could be (`simulation`).
//!
//! What the gate holds is what holds today: a card comes to the same
//! wherever the daemon is stopped while an agent has it, as its turn is
//! begun and once its program is there; a fleet comes up all the same
//! stopped as each kind of thing it is made of is first there and as
//! each row its tables start with is added, over a card that was made
//! ready before it as with one made ready after; and so does a rollout
//! after each of its steps. The rest is found by playing
//! more places, by hand, under the gate's lock:
//!
//! ```sh
//! SIMULATE=kinds cargo test -p api --test suite simulated:: -- --nocapture
//! SIMULATE=everything cargo test -p api --test suite simulated:: -- --nocapture
//! SIMULATE=places cargo test -p api --test suite simulated::a_card_worked -- --nocapture
//! SIMULATE_AT='of std/working (on.tidied)' cargo test -p api --test suite simulated:: -- --nocapture
//! ```
//!
//! `kinds` stops the daemon once inside each kind of run, at every turn
//! an agent is given and wherever something begins to wait; `everything`,
//! after everything it says, which takes minutes. `places` lists every
//! place a scenario passes and plays none, and `SIMULATE_AT` plays only
//! the places whose description has those words in it. A play that does
//! not come to what was meant says where everything the daemon said in it
//! was written down.
//!
//! Played at one stop inside every kind of run and wait (`kinds`), the
//! scenarios all hold, and none says a place is not held yet
//! (`not_yet`). What stopping at every place still finds is written
//! below, before the scenarios.

use std::path::PathBuf;

use kernel::Event;
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets;
use crate::found::{Found, Went};
use crate::lanes::{git, move_card};
use crate::observed::{agents, alive, asked, cards, node_with_stand_ins, repository, rows, said, under_way};
use crate::simulation::{BEGAN, Came, Grain, Holds, Pace, Place, Play, Scenario, simulate};

/// The round a card's lane does, and a card's review, as they ship.
const ROUND: &str = include_str!("../../../../blueprints/std/lane-round.bp");
const REVIEW: &str = include_str!("../../../../blueprints/std/review.bp");

/// What stands in "what the channel was told" for its being told that a
/// card is left for a person, whatever the words of why: those are the
/// application's to choose, and are held where they are chosen.
const LEFT: &str = "(the card is left for a person, and why)";

/// A change to a row of one of the fleet's tables: the table, the row by
/// what it is called, and the cells.
pub(crate) type Row = (&'static str, &'static str, Value);

/// Whether an event is a card having been moved to a column.
pub(crate) fn moved(event: &Event, card: &str, to: &str) -> bool {
    event.topic == "board.item.moved" && event.data["id"] == card && event.data["to"] == to
}

/// How many cards the play has heard tidied away. Nothing is kept for a
/// card, so no instance ends when one is finished: the last thing done for
/// it is its place being tidied, by the stage that worked it.
pub(crate) fn tidied(play: &Play) -> usize {
    play.times(|place| {
        place.topic == "run.finished" && place.within("std/working") && place.flow == "on.tidied"
    })
}

/// A project `shop` that is a repository, with a board and a fleet of so
/// many who work and so many who review, each a stand-in. `own` is what
/// the project has of its own in place of blueprints that ship, by name,
/// and `rows` what its fleet's tables say that they would not otherwise.
pub(crate) async fn shop(workers: u64, reviewers: u64, own: &[(&str, String)], rows: &[Row]) -> Play {
    let (rig, at) = node_with_stand_ins("api-simulated").await;
    let dir = shop_with_a_board(&rig, own).await;
    fleets::fleet(&rig, json!({ "workers": workers, "reviewers": reviewers })).await;
    for (table, row, cells) in rows {
        fleets::change(&rig, table, row, cells.clone()).await;
    }
    let mut play = Play::on(rig, &at).await;
    play.notes.insert("dir", dir.display().to_string());
    play
}

/// A project `shop` that is a repository and has a board, and no fleet:
/// its folder.
pub(crate) async fn shop_with_a_board(rig: &Rig, own: &[(&str, String)]) -> PathBuf {
    let project = rig.send("/api/projects", json!({ "name": "Shop" })).await;
    let dir = PathBuf::from(project["path"].as_str().unwrap());
    for (name, text) in own {
        let path = format!("@blueprints/{}", brand::blueprint_file(name));
        let (status, answer) = rig
            .call(
                reqwest::Method::PUT,
                "/api/projects/shop/file",
                Some(&rig.token),
                Some(json!({ "path": path, "content": text })),
            )
            .await;
        assert_eq!(status, reqwest::StatusCode::OK, "{answer}");
    }
    if !own.is_empty() {
        rig.send("/api/projects/shop/trust", json!({ "trusted": true })).await;
    }
    git(&dir, &["init", "--quiet", "."]);
    git(&dir, &["config", "user.name", "Ada"]);
    git(&dir, &["config", "user.email", "ada@example.com"]);
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "--quiet", "-m", "first"]);
    rig.board("shop").await;
    dir
}

/// The card of this title on the shop's board, if it is there.
pub(crate) async fn card_called(play: &Play, title: &str) -> Option<Value> {
    let board = play.rig.read("/api/board/shop").await;
    board["items"].as_array().unwrap().iter().find(|card| card["title"] == title).cloned()
}

/// Sees to it that there is a card of this title and that it has been
/// made ready, as a person would who was not sure their last click took:
/// its id.
pub(crate) async fn ready(play: &Play, title: &str) -> String {
    let id = match card_called(play, title).await {
        Some(card) => card["id"].as_str().unwrap().to_string(),
        None => fleets::add(&play.rig, title, &[]).await,
    };
    put(play, &id, "draft", "ready", None).await;
    id
}

/// Moves a card from one column to another, if it is still in the first.
async fn put(play: &Play, id: &str, from: &str, to: &str, note: Option<&str>) {
    if fleets::card(&play.rig, id).await["status"] == from {
        move_card(&play.rig, id, to, note).await;
    }
}

/// What a play in `shop` came to, with these agents in it.
pub(crate) async fn shop_came_to(play: &Play, agents: &[&str]) -> Came {
    let rig = &play.rig;
    // That a card is left for a person is held, and not the words of why.
    let told: Vec<Value> = said(rig, "shop").await.as_array().cloned().unwrap_or_default();
    let mut told: Vec<Value> = told
        .into_iter()
        .map(|said| match said.as_str() {
            Some(said) if said.contains("for a person") || said.contains("left as it was") => json!(LEFT),
            _ => said,
        })
        .collect();
    told.sort_by_key(Value::to_string);
    told.dedup();
    // How often each stand-in's program was started: more often after a
    // stop is allowed, as being asked again is.
    let mut asked = asked(rig, agents).await;
    for who in ["writer", "critic"] {
        let started = play.times(|place| place.topic == BEGAN && place.who == who);
        asked.insert(format!("the program of the {who}"), started as u64);
    }
    // Whoever works a card is asked once for each round of it, wherever
    // the daemon was stopped: a round begun again says which round it is,
    // and is given the turn it asked for before, ended or not.
    let mut exactly = Came::default();
    for worker in agents.iter().filter(|agent| agent.contains("worker")) {
        if let Some(times) = asked.remove(*worker) {
            exactly = exactly.with(&format!("times {worker} was asked"), json!(times));
        }
    }
    Came { asked, ..exactly }
        .with("cards", cards(rig, "shop").await)
        .with("repository", repository(&PathBuf::from(play.note("dir"))))
        .with("said in the channel", json!(told))
        .with("alive", alive(rig, Some("shop")).await)
        .with("runs under way", under_way(rig).await)
}

// What stopping at every place still finds, each to be played by name
// with `SIMULATE_AT` (2026-11-15). None is among the places of the `kinds`
// grain.
//
// - Two cards where one is worked at a time, `step.finished` of std/reviewing
//   (on.due), step `card`, the second time: now and then the first card is
//   left in review for a person. Not looked into.
// - Seen once (2026-11-16) and not again in ten plays of the same place:
//   a card worked and passed by a person, stopped after `run.started` of
//   std/pool (on.asked). When the play was judged the card's place was
//   still being tidied (`place.clear` begun and not ended), so one run
//   was under way and the last line of the channel was not yet said. It
//   was the play being judged before a slow `git` had finished on a busy
//   machine: a play is no longer judged while anything is under way.
//
// Three that were written here no longer are. Something said on a card
// cannot be written and not said: it is kept as its saying. What a
// person said as they sent a card back is on the card's row, written
// with the move, and is said with a move said late. And a fleet brought
// up over a card already in the column a stage works left the card there
// for good now and then, with no stop at all (20 plays of 160, eight at
// once on eight processors, 2026-11-17): the stage asked its pool for a
// place before the pool was listening. A pool now says when it is, and
// the stage asks again (`BroughtUpOver`, which is that scenario as it
// was first written).
//
// And one kind of place is no longer there to be listed at all: a card
// whose row was written with its move, or its adding, not yet said. The
// row and what is said of it are one write (`studio::whole`), as a
// message and the saying of it are, so stopped at any place a card is
// where it was last said to be. The places themselves are what they
// were, since a place is something said and the same things are said in
// the same order; nothing was taken off a list for it, there being no
// place on one.

/// A card is worked, and a person passes it.
#[derive(Clone, Default)]
struct PassedByAPerson {
    /// What the project has of its own in place of what ships.
    own: Vec<(&'static str, String)>,
    /// What the fleet's tables say that they would not otherwise.
    rows: Vec<Row>,
    /// With a daemon stopped while its worker's program waits: whether
    /// that program is let go, and ends, while there is no daemon.
    ends_while_away: bool,
}

/// How often a stand-in's program was started, as it is counted where
/// more is allowed and where it is not.
const WRITERS: &str = "the program of the writer";
const STARTED: &str = "times the worker's program was started";

impl PassedByAPerson {
    /// Whether the one who works has a program that goes on without the daemon.
    fn outlives(&self) -> bool {
        self.rows.iter().any(|(_, _, cells)| cells["outlives"] == "on")
    }
}

impl Scenario for PassedByAPerson {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        shop(1, 0, &self.own, &self.rows).await
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        if nth == 0 {
            let id = ready(play, "Add a cart").await;
            play.notes.insert("card", id);
        } else {
            put(play, play.note("card"), "review", "done", None).await;
        }
    }

    async fn away(&self, play: &mut Play) {
        // Only a program that was left waiting can be let go: one the
        // daemon had not started yet is not there.
        if self.ends_while_away && play.owed > 0 {
            let waiting = std::mem::take(&mut play.owed);
            play.let_go(waiting);
            play.ended(1).await;
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        if nth == 0 { moved(event, play.note("card"), "review") } else { tidied(play) == 1 }
    }

    async fn came_to(&self, play: &Play) -> Came {
        let mut came = shop_came_to(play, &["shop-worker-1"]).await;
        // A program that goes on without the daemon is started once,
        // wherever the daemon was stopped: taken up where it was at work,
        // and its answer taken where it had given it. That is held, and
        // not allowed to be more as it is for one that does not.
        if self.outlives() {
            let started = came.asked.remove(WRITERS).unwrap_or(0);
            came = came.with(STARTED, json!(started));
        }
        came
    }
}

/// A card is worked, sent back by a person with what is wrong, worked
/// again, and passed.
#[derive(Clone)]
struct SentBack;

impl Scenario for SentBack {
    const ACTS: usize = 3;

    async fn stage(&self) -> Play {
        shop(1, 0, &[], &[]).await
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => {
                let id = ready(play, "Add a cart").await;
                play.notes.insert("card", id);
            }
            1 => put(play, play.note("card"), "review", "doing", Some("The total is wrong.")).await,
            _ => put(play, play.note("card"), "review", "done", None).await,
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        if nth < 2 { moved(event, play.note("card"), "review") } else { tidied(play) == 1 }
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"]).await
    }
}

/// A card is worked, an agent reviews and approves it, and it is merged.
#[derive(Clone, Default)]
struct ReviewedAndMerged {
    own: Vec<(&'static str, String)>,
    /// The stage that reviews says a review in hand at a restart is left
    /// for a person, where it would otherwise go on.
    left: bool,
}

impl ReviewedAndMerged {
    /// Whether a place is one where whoever reviews has the card in hand:
    /// their turn has begun, or their program is there. A review stopped
    /// before that has nobody in the middle of anything, and goes on
    /// whatever its stage says.
    fn in_review(place: &Place) -> bool {
        (place.topic == "agent.turn.started" && place.who == "shop-reviewer-1")
            || (place.topic == BEGAN && place.who == "critic")
    }
}

impl Scenario for ReviewedAndMerged {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let leave = ("stages", "review", json!({ "after_restart": "leave" }));
        shop(1, 1, &self.own, if self.left { std::slice::from_ref(&leave) } else { &[] }).await
    }

    async fn act(&self, _: usize, play: &mut Play) {
        let id = ready(play, "Add a cart").await;
        play.notes.insert("card", id);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        tidied(play) == 1
    }

    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await
    }

    fn either(&self, place: &Place) -> Option<&'static str> {
        crate::either::a_review_over(place)
    }

    /// A review in hand when the daemon is stopped goes on when it is
    /// back, unless its stage says to leave it. Left, it is not begun
    /// again and nothing is passed or sent back on a guess: the channel is
    /// told that nobody decided, and the card stays in review, with its
    /// work on its own branch, for a person.
    fn meant(&self, place: &Place, usual: &Came) -> Came {
        if !self.left || !Self::in_review(place) {
            return usual.clone();
        }
        let mut said: Vec<Value> = usual.same["said in the channel"].as_array().cloned().unwrap();
        said.push(json!(LEFT));
        said.sort_by_key(Value::to_string);
        // Nothing is alive for a card that waits for a person: it is a row.
        let alive = usual.same["alive"].clone();
        let mut meant = usual
            .clone()
            .with("cards", json!({ "shop-1": { "in": "review", "sent back": 0 } }))
            .with(
                "repository",
                json!({
                    "branches": { "lane/shop-1": ["shop-1: Add a cart", "first"], "main": ["first"] },
                    "places": ["shop-1"],
                    "uncommitted": [],
                }),
            )
            .with("said in the channel", json!(said))
            .with("alive", alive);
        // Whoever was looking is not asked again, and where the daemon went
        // before their program was started, it never is.
        meant.asked.insert("shop-reviewer-1".to_string(), 0);
        meant.asked.insert("the program of the critic".to_string(), 0);
        meant
    }
}

/// Two cards are made ready where one can be worked at a time: one waits
/// for its place, and both are worked, reviewed and merged.
#[derive(Clone)]
struct WaitingItsTurn;

impl Scenario for WaitingItsTurn {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        shop(1, 1, &[], &[]).await
    }

    /// Both are made ready, and the second has asked for its place before
    /// whoever works the first is let go: a stand-in is let go once the
    /// play listens, which is when this is done. Left to the machine, the
    /// first was now and then worked and its place given back before the
    /// second had asked, and nothing waited its turn in a scenario of
    /// something waiting its turn: half its places were said in one play
    /// and not in the next.
    async fn act(&self, _: usize, play: &mut Play) {
        let first = ready(play, "Lay the floor").await;
        let second = ready(play, "Put the shelves up").await;
        play.said("pool.asked", |event| event.data["as"] == second.as_str()).await;
        play.notes.insert("first", first);
        play.notes.insert("second", second);
    }

    fn heard(&self, _: usize, play: &Play, _: &Event) -> bool {
        tidied(play) == 2
    }

    /// Which of the two is merged first is not held: both rounds begin
    /// again after a restart, and the one that had waited may be given the
    /// place first. So what is on a branch is held, and not in what order.
    async fn came_to(&self, play: &Play) -> Came {
        let mut came = shop_came_to(play, &["shop-worker-1", "shop-reviewer-1"]).await;
        if let Some(branches) =
            came.same.get_mut("repository").and_then(|all| all["branches"].as_object_mut())
        {
            for said in branches.values_mut().filter_map(Value::as_array_mut) {
                said.sort_by_key(Value::to_string);
            }
        }
        came
    }

    /// And each of the two reviews ends as one alone does
    /// (`a_review_over`): what hears its ending has a run or none, by
    /// which of its last two things the stage takes in first.
    fn either(&self, place: &Place) -> Option<&'static str> {
        crate::either::a_second_review(place).or_else(|| crate::either::a_review_over(place))
    }
}

/// A fleet is brought up for a project that has a board and nobody to
/// work it, and when the fleet says it is up a card is made ready, worked,
/// and passed by a person. What a fleet does as it comes up is played, so
/// every place of it is one the daemon is stopped at on purpose; and the
/// card says whether what came up after a stop is a fleet that works.
#[derive(Clone)]
struct BroughtUp;

impl Scenario for BroughtUp {
    const ACTS: usize = 3;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-simulated").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            // As somebody does who is not sure their asking took: only if
            // no fleet is there. One that is there and still coming up is
            // left to come up.
            0 if alive(&play.rig, Some("shop")).await.get("std/fleet").is_none() => {
                let params = fleets::params(json!({ "workers": 1, "reviewers": 0 }));
                let fleet =
                    json!({ "blueprint": "std/fleet", "project": "shop", "params": params, "wait": false });
                play.rig.send("/api/apply", fleet).await;
            }
            0 => {}
            1 => {
                let id = ready(play, "Add a cart").await;
                play.notes.insert("card", id);
            }
            _ => put(play, play.note("card"), "review", "done", None).await,
        }
    }

    /// A fleet is up when its steps have run to their end and gone well,
    /// which is what whoever applied it is answered. One whose steps
    /// failed says that they finished too, and is not up.
    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        match nth {
            0 => {
                event.topic == "run.finished"
                    && event.data["blueprint"] == "std/fleet"
                    && event.data["flow"] == "main"
                    && event.data["status"] == "ok"
            }
            1 => moved(event, play.note("card"), "review"),
            _ => tidied(play) == 1,
        }
    }

    /// What a card worked and passed comes to, and the fleet itself: the
    /// rows its two tables were given once, and who was brought up.
    async fn came_to(&self, play: &Play) -> Came {
        shop_came_to(play, &["shop-worker-1"])
            .await
            .with("who is in the fleet", rows(&play.rig, "shop", "agents").await)
            .with("what is done with a card", rows(&play.rig, "shop", "stages").await)
            .with("agents", agents(&play.rig).await)
    }

    fn either(&self, place: &Place) -> Option<&'static str> {
        crate::either::coming_up(place)
    }
}

/// A fleet is brought up for a project whose board has a card that was
/// made ready before there was anybody to work it: the fleet that comes
/// up takes the card it finds in its column and works it, and a person
/// passes it. A stage and its pool are started side by side, so the card
/// may be found, and its place asked for, before the pool is listening.
#[derive(Clone)]
struct BroughtUpOver;

impl Scenario for BroughtUpOver {
    const ACTS: usize = 2;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-simulated").await;
        let dir = shop_with_a_board(&rig, &[]).await;
        let id = fleets::add(&rig, "Add a cart", &[]).await;
        move_card(&rig, &id, "ready", None).await;
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("dir", dir.display().to_string());
        play.notes.insert("card", id);
        play
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        match nth {
            0 => BroughtUp.act(0, play).await,
            _ => put(play, play.note("card"), "review", "done", None).await,
        }
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        if nth == 0 { moved(event, play.note("card"), "review") } else { tidied(play) == 1 }
    }

    async fn came_to(&self, play: &Play) -> Came {
        BroughtUp.came_to(play).await
    }

    fn either(&self, place: &Place) -> Option<&'static str> {
        crate::either::coming_up_over_a_card(place)
    }
}

/// Something that answers every connection to its port with its name: the port.
async fn version(name: &'static str) -> u64 {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let _ = stream.write_all(name.as_bytes()).await;
        }
    });
    u64::from(port)
}

/// Who answers at a port just now; nobody, where nothing does in a moment.
async fn answers(port: u64) -> String {
    use tokio::io::AsyncReadExt;
    let asking = async {
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port as u16)).await.ok()?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer).await.ok()?;
        Some(answer)
    };
    match tokio::time::timeout(std::time::Duration::from_millis(300), asking).await {
        Ok(Some(answer)) if !answer.is_empty() => answer,
        _ => "nobody".to_string(),
    }
}

/// What a slot called `web` is told, for a version that answers at `at`.
/// The program it hosts does nothing; what answers is the play's own.
fn slot(at: u64) -> Value {
    json!({
        "name": "web",
        "service": { "cmd": [crate::STANDIN, "idle"], "port": at, "restart": "never" },
        "weights": [50, 100],
        "observe": "50ms",
    })
}

/// A service is hosted behind a slot, and a second version of it is
/// rolled out: given half the connections, then all, and promoted.
#[derive(Clone)]
struct RolledOut;

impl Scenario for RolledOut {
    const ACTS: usize = 1;

    async fn stage(&self) -> Play {
        let (rig, at) = node_with_stand_ins("api-simulated").await;
        let (one, two) = (version("one").await, version("two").await);
        let first = json!({ "blueprint": "std/slot", "params": slot(one), "wait": true });
        let applied = rig.send("/api/apply", first).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
        let mut play = Play::on(rig, &at).await;
        play.notes.insert("slot", applied["instance"].as_str().unwrap().to_string());
        play.notes.insert("at", applied["finished"]["outputs"]["port"].to_string());
        play.notes.insert("two", two.to_string());
        play
    }

    async fn act(&self, _: usize, play: &mut Play) {
        // As somebody does who is not sure their asking took: only if the
        // second version is not all there is already.
        if answers(play.note("at").parse().unwrap()).await != "two" {
            let two: u64 = play.note("two").parse().unwrap();
            play.rig
                .send("/api/apply", json!({ "blueprint": "std/slot", "params": slot(two), "wait": false }))
                .await;
        }
    }

    fn heard(&self, _: usize, _: &Play, event: &Event) -> bool {
        event.topic == "slot.promoted"
    }

    async fn came_to(&self, play: &Play) -> Came {
        let at: u64 = play.note("at").parse().unwrap();
        // Asked several times, since a slot may be sharing its connections
        // out; once, where nobody answers at all.
        let mut who = std::collections::BTreeSet::from([answers(at).await]);
        if !who.contains("nobody") {
            for _ in 0..7 {
                who.insert(answers(at).await);
            }
        }
        let fleet = play.rig.read("/api/fleet").await;
        Came::default()
            .with("who answers at the slot's address", json!(who))
            .with("how many services there are", json!(fleet["services"].as_array().map_or(0, Vec::len)))
            .with("alive", alive(&play.rig, None).await)
            .with("runs under way", under_way(&play.rig).await)
    }

    async fn strike(&self, play: &mut Play) {
        // A daemon leaves what it hosts running: the slot is stopped, and
        // what it hosted with it.
        let stop = format!("/api/instances/{}/stop", play.note("slot"));
        let _ = play.rig.post(&stop, json!({})).await;
    }
}

/// Where an agent has work in hand: its turn has just been said to begin,
/// and its program is there. Stopped at the first the daemon had not
/// started the program; at the second it goes while the program waits.
pub(crate) fn in_hand(place: &Place) -> bool {
    place.topic == "agent.turn.started" || place.topic == BEGAN
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_its_worker_has_in_hand_when_the_daemon_is_stopped_is_worked_and_passed_all_the_same() {
    let grain = Grain::asked_or(Grain::Where(in_hand));
    simulate("a card worked and passed by a person", PassedByAPerson::default(), grain).await.holds();
}

/// Where the stage that worked a card is saying, as it tidies, that the
/// card's branch was kept: as it begins to say it, and when it has.
fn saying_what_was_kept(place: &Place) -> bool {
    (place.within("std/working") && place.flow == "on.tidied" && place.step == "said")
        || (place.topic == "channel.message" && place.nth == 2)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn what_a_stage_was_saying_when_the_daemon_stopped_is_said_once_when_it_is_back_and_nothing_fails() {
    let found = simulate(
        "a card worked and passed by a person",
        PassedByAPerson::default(),
        Grain::Where(saying_what_was_kept),
    )
    .await;
    found.holds();
    assert!(
        found.tried.len() >= 3,
        "before it is said, after, and when the step has ended: {}",
        found.tried.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_sent_back_is_worked_again_all_the_same_whichever_of_its_rounds_the_daemon_is_stopped_in() {
    let grain = Grain::asked_or(Grain::Where(in_hand));
    simulate("a card sent back and worked again", SentBack, grain).await.holds();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_in_its_work_or_in_its_review_is_reviewed_and_merged_all_the_same() {
    let grain = Grain::asked_or(Grain::Where(in_hand));
    simulate("a card reviewed by an agent and merged", ReviewedAndMerged::default(), grain).await.holds();
}

/// Where an agent has just been given work to wait its turn, and whoever
/// asked has not yet kept which turn it waits for.
fn just_asked(place: &Place) -> bool {
    place.topic == "agent.turn.queued"
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_as_its_worker_or_its_reviewer_is_asked_is_reviewed_and_merged_all_the_same() {
    let found = simulate(
        "a card reviewed by an agent and merged",
        ReviewedAndMerged::default(),
        Grain::Where(just_asked),
    )
    .await;
    found.holds();
    assert!(
        found.tried.len() >= 2,
        "once for whoever works and once for whoever reviews: {}",
        found.tried.len()
    );
}

/// Where a review is cut off with the card in its reviewer's hands, or
/// just after the reviewer answered and before the review took the answer:
/// that answer is there, and is taken whatever the stage says of work in
/// hand.
fn in_hand_or_answered(place: &Place) -> bool {
    in_hand(place) || (place.topic == "agent.turn.finished" && place.who == "shop-reviewer-1")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_card_stopped_in_a_review_its_stage_says_to_leave_is_left_in_review_for_a_person() {
    let scenario = ReviewedAndMerged { left: true, ..Default::default() };
    let grain = Grain::asked_or(Grain::Where(in_hand_or_answered));
    let found = simulate("a card whose review is left at a restart", scenario, grain).await;
    found.holds();
    // Where it was stopped while somebody had the card to review, it was
    // held to what its stage says of that, and not passed over.
    let reviewing: Vec<_> =
        found.tried.iter().filter(|tried| ReviewedAndMerged::in_review(&tried.place)).collect();
    assert!(
        reviewing.len() >= 2 && reviewing.iter().all(|tried| matches!(tried.went, Went::AsMeant)),
        "{:?}",
        reviewing.iter().map(|tried| (tried.place.to_string(), &tried.went)).collect::<Vec<_>>()
    );
}

/// A worker whose program goes on without the daemon.
fn outliving(ends_while_away: bool) -> PassedByAPerson {
    let rows = vec![("agents", "worker", json!({ "outlives": "on" }))];
    PassedByAPerson { rows, ends_while_away, ..Default::default() }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_program_that_outlives_the_daemon_and_is_still_at_work_when_it_is_back_is_taken_up_and_asked_once()
{
    let grain = Grain::asked_or(Grain::Where(in_hand));
    let found = simulate("a card whose worker goes on without the daemon", outliving(false), grain).await;
    found.holds();
    stopped_at_work(&found);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_program_that_outlives_the_daemon_and_ends_while_it_is_away_has_its_answer_taken_and_is_asked_once()
{
    let grain = Grain::asked_or(Grain::Where(in_hand));
    let found =
        simulate("a card whose worker finishes while the daemon is away", outliving(true), grain).await;
    found.holds();
    stopped_at_work(&found);
}

/// Holds that the daemon was stopped while the worker's program was
/// there, and that the play came to what was meant: which says, of a
/// program that outlives the daemon, that it was started once.
fn stopped_at_work(found: &Found) {
    let there = |place: &Place| place.topic == BEGAN && place.who == "writer";
    assert!(
        found.tried.iter().any(|tried| there(&tried.place) && matches!(tried.went, Went::AsMeant)),
        "the daemon was not stopped while the program was there"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_service_rolled_out_through_a_slot_comes_to_the_same_whichever_step_the_daemon_is_stopped_after() {
    // A rollout is its steps, one after another: after each of them
    // begins, the first time it does.
    let grain = Grain::asked_or(Grain::Where(|place| place.topic == "step.started" && place.nth == 1));
    simulate("a service rolled out through a slot", RolledOut, grain).await.holds();
}

/// Where something a fleet is made of is first there: the fleet itself,
/// what brings it to what its tables say, a role, a pool, a stage. One
/// stop as each kind of thing comes up, written down and not yet begun.
/// A card's round, and the things done in one, are not of the fleet
/// coming up, and are stopped in by the scenarios of a card; the small
/// things a stage calls as it comes up (what is wrong with its row, what
/// was left in hand) are stopped in with every place.
fn first_there(place: &Place) -> bool {
    let of_a_round = [
        "std/lane-round",
        "std/work",
        "std/turn",
        "std/keep",
        "std/send",
        "std/cleared",
        "std/looked",
        "std/resumed",
    ]
    .iter()
    .any(|one| place.within(one));
    place.topic == "instance.started" && place.nth == 1 && !of_a_round
}

/// Where a row one of a fleet's tables starts with has just been added,
/// and the step that adds them has not yet kept that it was: the rows
/// are added by one step for each table, which finds again by what it is
/// called a row it had added, and goes on from there.
fn a_first_row_added(place: &Place) -> bool {
    place.topic == "record.created" && place.within("std/fleet")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fleet_stopped_as_it_comes_up_comes_up_all_the_same_and_works_a_card_made_ready_then() {
    let coming_up: fn(&Place) -> bool = |place| first_there(place) || a_first_row_added(place);
    let grain = Grain::asked_or(Grain::Where(coming_up));
    let found = simulate("a fleet brought up, and a card worked by it", BroughtUp, grain).await;
    found.holds();
    // Stopped as each row of each table was added, where that was what
    // was played: two who are in the fleet, and two things done with a card.
    let rows = found.tried.iter().filter(|tried| a_first_row_added(&tried.place)).count();
    let by_hand = ["SIMULATE", "SIMULATE_AT"].iter().any(|said| std::env::var(said).is_ok());
    assert!(by_hand || rows == 4, "{rows} of the places were a row being added");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_fleet_brought_up_over_a_card_made_ready_works_the_card_that_was_waiting() {
    let grain = Grain::asked_or(Grain::Where(first_there));
    simulate("a fleet brought up over a card made ready", BroughtUpOver, grain).await.holds();
}

/// Of a scenario played with the daemon stopped where `such` says, how
/// the one such place went.
async fn at<S: Scenario>(name: &'static str, scenario: S, such: fn(&Place) -> bool) -> (Place, Went) {
    let found = simulate(name, scenario, Grain::Where(such)).await;
    assert_eq!(found.tried.len(), 1, "one place was to be tried");
    let tried = found.tried.into_iter().next().unwrap();
    (tried.place, tried.went)
}

/// The first time the program of the one who works is there.
fn the_worker_at_work(place: &Place) -> bool {
    place.topic == BEGAN && place.who == "writer" && place.nth == 1
}

fn the_reviewer_at_work(place: &Place) -> bool {
    place.topic == BEGAN && place.who == "critic" && place.nth == 1
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_harness_finds_a_round_that_leaves_its_work_at_a_restart_whatever_its_stage_says() {
    // A round of the project's own that tells its agent's turn to be left
    // at a restart, where the stage's row says it goes on: the card stays
    // in doing. (A round that is kept step by step, which left its card in
    // doing on the morning of 2026-11-15, no longer does: a turn in hand
    // is now taken up again whoever asks for it a second time.)
    let asks = "after_restart: \"${params.after_restart}\"";
    let leaves = ROUND.replacen(asks, "after_restart: leave", 1);
    assert_ne!(leaves, ROUND, "the round that ships no longer hands on `after_restart` in those words");
    let scenario = PassedByAPerson { own: vec![("std/lane-round", leaves)], ..Default::default() };

    let (place, went) = at("a card whose round leaves its work", scenario, the_worker_at_work).await;

    let Went::Fault(why) = went else { panic!("stopped after {place}, nothing was found wrong: {went:?}") };
    let why = why.join("\n");
    assert!(why.contains(r#"cards: it came to {"shop-1":{"in":"doing","sent back":0}}"#), "{why}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_harness_finds_a_review_that_keeps_nothing_leaving_its_card_in_review_with_nothing_said() {
    // As was tried on 2026-11-15 and not kept: a review that keeps nothing
    // as it goes is simply gone at a restart.
    let described = REVIEW.find("\ndescription:").expect("a review has a description") + 1;
    let after = described + REVIEW[described..].find('\n').unwrap() + 1;
    let forgetful = format!("{}durable: false\n{}", &REVIEW[..after], &REVIEW[after..]);
    let scenario = ReviewedAndMerged { own: vec![("std/review", forgetful)], left: false };

    let (place, went) = at("a card whose review keeps nothing", scenario, the_reviewer_at_work).await;

    let Went::Fault(why) = went else { panic!("stopped after {place}, nothing was found wrong: {went:?}") };
    // The card is still in review, where it was meant to be merged, and
    // nobody was told anything.
    let why = why.join("\n");
    assert!(why.contains(r#"cards: it came to {"shop-1":{"in":"review","sent back":0}}"#), "{why}");
    assert!(!why.contains("said in the channel"), "{why}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_cards_where_one_is_worked_at_a_time_are_both_worked_reviewed_and_merged_wherever_the_daemon_is_stopped()
 {
    // Stopped the first time each stand-in's program is there: while the
    // worker has the first card and the second waits its turn, and while
    // the first is being reviewed. Until 2026-11-15 the first of those
    // left a card in doing or in ready for ever, twice over: a lane that
    // was picked up first waited on the round it had called
    // (`Engine::end_runs_of`), and a pool gave a place to a round that
    // was gone.
    let grain = Grain::asked_or(Grain::Where(|place| place.topic == BEGAN && place.nth == 1));
    simulate("a card waiting for its turn", WaitingItsTurn, grain).await.holds();
}

/// A card worked and passed by a person, with the program of the one who
/// works held once the daemon is back: what holds the harness to its own
/// rule for when a play is judged. It is waited for a short while, so
/// that the tests of the rule take a second and not a minute.
#[derive(Clone)]
struct Held(PassedByAPerson, Holds);

impl Scenario for Held {
    const ACTS: usize = PassedByAPerson::ACTS;

    async fn stage(&self) -> Play {
        self.0.stage().await
    }

    async fn act(&self, nth: usize, play: &mut Play) {
        self.0.act(nth, play).await
    }

    fn heard(&self, nth: usize, play: &Play, event: &Event) -> bool {
        self.0.heard(nth, play, event)
    }

    async fn came_to(&self, play: &Play) -> Came {
        self.0.came_to(play).await
    }

    fn pace(&self) -> Pace {
        let ms = std::time::Duration::from_millis;
        Pace { stuck: ms(300), patience: ms(1200), holds: self.1 }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_play_with_something_still_at_work_and_nothing_said_is_waited_for_and_not_called_a_fault() {
    // Once the daemon is back the worker's program says nothing for longer
    // than a play may be silent somewhere else before it is looked at: as
    // a slow commit does. It is let go only once the play has been waited
    // for with it at work, so the play comes to what was meant only if it
    // was waited for; judged by its silence, it was a fault.
    let scenario = Held(PassedByAPerson::default(), Holds::UntilWaitedFor);
    let (place, went) = at("a card whose worker is slow", scenario, the_worker_at_work).await;
    assert!(matches!(went, Went::AsMeant), "stopped after {place}: {went:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_play_that_never_ends_is_said_not_to_have_come_to_rest_which_is_not_a_fault() {
    // The worker's program never ends: the round waits on it for good.
    let scenario = Held(PassedByAPerson::default(), Holds::ForEver);
    let (place, went) = at("a card whose worker never ends", scenario, the_worker_at_work).await;
    let Went::Unrested(why) = went else { panic!("stopped after {place}, it was not said: {went:?}") };
    // Said with what was under way: the round, and its agent's turn.
    assert!(why[0].starts_with("still under way: "), "{why:?}");
    assert!(
        why[0].contains("a run of std/lane-round (main)") && why[0].contains("a turn of shop-worker-1"),
        "{why:?}"
    );
    // And with where the card was, which is what was not as meant.
    assert!(why.iter().any(|line| line.starts_with("cards: it came to")), "{why:?}");
}
