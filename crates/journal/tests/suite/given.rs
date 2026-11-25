//! One giver of numbers for the factory: a journal draws a run of them at
//! a time and gives them out in order; how far the giver has given is
//! kept before any number of a run is an event's, in the write of the
//! first of them; and so nothing is given twice, wherever the journal is
//! stopped, by a journal made now or one an earlier program made, by a
//! copy of one, or by two that draw from the one giver.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use journal::giver::{DRAWN, RUN};
use journal::{Db, EventQuery, Journal};
use kernel::{Clock, ManualClock, Source};
use serde_json::json;

use crate::scratch;

fn clock() -> Arc<dyn Clock> {
    Arc::new(ManualClock::new())
}

async fn opened(path: &Path) -> Journal {
    Journal::open(Db::open(path).unwrap(), clock(), "dev").await.unwrap()
}

/// What the journal of a project of these tests is called.
fn shop() -> String {
    journal::of_project("shop")
}

/// Says one thing, and gives the number it was given.
async fn say(journal: &Journal) -> u64 {
    journal.emit("said", Source::default(), json!({})).await.unwrap().seq
}

/// Says `many` things in one write, and gives their numbers.
async fn say_many(journal: &Journal, many: u64) -> Vec<u64> {
    let wrote = journal.write(move |writing| {
        (0..many).map(|_| writing.say("said", Source::default(), json!({}))).collect::<Result<Vec<u64>, _>>()
    });
    wrote.await.unwrap().0
}

/// The giver's mark as the journal's database keeps it, where it keeps one.
async fn mark(journal: &Journal) -> Option<u64> {
    journal.kv_get(DRAWN.0, DRAWN.1).await.unwrap().and_then(|kept| kept.as_u64())
}

/// The number of every event the journal holds, in the order they are kept.
async fn numbers(journal: &Journal) -> Vec<u64> {
    let all = EventQuery { limit: usize::MAX, ..Default::default() };
    journal.read(all).await.unwrap().iter().map(|event| event.seq).collect()
}

/// Closes a journal's database and waits for its file to be let go, as a
/// daemon that stops does.
async fn closed(journal: Journal) {
    journal.db().close();
    journal.db().closed().await;
}

/// What a daemon killed at this moment leaves: the database and its log
/// as they are on the disk, copied to `to` while the journal has them
/// open. Nothing is being written: whoever asks has had every answer.
fn left_by_a_stop(dir: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for file in ["journal.db", "journal.db-wal"] {
        std::fs::copy(dir.join(file), to.join(file)).unwrap();
    }
}

/// Takes the giver's mark out of the database at `path`, which nothing
/// has open: what is left is a journal as the program before this made
/// it, which gave each event a number one above the last and kept no mark.
fn as_the_program_before_left_it(path: &Path) {
    let connection = journal::copy::Connection::open(path).unwrap();
    assert_eq!(
        connection.execute("DELETE FROM kv WHERE namespace = ?1 AND key = ?2", [DRAWN.0, DRAWN.1]).unwrap(),
        1
    );
}

#[tokio::test]
async fn a_run_of_numbers_is_drawn_in_the_write_of_its_first_event_and_is_no_write_of_its_own() {
    let dir = scratch("drawn");
    let journal = opened(&dir.join("journal.db")).await;
    assert_eq!(mark(&journal).await, None, "nothing is drawn, and nothing written, until something is said");
    let before = journal.db().writes();

    // The first thing said draws a run, in its own write.
    assert_eq!(say(&journal).await, 1);
    assert_eq!(mark(&journal).await, Some(RUN));
    assert_eq!(journal.db().writes(), before + 1);
    // The rest of the run is given with nothing more kept of the giver.
    assert_eq!(say_many(&journal, RUN - 1).await, (2..=RUN).collect::<Vec<_>>());
    assert_eq!(mark(&journal).await, Some(RUN));
    // And the next draws another, where the last ended, in its own write.
    assert_eq!(say(&journal).await, RUN + 1);
    assert_eq!(mark(&journal).await, Some(2 * RUN));
    assert_eq!(journal.db().writes(), before + 3, "three writes for what was said three times");
    assert_eq!(journal.giver().drawn(), 2 * RUN);

    // One write that says more than a run holds draws as often as it has to.
    let said = say_many(&journal, 2 * RUN).await;
    assert_eq!(said, (RUN + 2..=3 * RUN + 1).collect::<Vec<_>>());
    assert_eq!(mark(&journal).await, Some(4 * RUN));
    assert_eq!(journal.db().writes(), before + 4);
}

