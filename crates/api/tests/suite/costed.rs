//! What one card costs the factory, held to a budget as a build's time
//! is (`[card]` of `build-budget.toml`; *What a card is held to* in
//! `docs/build-times.md`). Cards are worked through the fleet that ships
//! by stand-ins, reviewed and merged, one at a time, each in a factory at
//! rest before and after: one with nothing laid into its stage's lists,
//! one with a command laid into its round by a row, and one with a
//! second review of two seats laid in after the usual one. For each the
//! journal's own counts are read: what was said, what was made and run,
//! the pieces of work every journal's database did and how many of them
//! wrote (what `db.work` counts), the commits, and the frames written to
//! the project's log, with what the factory does whichever card's moment
//! it falls in counted apart (`Cost::own`), so that the pieces of work,
//! the writes and the commits are the same in every run. No time is
//! held: those are the machine's. And the
//! plain card is worked beside three cards and beside 253, with every
//! database counting its own steps, and held to costing the same: what
//! reads its board whole for one card is a cost that grows with the board.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use journal::FollowEvery;
use serde_json::{Value, json};

use crate::Rig;
use crate::fleets::{add_row, change, remove_row, rig_told, shop};
use crate::lanes::move_card;

/// How long the factory waits for an event before it is looked at again.
const LULL: Duration = Duration::from_millis(40);
/// How long nothing has to be said, and nothing asked of any database,
/// with nothing under way, for the factory to be at rest.
const REST: Duration = Duration::from_millis(400);
/// How long a card may take to be at rest at all.
const PATIENCE: Duration = Duration::from_secs(60);
/// How big a frame of a log is: a page and its head.
const FRAME_BYTES: u64 = 2048 + 24;

/// What the databases have counted so far.
#[derive(Clone, Copy, Default)]
struct Counts {
    done: u64,
    writes: u64,
    commits: u64,
    frames: u64,
    /// The databases' own steps, where they are counted (`stepping`).
    steps: u64,
}

fn counts(rig: &Rig) -> Counts {
    let journal = &rig.node().app.journal;
    let mut all = Counts::default();
    for db in journal.dbs() {
        all.done += db.done();
        all.writes += db.writes();
        all.commits += db.commits();
    }
    all.frames = journal.of(Some("shop")).db().logged();
    all
}

/// The factory is followed once, from before the first card, so that
/// following it asks no database anything while a card is measured. A
/// run and an agent's turn say when they begin and end, so what is under
/// way is known from what was said, and no database is asked that either.
struct Watching {
    follow: FollowEvery,
    runs: BTreeSet<String>,
    turns: BTreeSet<String>,
    /// The databases' own steps, counted only once `stepping` is asked.
    steps: Arc<AtomicU64>,
}

impl Watching {
    fn new(rig: &Rig) -> Watching {
        let journal = &rig.node().app.journal;
        Watching {
            follow: journal.follow_every(journal.now(), ""),
            runs: BTreeSet::new(),
            turns: BTreeSet::new(),
            steps: Arc::default(),
        }
    }

    /// What the databases have counted so far, their own steps with it.
    fn counts(&self, rig: &Rig) -> Counts {
        Counts { steps: self.steps.load(Ordering::Relaxed), ..counts(rig) }
    }

    fn heard(&mut self, event: &kernel::Event) {
        let run = event.source.run.clone().unwrap_or_default();
        let turn = event.data["turn"].as_str().unwrap_or_default().to_string();
        match event.topic.as_str() {
            "run.started" | "run.resumed" => self.runs.insert(run),
            "run.finished" => self.runs.remove(&run),
            "agent.turn.queued" | "agent.turn.started" => self.turns.insert(turn),
            "agent.turn.finished" => self.turns.remove(&turn),
            _ => false,
        };
    }

    /// Waits for the factory to be at rest, and gives what the databases
    /// had counted then: before anything is asked to make sure of it.
    async fn rested(&mut self, rig: &Rig) -> Counts {
        let began = Instant::now();
        loop {
            let (mut quiet, mut done) = (Instant::now(), counts(rig).done);
            loop {
                assert!(
                    began.elapsed() < PATIENCE,
                    "the factory did not come to rest: {}",
                    rig.lately().await
                );
                if let Ok(event) = tokio::time::timeout(LULL, self.follow.next()).await {
                    self.heard(&event.expect("the journal is followed"));
                    quiet = Instant::now();
                    continue;
                }
                let now = counts(rig).done;
                if now != done {
                    (done, quiet) = (now, Instant::now());
                }
                if quiet.elapsed() >= REST && self.runs.is_empty() && self.turns.is_empty() {
                    break;
                }
            }
            let seen = self.counts(rig);
            // What was said says nothing of a run that has its place and
            // waits its turn to begin: the rows are asked, once quiet.
            let (runs, turns, _) = crate::observed::at_work(rig).await;
            if runs == 0 && turns == 0 {
                return seen;
            }
        }
    }
}

/// What one card cost, by what the journal counts.
#[derive(Clone, Debug, PartialEq)]
struct Cost {
    events: u64,
    instances: u64,
    runs: u64,
    done: u64,
    writes: u64,
    commits: u64,
    frames: u64,
    /// The databases' own steps, which no budget holds: they are counted
    /// only where a test asks (`stepping`).
    steps: u64,
    /// Of the writes, those that kept a handler's place for no event of
    /// its own (`conductor::PLACES_KEPT`): each is one piece of work and
    /// one commit of its own, and which card's moment it falls in goes by
    /// everything said before the card, and by how a handler's reading
    /// fell.
    kept: u64,
    /// Of the pieces of work done, those that read an instance's row for
    /// want of it in hand (`runs::LOOKED_UP`): whether one is read goes
    /// by whether the instance had been let go of in that moment.
    looked: u64,
    /// Of the writes, those of a step's record that would have been held
    /// and written with the next thing its run writes, but for another
    /// step of its run at work in that moment (`conductor::WRITTEN_BESIDE`).
    beside: u64,
    /// Of the pieces of work done, those that only read for want of
    /// something in hand or heard: a page a follower read back from the
    /// log (`journal::READ_BACK`), and the files more than one a turn was
    /// asked of by its id once it had ended (`agents::TURN_LOOKED_UP`),
    /// and a turn read by whoever waits for it, as the wait begins and as
    /// it is woken (`agents::TURN_WAITED`).
    read_back: u64,
}