#[tokio::test]
async fn nothing_is_given_twice_wherever_the_journal_is_stopped_about_the_end_of_a_run() {
    let dir = scratch("stopped");
    let journal = opened(&dir.join("journal.db")).await;
    // Up to three short of the end of its first run, then one at a time
    // over the end and into the next: and before each, what a daemon
    // killed at that moment would leave is put aside.
    let mut given = say_many(&journal, RUN - 3).await;
    let mut stops = Vec::new();
    for stop in 0..6 {
        let left = dir.join(format!("left-{stop}"));
        left_by_a_stop(&dir, &left);
        stops.push((left, given.clone()));
        given.push(say(&journal).await);
    }
    assert_eq!(given, (1..=RUN + 3).collect::<Vec<_>>(), "a journal that goes on gives its runs end to end");

    for (left, given) in stops {
        let journal = opened(&left.join("journal.db")).await;
        let last = *given.last().unwrap();
        assert_eq!(numbers(&journal).await, given, "what was answered before the stop is there");
        // How far the giver had given was kept before any of it was given.
        let kept = mark(&journal).await.unwrap();
        assert!(kept >= last, "the mark kept is {kept}, and {last} was given");
        // What is said next is above all of it: what was left of the run is a gap.
        let next = say(&journal).await;
        assert_eq!(next, kept + 1);
        assert_eq!(numbers(&journal).await.len(), given.len() + 1);
        closed(journal).await;
    }
}

#[tokio::test]
async fn a_journal_shut_at_a_place_has_kept_how_far_it_drew_and_what_was_said_after_draws_nothing() {
    let dir = scratch("shut");
    for (place, goes_on_from) in [(RUN, RUN + 1), (RUN + 1, 2 * RUN + 1)] {
        let path = dir.join(format!("shut-at-{place}.db"));
        let journal = opened(&path).await;
        journal.shut_after(move |event| event.seq == place);
        // The place is the last of a run, or the first of the next, which
        // is drawn in the write that reaches it.
        let said = journal.write(move |writing| {
            for _ in 0..place {
                writing.say("said", Source::default(), json!({}))?;
            }
            Ok(())
        });
        said.await.unwrap();
        journal.shut().await;
        // Said after the place, and never answered: it would have drawn.
        let late = tokio::spawn({
            let journal = journal.clone();
            async move { say(&journal).await }
        });
        for _ in 0..50 {
            tokio::task::yield_now().await;
        }
        assert!(!late.is_finished());
        late.abort();
        closed(journal).await;

        let journal = opened(&path).await;
        assert_eq!(numbers(&journal).await, (1..=place).collect::<Vec<_>>());
        assert_eq!(mark(&journal).await, Some(goes_on_from - 1));
        assert_eq!(say(&journal).await, goes_on_from);
        closed(journal).await;
    }
}

#[tokio::test]
async fn a_write_that_fails_having_drawn_a_run_keeps_no_mark_and_gives_no_number_away() {
    let dir = scratch("failed");
    let journal = opened(&dir.join("journal.db")).await;
    let failed = journal.write(|writing| {
        writing.say("said", Source::default(), json!({}))?;
        writing.transaction().execute("INSERT INTO nothing_there VALUES (1)", [])?;
        Ok(())
    });
    assert!(failed.await.is_err());
    assert_eq!(mark(&journal).await, None, "the mark it raised was taken back with what it wrote");
    assert_eq!(journal.last_seq(), 0);
    // The run it drew is the next one's.
    assert_eq!(say(&journal).await, 1);
    assert_eq!(mark(&journal).await, Some(RUN));

    // And one that fails over the end of a run, having said the last of
    // one and the first of the next.
    say_many(&journal, RUN - 2).await;
    let failed = journal.write(|writing| {
        for _ in 0..3 {
            writing.say("said", Source::default(), json!({}))?;
        }
        writing.transaction().execute("INSERT INTO nothing_there VALUES (1)", [])?;
        Ok(())
    });
    assert!(failed.await.is_err());
    assert_eq!(mark(&journal).await, Some(RUN));
    assert_eq!(say_many(&journal, 3).await, [RUN, RUN + 1, RUN + 2]);
    assert_eq!(mark(&journal).await, Some(2 * RUN));
}