impl Cost {
    fn figures(&self) -> [(&'static str, u64); 7] {
        [
            ("events", self.events),
            ("instances", self.instances),
            ("runs", self.runs),
            ("done", self.done),
            ("writes", self.writes),
            ("commits", self.commits),
            ("frames", self.frames),
        ]
    }

    /// What the card asked for itself: its figures without what the
    /// factory does whichever card's moment it falls in, or does or not
    /// by which of two things side by side came first, which is counted
    /// apart (`kept`, `looked`, `beside`, `read_back`).
    fn own(&self) -> Cost {
        Cost {
            done: self.done - self.kept - self.looked - self.beside - self.read_back,
            writes: self.writes - self.kept - self.beside,
            commits: self.commits.saturating_sub(self.kept + self.beside),
            ..self.clone()
        }
    }
}

/// How many times what is measured as `name` has been measured so far,
/// and what it came to in all.
async fn measured_all(rig: &Rig, name: &str) -> (u64, f64) {
    let journal = &rig.node().app.journal;
    journal.flush_metrics().await.unwrap();
    let query = journal::MetricQuery { name: name.into(), ..Default::default() };
    let series = journal.query_metrics(query).await.unwrap();
    (series.iter().map(|series| series.total.count).sum(), series.iter().map(|series| series.total.sum).sum())
}

/// How many times what is measured as `name` has been measured so far.
async fn measured(rig: &Rig, name: &str) -> u64 {
    measured_all(rig, name).await.0
}

/// How many writes have kept a handler's place for no event of its own,
/// and how many pieces of work have read an instance's row for want of
/// it in hand, so far.
async fn apart(rig: &Rig) -> (u64, u64, u64, u64) {
    let (kept, looked) = (measured(rig, conductor::PLACES_KEPT).await, measured(rig, runs::LOOKED_UP).await);
    // A run of numbers drawn ahead by a project's journal from the
    // factory's giver falls in whichever card's moment it falls in, by how
    // many events were said before it (`journal::giver`): a write of the
    // factory's own, as a handler's place kept is.
    let drawn: u64 =
        writes_by_whom(rig).await.iter().filter(|(who, _)| who.ends_with(" by giver")).map(|(_, n)| n).sum();
    let beside = measured(rig, conductor::WRITTEN_BESIDE).await + drawn;
    let read_back = measured(rig, journal::READ_BACK).await
        + measured_all(rig, agents::TURN_LOOKED_UP).await.1 as u64
        + measured(rig, agents::TURN_WAITED).await;
    (kept, looked, beside, read_back)
}

/// The pieces of work that wrote so far, by database and by whose work
/// they were (`db.work`), for saying where to look.
async fn writes_by_whom(rig: &Rig) -> BTreeMap<String, u64> {
    let journal = &rig.node().app.journal;
    journal.flush_metrics().await.unwrap();
    let query = journal::MetricQuery {
        name: "db.work".into(),
        labels: vec![("kind".into(), "write".into())],
        group_by: vec!["db".into(), "caller".into()],
        ..Default::default()
    };
    let mut by = BTreeMap::new();
    for series in journal.query_metrics(query).await.unwrap() {
        let label = |key: &str| series.labels.get(key).cloned().unwrap_or_default();
        by.insert(format!("{} by {}", label("db"), label("caller")), series.total.count);
    }
    by
}

/// Adds a card, makes it ready and waits for it to be worked, reviewed,
/// merged and done, and for the factory to be at rest again.
async fn card(rig: &Rig, watching: &mut Watching, title: &str) -> (Cost, BTreeMap<String, u64>, Vec<Ran>) {
    let whom = writes_by_whom(rig).await;
    let (kept, looked, beside, read_back) = apart(rig).await;
    let (mark, before) = (rig.mark(), watching.counts(rig));
    let id = rig.send("/api/board/shop/items", json!({ "title": title })).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    move_card(rig, &id, "ready", None).await;
    let after = watching.rested(rig).await;
    let until = rig.mark();

    let board = rig.read("/api/board/shop").await;
    let there = board["items"].as_array().unwrap().iter().find(|card| card["id"] == id.as_str()).cloned();
    assert_eq!(there.map(|card| card["status"].clone()), Some(json!("done")), "{title} was not done");
    let said = rig.said(journal::EventQuery {
        limit: 100_000,
        after: mark,
        before: until + 1,
        ..Default::default()
    });
    let said = said.await;
    let of = |topic: &str| said.iter().filter(|event| event.topic == topic).count() as u64;
    let cost = Cost {
        events: said.len() as u64,
        instances: of("instance.started"),
        runs: of("run.started"),
        done: after.done - before.done,
        writes: after.writes - before.writes,
        commits: after.commits - before.commits,
        frames: after.frames - before.frames,
        steps: after.steps - before.steps,
        kept: 0,
        looked: 0,
        beside: 0,
        read_back: 0,
    };
    let (kept_now, looked_now, beside_now, read_back_now) = apart(rig).await;
    let cost = Cost {
        kept: kept_now - kept,
        looked: looked_now - looked,
        beside: beside_now - beside,
        read_back: read_back_now - read_back,
        ..cost
    };
    let since = writes_by_whom(rig).await;
    let wrote = |(who, n): (&String, &u64)| (who.clone(), n - whom.get(who).copied().unwrap_or(0));
    let whose = since.iter().map(wrote).filter(|(_, n)| *n > 0).collect();
    (cost, whose, ran(rig, &said).await)
}

/// A run a card's moment began: what it is a run of, and what set it
/// off, with whether that was said in the card's moment or before it.
#[derive(Clone, Debug)]
struct Ran {
    /// The blueprint and the flow, and the topic that set it off: what
    /// two cards' runs are compared by.
    kind: String,
    /// The whole of it, for saying which.
    said: String,
}

/// The runs said to have started in `said`, each by what it is a run of
/// and what set it off (`RunInfo.event`, as `rows.rs` tells runs apart).
async fn ran(rig: &Rig, said: &[kernel::Event]) -> Vec<Ran> {
    let numbers: BTreeSet<u64> = said.iter().map(|event| rig.number(event.seq)).collect();
    let mut ran = Vec::new();
    for event in said.iter().filter(|event| event.topic == "run.started") {
        let (blueprint, flow) =
            (event.data["blueprint"].as_str().unwrap_or("?"), event.data["flow"].as_str().unwrap_or("?"));
        let run = event.source.run.clone().unwrap_or_default();
        let info = rig.node().app.engine.run(&run).await.ok();
        let by = info.as_ref().map(|info| info.event.clone()).unwrap_or(Value::Null);
        let topic = by["topic"].as_str().unwrap_or("nothing");
        let when = match by["seq"].as_u64() {
            None => "",
            Some(number) if numbers.contains(&number) => "",
            Some(_) => ", said before the card",
        };
        let whole = by["data"].to_string();
        let mut data: String = whole.chars().take(200).collect();
        if data.len() < whole.len() {
            data.push('…');
        }
        let instance = event.source.instance.clone().unwrap_or_default();
        ran.push(Ran {
            kind: format!("{blueprint} {flow}, set off by {topic}{when}"),
            said: format!("{run} of {instance} ({blueprint} {flow}), set off by {topic}{when} {data}"),
        });
    }
    ran
}

/// What one card's runs had that the other's did not, by what they were
/// runs of and what set them off, each with its runs whole.
fn ran_apart(little: &[Ran], much: &[Ran]) -> String {
    let count = |runs: &[Ran]| {
        let mut by: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for run in runs {
            by.entry(run.kind.clone()).or_default().push(run.said.clone());
        }
        by
    };
    let (few, many) = (count(little), count(much));
    let mut said = Vec::new();
    for kind in few.keys().chain(many.keys()).collect::<BTreeSet<_>>() {
        let (was, now) =
            (few.get(kind).map_or(&[][..], Vec::as_slice), many.get(kind).map_or(&[][..], Vec::as_slice));
        if was.len() != now.len() {
            said.push(format!("  {kind}: {} beside four cards and {} beside 254", was.len(), now.len()));
            for (side, runs) in [("four", was), ("254", now)] {
                for run in runs {
                    said.push(format!("    beside {side}: {run}"));
                }
            }
        }
    }
    if said.is_empty() {
        "  none: the same runs by what they were of and what set them off".into()
    } else {
        said.join("\n")
    }
}

/// Has every journal's database count its own steps from here on, each
/// thing its program does (`progress_handler`), on the one connection
/// its thread asks everything on.
async fn stepping(rig: &Rig, watching: &Watching) {
    for db in rig.node().app.journal.dbs() {
        let counted = watching.steps.clone();
        let told = db.call(move |connection| {
            connection.progress_handler(
                1,
                Some(move || {
                    counted.fetch_add(1, Ordering::Relaxed);
                    false
                }),
            )
        });
        told.await.unwrap();
    }
}

/// The pieces of work done so far, writing or not, by database and by
/// whose work they were, each with how many and how long they took.
async fn work_by_whom(rig: &Rig) -> BTreeMap<String, (u64, f64)> {
    let journal = &rig.node().app.journal;
    journal.flush_metrics().await.unwrap();
    let query = journal::MetricQuery {
        name: "db.work".into(),
        group_by: vec!["db".into(), "caller".into(), "kind".into()],
        ..Default::default()
    };
    let mut by = BTreeMap::new();
    for series in journal.query_metrics(query).await.unwrap() {
        let label = |key: &str| series.labels.get(key).cloned().unwrap_or_default();
        let whom = format!("{} {} by {}", label("db"), label("kind"), label("caller"));
        by.insert(whom, (series.total.count, series.total.sum));
    }
    by
}

/// What a card cost, with the pieces of work that were done for it by
/// whose they were.
async fn whose_card(
    rig: &Rig,
    watching: &mut Watching,
    title: &str,
) -> (Cost, BTreeMap<String, (u64, f64)>, Vec<Ran>) {
    let whom = work_by_whom(rig).await;
    let (cost, _, ran) = card(rig, watching, title).await;
    let since = work_by_whom(rig).await;
    let mut done = BTreeMap::new();
    for (who, (count, ms)) in since {
        let (was, was_ms) = whom.get(&who).copied().unwrap_or((0, 0.0));
        if count > was {
            done.insert(who, (count - was, ms - was_ms));
        }
    }
    (cost, done, ran)
}

/// Cards added to the shop's board in a column, each a card nobody
/// does anything with there.
async fn many_in(rig: &Rig, column: &str, how_many: usize) {
    for n in 0..how_many {
        let card = json!({ "title": format!("Waits in {column}, {n}"), "status": column });
        rig.send("/api/board/shop/items", card).await;
    }
}

/// How many of the databases' own steps reading what the card called
/// `title` cost, by round, takes (`GET /api/board/items/{id}/cost`).
async fn cost_read(rig: &Rig, watching: &Watching, title: &str) -> u64 {
    let board = rig.read("/api/board/shop").await;
    let card = board["items"].as_array().unwrap().iter().find(|card| card["title"] == title).cloned();
    let id = card.unwrap()["id"].as_str().unwrap().to_string();
    let before = watching.steps.load(Ordering::Relaxed);
    let cost = rig.read(&format!("/api/board/items/{id}/cost")).await;
    let steps = watching.steps.load(Ordering::Relaxed) - before;
    assert_eq!(cost["rounds"][0]["decided"]["came"], "passed", "{cost}");
    steps
}

#[tokio::test]
async fn what_a_card_costs_is_the_same_beside_three_cards_as_beside_two_hundred_and_fifty_three() {
    if crate::one_journal() {
        return;
    }
    let clock = kernel::ManualClock::new();
    let (rig, _) = rig_told(move |config| config.clock = Some(Arc::new(clock))).await;
    shop(&rig, json!({ "reviewers": 2 })).await;
    change(&rig, "stages", "review", json!({ "seats": 1, "needed": 1 })).await;
    let mut watching = Watching::new(&rig);
    stepping(&rig, &watching).await;
    watching.rested(&rig).await;
    card(&rig, &mut watching, "Warm the factory").await;

    // Two waiting beside the one done: a card worked there is the fourth.
    many_in(&rig, "draft", 2).await;
    watching.rested(&rig).await;
    let few = whose_card(&rig, &mut watching, "Add a cart").await;
    // Then two hundred more waiting, and fifty done.
    many_in(&rig, "draft", 200).await;
    many_in(&rig, "done", 50).await;
    watching.rested(&rig).await;
    let many = whose_card(&rig, &mut watching, "Add a till").await;

    for ((cost, done, _), cards) in [(&few, 4), (&many, 254)] {
        println!("a card on a board of {cards}: {cost:?}");
        println!("  pieces of work, by database, by whether they wrote and by whose they were:");
        for (who, (count, ms)) in done {
            println!("    {who}: {count}, {ms:.2} ms");
        }
    }
    // What the factory does whichever card's moment it falls in is
    // counted apart (`Cost::own`): the cards added between the two bring
    // some ten keepings of handlers' places into the second card, and
    // where each falls goes by how the factory's work fell before it (a
    // card of the same board came to seven and to ten, and the two
    // windows came to 214 and 227 writes). Without it either card came to
    // 590 pieces of work and 204 writes in every run.
    println!("the runs that differ:\n{}", ran_apart(&few.2, &many.2));
    let (little, much) = (few.0.own(), many.0.own());
    let (little, much) = (&little, &much);
    let mut grew = Vec::new();
    // And what a card cost, read for one card, reads that card's turns
    // and history by their indexes and nothing of the cards beside it.
    let (cart, till) =
        (cost_read(&rig, &watching, "Add a cart").await, cost_read(&rig, &watching, "Add a till").await);
    println!("what a card cost, read: {cart} steps beside four cards and {till} beside 254");
    if till > cart + cart / 50 {
        grew.push(format!(
            "reading what a card cost: {cart} steps on a board of four cards and {till} on one of 254"
        ));
    }
    for ((figure, was), (_, now)) in little.figures().into_iter().zip(much.figures()) {
        // Frames are what was written to the log, which a board read whole
        // writes nothing to, and they move by where pages fall from one
        // card to the next: two single cards of the same board differed by
        // seven in a hundred once the runner kept less. What a
        // read of the board costs is held below, by the databases' steps.
        if figure == "frames" {
            continue;
        }
        // Played with gathering forced (`ways gathered`), a commit keeps
        // whatever was waiting as it was begun, so how many there are is
        // how the machine's work fell from one run to the next (the same
        // card came to 185 and to 196 on one board): what the card asked
        // for is the writes, and the databases' steps below, which hold.
        if figure == "commits" && journal::gathering_forced() {
            continue;
        }
        // The same, to the one: nothing of what is left moves from one
        // run to the next.
        if now != was {
            grew.push(format!("{figure}: {was} on a board of four cards and {now} on one of 254"));
        }
    }
    // Every step of a database's own is counted, so an index a level
    // deeper is a few more for each thing looked up in it: a board of 254
    // read whole once is some thousands.
    if much.steps > little.steps + little.steps / 50 {
        let (was, now) = (little.steps, much.steps);
        grew.push(format!(
            "the databases' own steps: {was} on a board of four cards and {now} on one of 254"
        ));
    }
    assert!(
        grew.is_empty(),
        "what a card costs is not the same with the cards beside it on its board:\n{}\n\nThe runs that differ, \
         by what they were runs of and what set them off:\n{}\n\nRun this test with \
         `--nocapture`: it prints the pieces of work each card asked for, by database, by whether they \
         wrote and by whose work they were, with how long they took, and something that reads the board \
         whole for one card is where the time grew. Ask for one card by its id (`Collections::one`), and \
         for the cards of one column, or those that wait on others, by what they say (`studio::column`).",
        grew.join("\n"),
        ran_apart(&few.2, &many.2),
    );
}

/// A table of the shop's own, nothing of the fleet's, whose rows each
/// name a card and are each written by a run of a blueprint of the
/// project's (as a table of a command's runs is, or of how long each card
/// took), with so many rows more in it.
async fn notes(rig: &Rig, from: usize, how_many: usize) {
    const NOTE: &str = "blueprint: note\npermissions: [records]\nparams:\n  title: { type: string }\nsteps:\n  kept:\n    do: record.add\n    with:\n      project: shop\n      collection: notes\n      fields: { title: \"${params.title}\", card: shop-1 }\n      declaring: { title: { type: text }, card: { type: record, of: board } }\n";
    for n in from..from + how_many {
        let body = json!({
            "source": NOTE, "project": "shop", "key": format!("note-{n}"),
            "params": { "title": format!("Note {n}") }, "wait": true,
        });
        let applied = rig.send("/api/apply", body).await;
        assert_eq!(applied["finished"]["status"], "ok", "{applied}");
    }
}

#[tokio::test]
async fn what_a_card_costs_is_the_same_beside_a_table_of_three_rows_as_beside_one_of_sixty() {
    if crate::one_journal() {
        return;
    }
    let clock = kernel::ManualClock::new();
    let (rig, _) = rig_told(move |config| config.clock = Some(Arc::new(clock))).await;
    shop(&rig, json!({ "reviewers": 2 })).await;
    change(&rig, "stages", "review", json!({ "seats": 1, "needed": 1 })).await;
    let mut watching = Watching::new(&rig);
    stepping(&rig, &watching).await;
    watching.rested(&rig).await;
    card(&rig, &mut watching, "Warm the factory").await;

    // Another table of the project, of three rows and then of sixty, each
    // written by a run: a card's journey reads none of it, and none of
    // the runs that wrote it.
    // And a card nobody works, moved and moved back: what one move costs,
    // where everything a move does is nearly all of what is counted.
    let waits = rig.send("/api/board/shop/items", json!({ "title": "Moved about", "status": "draft" })).await;
    let waits = waits["id"].as_str().unwrap().to_string();
    let moved = async |watching: &mut Watching| {
        let before = watching.rested(&rig).await.steps;
        move_card(&rig, &waits, "done", None).await;
        move_card(&rig, &waits, "draft", None).await;
        watching.rested(&rig).await.steps - before
    };
    notes(&rig, 0, 3).await;
    watching.rested(&rig).await;
    let few = whose_card(&rig, &mut watching, "Add a cart").await;
    let few_moved = moved(&mut watching).await;
    notes(&rig, 3, 57).await;
    watching.rested(&rig).await;
    let many = whose_card(&rig, &mut watching, "Add a till").await;
    let many_moved = moved(&mut watching).await;
    println!("a card moved and moved back: {few_moved} steps beside three rows, {many_moved} beside sixty");
    assert!(
        many_moved <= few_moved + few_moved / 50,
        "a move costs more the more rows another table of its project has: the databases' own steps of \
         a card moved and moved back were {few_moved} beside a table of three rows and {many_moved} beside \
         one of sixty"
    );
    for ((cost, done, _), rows) in [(&few, 3), (&many, 60)] {
        println!("a card beside a table of {rows} rows: {cost:?}");
        for (who, (count, ms)) in done {
            println!("    {who}: {count}, {ms:.2} ms");
        }
    }
    let (was, now) = (few.0.steps, many.0.steps);
    assert!(
        now <= was + was / 50,
        "what a card costs grows with the rows of another table of its project: the databases' own steps \
         were {was} beside a table of three rows and {now} beside one of sixty. Run this test with \
         `--nocapture`: it prints the pieces of work each card asked for, by whose they were; something that \
         reads that table, or the runs that wrote it, whole on every move is where they grew."
    );
}

/// A row of `segments` that runs a command in a card's round, before
/// its work is kept.
fn command() -> Value {
    json!({ "title": "Check", "stage": "work", "order": 15, "does": "std/run",
        "command": crate::standin::line(&["say", "ok"]) })
}

/// A row of `segments` that lays a second review of two seats, both of
/// which have to approve, after the usual one.
fn council() -> Value {
    json!({ "title": "Second look", "stage": "review", "order": 15, "does": "std/review", "seats": 2, "needed": 2 })
}

/// What `[card]` of `build-budget.toml` allows, by name.
fn budget() -> BTreeMap<String, u64> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build-budget.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut within = false;
    let mut allowed = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            within = line == "[card]";
        } else if within && let Some((key, value)) = line.split_once('=') {
            let value = value.split('#').next().unwrap_or_default().trim();
            allowed.insert(key.trim().to_string(), value.parse().expect("a whole number in [card]"));
        }
    }
    allowed
}