#[tokio::test]
async fn a_commit_that_cannot_be_made_keeps_no_mark_and_its_numbers_are_the_next_ones() {
    let dir = scratch("lost");
    let path = dir.join("journal.db");
    let journal = opened(&path).await;
    // A part that names a whole which is not there is taken as it is
    // written and refused as the commit is made: nothing the factory
    // keeps is checked so, and this is, so that a commit can fail.
    let made = "CREATE TABLE wholes (name TEXT PRIMARY KEY);
                CREATE TABLE parts (of TEXT NOT NULL REFERENCES wholes (name) DEFERRABLE INITIALLY DEFERRED);";
    journal.db().call(move |connection| connection.execute_batch(made)).await.unwrap();
    let lost = |journal: &Journal, many: u64| {
        let wrote = journal.write(move |writing| {
            for _ in 0..many {
                writing.say("said", Source::default(), json!({}))?;
            }
            writing.transaction().execute("INSERT INTO parts VALUES ('no whole')", []).map(|_| ())
        });
        async move { assert!(wrote.await.unwrap_err().message.contains("could not be kept")) }
    };

    // The first thing ever said, which drew the first run: neither is kept.
    lost(&journal, 1).await;
    assert_eq!((mark(&journal).await, journal.last_seq()), (None, 0));
    assert_eq!(say(&journal).await, 1);
    // In the middle of a run, whose mark was kept before.
    lost(&journal, 3).await;
    assert_eq!(say_many(&journal, RUN - 2).await, (2..RUN).collect::<Vec<_>>());
    // And over the end of one: the last of it, and the first of a run
    // whose mark went with the commit.
    lost(&journal, 3).await;
    assert_eq!(mark(&journal).await, Some(RUN));
    assert_eq!(say_many(&journal, 3).await, [RUN, RUN + 1, RUN + 2]);
    assert_eq!(mark(&journal).await, Some(2 * RUN));
    closed(journal).await;
    // What the disk holds says the same to a journal opened on it.
    let journal = opened(&path).await;
    assert_eq!(numbers(&journal).await, (1..=RUN + 2).collect::<Vec<_>>());
    assert_eq!(say(&journal).await, 2 * RUN + 1);
}

#[tokio::test]
async fn two_journals_drawing_from_one_giver_share_no_number_and_each_gives_its_own_in_order() {
    let factory = Journal::open(Db::memory().unwrap(), clock(), "dev").await.unwrap();
    let project =
        Journal::open_drawing(Db::memory().unwrap(), clock(), "dev", shop(), &factory.giver()).await.unwrap();

    // Side by side: each says a run and a half, one thing at a time and
    // several at once, giving way to the other as it goes.
    let saying = |journal: &Journal| {
        let journal = journal.clone();
        tokio::spawn(async move {
            let mut given = Vec::new();
            for round in 0..RUN / 8 {
                match round % 3 {
                    0 => given.extend(say_many(&journal, 16).await),
                    _ => {
                        for _ in 0..10 {
                            given.push(say(&journal).await);
                        }
                    }
                }
            }
            given
        })
    };
    let (of_factory, of_project) = (saying(&factory), saying(&project));
    let (of_factory, of_project) = (of_factory.await.unwrap(), of_project.await.unwrap());

    for (whose, given) in [("the factory's", &of_factory), ("the project's", &of_project)] {
        assert!(given.len() as u64 > RUN, "{whose} drew more than once: {}", given.len());
        assert!(given.windows(2).all(|two| two[0] < two[1]), "{whose} numbers are not in order");
    }
    assert_eq!(numbers(&factory).await, of_factory);
    assert_eq!(numbers(&project).await, of_project);
    let mut all: Vec<u64> = of_factory.iter().chain(&of_project).copied().collect();
    all.sort_unstable();
    let many = all.len();
    all.dedup();
    assert_eq!(all.len(), many, "a number was given by both");
    // There are gaps where the other drew, and the mark is kept where the
    // giver is, above all either was given.
    let gaps = |given: &Vec<u64>| given.windows(2).any(|two| two[1] > two[0] + 1);
    assert!(gaps(&of_factory) || gaps(&of_project));
    let kept = mark(&factory).await.unwrap();
    assert!(kept >= *all.last().unwrap() && kept == factory.giver().drawn());
    assert_eq!(mark(&project).await, None, "a journal that draws elsewhere keeps no mark of its own");
}