#[tokio::test]
async fn what_a_card_costs_the_factory_is_held_to_its_budget() {
    if crate::one_journal() {
        return;
    }
    // A clock that stands still: nothing the factory does on a timer
    // (measurements written out, a turn's time running out) comes in the
    // middle of a card.
    let clock = kernel::ManualClock::new();
    let (rig, _) = rig_told(move |config| config.clock = Some(Arc::new(clock))).await;
    shop(&rig, json!({ "reviewers": 2 })).await;
    change(&rig, "stages", "review", json!({ "seats": 1, "needed": 1 })).await;
    let mut watching = Watching::new(&rig);
    watching.rested(&rig).await;
    // The first card pays for each table's first pages and what is first
    // kept of each text, and is held to nothing.
    card(&rig, &mut watching, "Warm the factory").await;

    let mut costs = Vec::new();
    costs.push(("plain", "worked, reviewed and merged", card(&rig, &mut watching, "Add a cart").await));
    add_row(&rig, "segments", command()).await;
    watching.rested(&rig).await;
    costs.push((
        "command",
        "with a command laid into its round",
        card(&rig, &mut watching, "Add a till").await,
    ));
    remove_row(&rig, "segments", "Check").await;
    add_row(&rig, "segments", council()).await;
    watching.rested(&rig).await;
    let after = card(&rig, &mut watching, "Add a receipt").await;
    costs.push(("council", "with a second review of two seats laid in", after));

    let allowed = budget();
    let mut over = Vec::new();
    for (kind, what, (cost, whose, _)) in &costs {
        let bytes = cost.frames * FRAME_BYTES;
        println!("a card {what} ({kind}): {cost:?}, {bytes} bytes to the project's log");
        println!("  pieces of work that wrote, by database and by whose: {whose:?}");
        // What the factory does whichever card's moment it falls in is
        // counted apart (`Cost::own`), as beside many cards: with it a
        // card's writes came to as many as seven more or fewer from one
        // run to the next, and without it the same in every run.
        for (figure, now) in cost.own().figures() {
            let key = format!("{kind}_{figure}");
            let most = *allowed
                .get(&key)
                .unwrap_or_else(|| panic!("build-budget.toml says nothing of `{key}` in [card]"));
            if now > most {
                over.push(format!(
                    "a card {what} came to {now} {figure} where it may come to {most} ({}% more; `{key}` in \
                     [card] of build-budget.toml). Of the writes, by database and by whose work they were: \
                     {whose:?}",
                    (now - most) * 100 / most.max(1)
                ));
            }
        }
    }
    assert!(
        over.is_empty(),
        "what a card costs has grown:\n{}\n\nFind what grew before anything else: run this test with \
         `--nocapture` before the change and after it, and read what each kind of card came to and which \
         crate's writes grew; `docs/build-times.md` (*What one card costs*, *What a card is held to*) says \
         how a card was measured whole and where its writes went. Raise the figure only for something worth \
         having, and say beside it in build-budget.toml what that was and why it is worth what it costs.",
        over.join("\n")
    );
}