#[tokio::test]
async fn a_journal_is_called_for_whose_it_is_and_no_projects_is_called_as_the_factorys_is() {
    let dir = scratch("called");
    std::fs::create_dir_all(dir.join("somewhere")).unwrap();
    let factory = opened(&dir.join("somewhere").join("journal.db")).await;
    assert_eq!(factory.name(), journal::FACTORY);
    let giver = factory.giver();
    let draws = |name: String| {
        let giver = giver.clone();
        async move { Journal::open_drawing(Db::memory().unwrap(), clock(), "dev", name, &giver).await }
    };
    // A project's is called by its project, whatever the project is called.
    let project = draws(journal::of_project(journal::FACTORY)).await.unwrap();
    assert_eq!(project.name(), "project:factory");
    assert_ne!(project.name(), factory.name());
    // And one that draws from another's giver is never called as that one is.
    for name in [journal::FACTORY.to_string(), String::new()] {
        let refused = draws(name).await.err().expect("refused");
        assert!(refused.message.contains("something of its own"), "{}", refused.message);
    }
    // Where its file is says nothing of what it is called: moved, and
    // opened again, it is the journal it was.
    closed(factory).await;
    std::fs::rename(dir.join("somewhere"), dir.join("elsewhere")).unwrap();
    assert_eq!(opened(&dir.join("elsewhere").join("journal.db")).await.name(), journal::FACTORY);
}

#[tokio::test]
async fn two_journals_on_one_giver_stopped_and_started_give_nothing_either_gave_before() {
    let dir = scratch("two");
    let (factory, project) = (dir.join("factory.db"), dir.join("project.db"));
    let mut all = Vec::new();
    for _ in 0..3 {
        let of_factory = opened(&factory).await;
        let of_project =
            Journal::open_drawing(Db::open(&project).unwrap(), clock(), "dev", shop(), &of_factory.giver())
                .await
                .unwrap();
        let mut here = Vec::new();
        for _ in 0..3 {
            here.push(say(&of_project).await);
            here.push(say(&of_factory).await);
        }
        here.extend(say_many(&of_project, RUN).await);
        let above = all.iter().copied().max().unwrap_or(0);
        assert!(here.iter().all(|number| *number > above), "{here:?} after {above}");
        all.extend(here);
        // The project's is closed first: the giver is kept in the factory's.
        closed(of_project).await;
        closed(of_factory).await;
    }
    let many = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), many, "a number was given twice");
}

#[tokio::test]
async fn a_journal_that_numbered_its_own_and_then_draws_elsewhere_is_given_nothing_it_held() {
    let dir = scratch("joined");
    let (factory, project) = (dir.join("factory.db"), dir.join("project.db"));
    // A journal of its own, which said more than a factory made now has.
    let held = {
        let journal = opened(&project).await;
        let held = say_many(&journal, 5).await;
        closed(journal).await;
        held
    };
    let of_factory = opened(&factory).await;
    let of_project =
        Journal::open_drawing(Db::open(&project).unwrap(), clock(), "dev", shop(), &of_factory.giver())
            .await
            .unwrap();
    // The giver was told what it held, and gives both what is above it.
    let first = say(&of_factory).await;
    let next = say(&of_project).await;
    assert_eq!(first, 6);
    assert_eq!(next, RUN + 6);
    assert_eq!(numbers(&of_project).await, [held, vec![next]].concat());
}

#[tokio::test]
async fn a_journal_the_program_before_made_is_taken_up_with_no_mark_and_gives_no_number_twice() {
    let dir = scratch("unmarked");
    let path = dir.join("journal.db");
    {
        // Four said and the last two taken out, by a program that gave
        // each the number above the last and kept no mark.
        let journal = opened(&path).await;
        say_many(&journal, 2).await;
        for _ in 0..2 {
            journal.emit("room.said", Source::default(), json!({ "room": "r_1" })).await.unwrap();
        }
        assert_eq!(journal.forget_events("room.said", "room", "r_1").await.unwrap(), 2);
        closed(journal).await;
    }
    as_the_program_before_left_it(&path);

    let journal = opened(&path).await;
    // Opening it kept nothing of the giver: there is nothing to wait for
    // however much it holds.
    assert_eq!(mark(&journal).await, None);
    assert_eq!(journal.last_seq(), 4, "the highest it ever gave, though its events end at 2");
    // It goes on from just above that, and from then keeps a mark.
    assert_eq!(say(&journal).await, 5);
    assert_eq!(mark(&journal).await, Some(4 + RUN));
    closed(journal).await;
    let journal = opened(&path).await;
    assert_eq!(say(&journal).await, 4 + RUN + 1);
    assert_eq!(numbers(&journal).await, [1, 2, 5, 4 + RUN + 1]);
}

#[tokio::test]
async fn events_taken_out_as_old_and_the_journal_opened_again_give_no_number_twice() {
    let dir = scratch("pruned");
    for from_before in [false, true] {
        let path = dir.join(format!("before-{from_before}.db"));
        let clock = ManualClock::new();
        let journal = Journal::open(Db::open(&path).unwrap(), Arc::new(clock.clone()), "dev").await.unwrap();
        say_many(&journal, 5).await;
        clock.advance(Duration::from_secs(60));
        // Every one of them is old, the last among them, and none is kept.
        assert_eq!(journal.prune_events(journal.now_ms(), 0).await.unwrap(), 5);
        assert!(numbers(&journal).await.is_empty());
        closed(journal).await;
        if from_before {
            as_the_program_before_left_it(&path);
        }

        let journal = opened(&path).await;
        assert_eq!(journal.last_seq(), 5, "what it gave is written down with what took the events out");
        assert_eq!(journal.held_when_opened(), 0);
        // By the mark it goes on above the run it had drawn; with none,
        // as a journal from before, above what was written down.
        assert_eq!(say(&journal).await, if from_before { 6 } else { RUN + 1 });
        closed(journal).await;
    }
}

#[tokio::test]
async fn the_newest_events_kept_whatever_their_age_are_counted_though_their_numbers_have_gaps() {
    let dir = scratch("counted");
    let path = dir.join("journal.db");
    let clock = ManualClock::new();
    let opened = || Journal::open(Db::open(&path).unwrap(), Arc::new(clock.clone()), "dev");
    // Three said at each of three starts: nine events, and two gaps of
    // nearly a run between them.
    let mut given = Vec::new();
    for _ in 0..3 {
        let journal = opened().await.unwrap();
        given.extend(say_many(&journal, 3).await);
        closed(journal).await;
    }
    assert_eq!(given, [1, 2, 3, RUN + 1, RUN + 2, RUN + 3, 2 * RUN + 1, 2 * RUN + 2, 2 * RUN + 3]);
    clock.advance(Duration::from_secs(60));

    // All of them are old. More are to be kept than there are: none goes.
    let journal = opened().await.unwrap();
    let old = journal.now_ms();
    assert_eq!(journal.prune_events(old, 20).await.unwrap(), 0);
    // The newest seven are kept, which reach back over both gaps; then
    // the newest four, over one; then three, which are the last start's.
    assert_eq!(journal.prune_events(old, 7).await.unwrap(), 2);
    assert_eq!(numbers(&journal).await, given[2..]);
    assert_eq!(journal.prune_events(old, 4).await.unwrap(), 3);
    assert_eq!(numbers(&journal).await, given[5..]);
    assert_eq!(journal.prune_events(old, 3).await.unwrap(), 1);
    assert_eq!(numbers(&journal).await, given[6..]);
    // And what it goes on with is above them all, counted from there.
    assert_eq!(say(&journal).await, 3 * RUN + 1);
    assert_eq!(journal.prune_events(old, 2).await.unwrap(), 2);
    assert_eq!(numbers(&journal).await, [2 * RUN + 3, 3 * RUN + 1]);
}

#[tokio::test]
async fn a_copy_of_a_journal_goes_on_above_all_the_original_had_given() {
    let dir = scratch("copied-on");
    let (original, copy) = (dir.join("journal.db"), dir.join("copy.db"));
    let journal = opened(&original).await;
    say_many(&journal, 2).await;
    // The third is said and taken out: the copy holds two events, and
    // three numbers were given.
    journal.emit("room.said", Source::default(), json!({ "room": "r_1" })).await.unwrap();
    assert_eq!(journal.forget_events("room.said", "room", "r_1").await.unwrap(), 1);

    let (from, to) = (original.clone(), copy.clone());
    tokio::task::spawn_blocking(move || journal::copy::take(&from, &to)).await.unwrap().unwrap();
    let connection = journal::copy::open(&copy).unwrap();
    journal::copy::leave_out(&connection).unwrap();
    // Held to what travels, of which nothing else is said here: the
    // giver's mark is the journal's own, and goes with the events.
    journal::copy::keep_state(&connection, &[]).unwrap();
    journal::copy::keep_tables(&connection, journal::copy::TABLES).unwrap();
    journal::copy::seal(connection).unwrap();
    // The original goes on meanwhile, in the run it has.
    assert_eq!(say(&journal).await, 4);

    let copied = opened(&copy).await;
    assert_eq!(numbers(&copied).await, [1, 2]);
    assert_eq!(mark(&copied).await, Some(RUN), "the mark went with it");
    assert_eq!(say(&copied).await, RUN + 1, "above all the original had drawn when it was taken");
    closed(copied).await;

    // A copy an earlier program made holds no mark, and goes on above
    // what the original had given all the same.
    as_the_program_before_left_it(&copy);
    let connection = journal::copy::open(&copy).unwrap();
    connection.execute("DELETE FROM events WHERE seq > 2", []).unwrap();
    journal::copy::seal(connection).unwrap();
    let copied = opened(&copy).await;
    assert_eq!(say(&copied).await, 4, "the third was given, and is not given again");
}
