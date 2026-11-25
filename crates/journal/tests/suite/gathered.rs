//! Several pieces of work kept by one commit: each still whole or not at
//! all, nobody answered or told before the commit, a place the journal
//! was told still where it was told, and what was answered there in a
//! copy of the files taken at any moment.
//!
//! How work is gathered is the program's to say (`JOURNAL_GATHERS`): as
//! a daemon does it, only where many are waiting; whatever is waiting,
//! however little (`all`); or never (`1`). What is held here of each
//! piece of work is held each way.
//!
//! Which pieces a commit kept is the machine's to say and no test's to
//! arrange. A commit goes on taking in what is waiting for two
//! thousandths of a second at most, by the machine's own clock, and a
//! thread the machine kept from running for longer than that in the
//! middle of one makes the commit with what it has: twenty pieces asked
//! together were kept as fourteen and then six in a gate held to four
//! processors, and a commit was cut so in seven playings of eight
//! hundred, sixteen at once on one processor. So nothing here holds
//! that one commit kept everything asked together.
//! Each piece says as it is done which commit it is in ([`Commits`]); a
//! test holds what it means of each commit that came about, and that
//! they came about as work is gathered: one begun with enough waiting
//! behind it took in all of it, or had been open as long as a commit
//! may be.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use journal::checkpoint::Kept;
use journal::{Db, Journal, MetricQuery};
use kernel::{ManualClock, Source};
use serde_json::json;
use tokio::sync::oneshot;

use crate::scratch;

/// How long a commit goes on taking in what is waiting, at most, and how
/// many pieces of work are waiting when a daemon's thread is taken to
/// have fallen behind: as `crates/journal/src/gather.rs` says them.
const FOR: Duration = Duration::from_millis(2);
const DEEP: usize = 16;

/// How long a test waits for what is on its way before it says so.
const PATIENCE: Duration = Duration::from_secs(40);

/// What this program was told of gathering, if anything.
fn said() -> Option<String> {
    std::env::var("JOURNAL_GATHERS").ok()
}

/// Whether this program gathers work at all.
pub(crate) fn gathers() -> bool {
    said().as_deref() != Some("1")
}

/// Asks as many small things of the database as a daemon takes for its
/// thread having fallen behind, so that what is asked after them shares
/// a commit as it would there. They only read, and are asked first.
async fn crowd(db: &Db) {
    for _ in 0..16 {
        let db = db.clone();
        tokio::spawn(async move { db.call(|_| Ok(())).await });
    }
    for _ in 0..40 {
        tokio::task::yield_now().await;
    }
}

/// What the journal has measured of work that shared its commit: how
/// many pieces did, and what each said of how many its commit kept,
/// added up.
async fn shared(journal: &Journal) -> (u64, f64) {
    let series = journal
        .query_metrics(MetricQuery {
            name: "db.shared_count".into(),
            since_ms: journal.now_ms() - 60_000,
            ..Default::default()
        })
        .await
        .unwrap();
    series.first().map(|all| (all.total.count, all.total.sum)).unwrap_or_default()
}

/// Holds the database's thread, as work that is slow would, until what
/// it gives back is put away: what is asked meanwhile waits together.
async fn held(db: &Db) -> mpsc::Sender<()> {
    let (entered, enters) = oneshot::channel();
    let (let_go, go) = mpsc::channel::<()>();
    drop(db.call_apart(move |_| {
        let _ = entered.send(());
        let _ = go.recv();
        Ok(())
    }));
    enters.await.unwrap();
    let_go
}

/// Completes once `many` pieces of work are waiting for the database's
/// thread: for something asked that reaches the database only when it
/// is waited for, so that what is asked next is asked behind it.
async fn waiting(db: &Db, many: usize) {
    tokio::time::timeout(PATIENCE, async {
        while db.waiting() < many {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{} of {many} pieces of work came to wait: {}", db.waiting(), db.standing()));
}

/// What a piece of work asked for comes to, for whoever waits for it.
type Came = Pin<Box<dyn Future<Output = kernel::Result<u64>>>>;

/// Something that was asked of the database as `work` was made, whoever
/// waits for it and whenever: it has been waiting its turn since then.
fn came_to<T: 'static>(work: impl Future<Output = kernel::Result<T>> + 'static, came: fn(T) -> u64) -> Came {
    Box::pin(async move { work.await.map(came) })
}

/// What a piece of work says as it is done: which piece it is, how many
/// the database had done by then, and when that was.
struct Seen {
    piece: usize,
    done: u64,
    began: Instant,
}

/// Which commit kept each piece of work a test asked together, said by
/// the pieces themselves as they are done on the database's thread.
#[derive(Clone)]
struct Commits {
    db: Db,
    seen: Arc<Mutex<Vec<Seen>>>,
}

impl Commits {
    fn of(db: &Db) -> Commits {
        Commits { db: db.clone(), seen: Arc::default() }
    }

    /// Said by the piece of work that is the `piece`th asked, as it is
    /// done. A piece of work is counted as done once the commit it is
    /// in is made, so two pieces find the same count where one commit
    /// keeps both, and nowhere else.
    fn doing(&self, piece: usize) {
        let seen = Seen { piece, done: self.db.done(), began: Instant::now() };
        self.seen.lock().unwrap().push(seen);
    }

    /// The pieces each commit kept, in the order they were made.
    fn kept(&self) -> Vec<Vec<usize>> {
        let seen = self.seen.lock().unwrap();
        let mut kept: Vec<(u64, Vec<usize>)> = Vec::new();
        for one in seen.iter() {
            match kept.last_mut() {
                Some((done, pieces)) if *done == one.done => pieces.push(one.piece),
                _ => kept.push((one.done, vec![one.piece])),
            }
        }
        kept.into_iter().map(|(_, pieces)| pieces).collect()
    }

    /// What the journal is to have measured of these commits beyond
    /// `before`, which it had measured when they were asked for: each
    /// piece that shared its commit says how many the commit kept.
    fn shared_since(&self, before: (u64, f64)) -> (u64, f64) {
        let kept = self.kept();
        let shared = kept.iter().map(Vec::len).filter(|pieces| *pieces > 1);
        let (pieces, said) =
            (shared.clone().sum::<usize>(), shared.map(|pieces| pieces * pieces).sum::<usize>());
        (before.0 + pieces as u64, before.1 + said as f64)
    }

    /// Holds the commits that came about to how work is gathered, of
    /// `many` pieces that were all waiting, and nothing else, when the
    /// thread was let go at `let_go`. They are done in the order they
    /// were asked. A commit begun with too little waiting behind it for
    /// the thread to be behind keeps its one piece. One begun behind
    /// takes in all that waits, and where it left any it had been open
    /// as long as a commit may be: no longer ago than whatever was done
    /// before it was begun, and its commit made before what was done
    /// next began, which is all a test can see of when it was open and
    /// is never less than it was.
    fn as_gathered(&self, many: usize, let_go: Instant) {
        let deep = match said().as_deref() {
            Some("1") => usize::MAX,
            Some("all") => 1,
            _ => DEEP,
        };
        let seen = self.seen.lock().unwrap();
        let order: Vec<usize> = seen.iter().map(|one| one.piece).collect();
        assert!(order.is_sorted(), "work was done in another order than it was asked in: {order:?}");
        let mut from = 0;
        while from < seen.len() {
            let to = from + seen[from..].iter().take_while(|one| one.done == seen[from].done).count();
            let (behind, kept) = (many - 1 - seen[from].piece, &order[from..to]);
            if behind < deep {
                assert_eq!(kept.len(), 1, "a commit begun with {behind} waiting behind it kept {kept:?}");
            } else if let Some(next) = seen.get(to) {
                let since = if from == 0 { let_go } else { seen[from - 1].began };
                let open = next.began.duration_since(since);
                assert!(
                    open >= FOR,
                    "a commit begun with {behind} waiting behind it kept {kept:?} and left the rest, having \
                     been open for {open:?} at most"
                );
            }
            from = to;
        }
    }
}

async fn on_disk(dir: &Path, kept: Kept) -> (Journal, Db) {
    let db = Db::open_keeping(&dir.join("journal.db"), kept).unwrap();
    let journal = Journal::open(db.clone(), Arc::new(ManualClock::new()), "here").await.unwrap();
    db.call(|connection| {
        connection
            .execute_batch("CREATE TABLE things (name TEXT PRIMARY KEY, said TEXT NOT NULL DEFAULT '');")
    })
    .await
    .unwrap();
    (journal, db)
}

async fn names(db: &Db) -> Vec<String> {
    db.call(|connection| {
        let mut read = connection.prepare("SELECT name FROM things ORDER BY name")?;
        read.query_map([], |row| row.get(0))?.collect()
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn work_that_waits_together_is_kept_by_one_commit_and_each_piece_is_still_whole_or_not_at_all() {
    let dir = scratch("gathered");
    let (journal, db) = on_disk(&dir, Kept { ask: 1_000_000, bound: 1_000_000 }).await;
    let (pages, measured, commits) = (db.log_pages(), shared(&journal).await, Commits::of(&db));

    // Twenty pieces of work asked while the thread is busy: a row each,
    // one of them in a whole of its own that fails, and one that says
    // something.
    let go = held(&db).await;
    let mut asked: Vec<Came> = Vec::new();
    for n in 0..20 {
        let (name, commits) = (format!("thing-{n:02}"), commits.clone());
        asked.push(match n {
            7 => came_to(
                db.call(move |connection| {
                    commits.doing(n);
                    let whole = journal::whole(connection)?;
                    whole.execute("INSERT INTO things (name) VALUES (?1)", [&name])?;
                    // The same again is refused, and the whole goes with it.
                    whole.execute("INSERT INTO things (name) VALUES (?1)", [&name])?;
                    whole.commit()
                }),
                |()| 0,
            ),
            11 => came_to(
                journal.write(move |writing| {
                    commits.doing(n);
                    writing.transaction().execute("INSERT INTO things (name) VALUES (?1)", [&name])?;
                    writing.say("thing.made", Source::default(), json!({ "name": name }))
                }),
                |(seq, _)| seq,
            ),
            _ => came_to(
                db.call(move |connection| {
                    commits.doing(n);
                    connection.execute("INSERT INTO things (name) VALUES (?1)", [&name])
                }),
                |_| 0,
            ),
        });
    }
    // All of it is waiting before any of it is done, and nothing else is.
    assert_eq!(db.waiting(), 20);
    let let_go = Instant::now();
    drop(go);
    let mut came = Vec::new();
    for one in asked {
        came.push(one.await);
    }

    // The one that failed took back what it wrote and nobody else's.
    assert!(came[7].is_err(), "{:?}", came[7]);
    let kept = names(&db).await;
    assert_eq!(kept.len(), 19, "{kept:?}");
    assert!(!kept.contains(&"thing-07".to_string()));
    // What was said was numbered as it was written, and is the last there is.
    assert_eq!(came[11].as_ref().unwrap(), &1);
    assert_eq!(journal.last_seq(), 1);

    // One commit kept the twenty, where the program gathers work and
    // the machine let the commit be made in the time one may be open;
    // where it did not, each commit that left work waiting had been
    // open that long. Told to gather nothing, a commit each.
    commits.as_gathered(20, let_go);
    let made = commits.kept();
    if made.len() == 1 {
        // Kept by one commit, the page they are all on was written once:
        // a commit each is a page or more each.
        let pages = db.log_pages() - pages;
        assert!(pages < 12, "nineteen rows and an event took {pages} pages of the log");
    }
    // And the journal says of each piece how many its commit kept.
    assert_eq!(shared(&journal).await, commits.shared_since(measured), "{made:?}");
}

#[tokio::test]
async fn a_thread_that_is_keeping_up_commits_each_piece_of_work_by_itself() {
    let dir = scratch("alone");
    let (journal, db) = on_disk(&dir, Kept::default()).await;
    let (measured, commits) = (shared(&journal).await, Commits::of(&db));
    // A few things asked while the thread is busy: not so many that it has
    // fallen behind, so each is kept and answered as it is done.
    let go = held(&db).await;
    let mut asked = Vec::new();
    for n in 0..5 {
        let commits = commits.clone();
        asked.push(db.call(move |connection| {
            commits.doing(n);
            connection.execute("INSERT INTO things (name) VALUES (?1)", [format!("thing-{n}")])
        }));
    }
    assert_eq!(db.waiting(), 5);
    let let_go = Instant::now();
    drop(go);
    for one in asked {
        one.await.unwrap();
    }
    assert_eq!(names(&db).await.len(), 5);
    // Told to share whatever waits, it shares these, as far as the time
    // a commit may be open lets it.
    commits.as_gathered(5, let_go);
    let made = commits.kept();
    if said().as_deref() != Some("all") {
        assert_eq!(made.len(), 5, "five waiting is not being behind: {made:?}");
    }
    assert_eq!(shared(&journal).await, commits.shared_since(measured), "{made:?}");
}

#[tokio::test]
async fn events_written_in_one_commit_are_numbered_in_order_and_heard_once_it_is_made_and_one_that_fails_gives_no_number()
 {
    let dir = scratch("heard");
    let (journal, db) = on_disk(&dir, Kept::default()).await;
    let mut hears = journal.live();

    let go = held(&db).await;
    crowd(&db).await;
    let mut asked = Vec::new();
    for n in 0..6 {
        let journal = journal.clone();
        asked.push(tokio::spawn(async move {
            match n {
                // Says something and then fails: nothing of it is kept.
                3 => journal
                    .write(|writing| {
                        writing.say("thing.failed", Source::default(), json!({}))?;
                        writing.transaction().execute("INSERT INTO nothing_there VALUES (1)", [])?;
                        Ok(())
                    })
                    .await
                    .map(|_| 0),
                _ => journal
                    .emit("thing.made", Source::default(), json!({ "n": n }))
                    .await
                    .map(|event| event.seq),
            }
        }));
    }
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    // Nothing is heard while nothing is kept.
    assert!(hears.try_recv().is_err());
    assert_eq!(journal.last_seq(), 0);
    drop(go);
    let mut seqs = Vec::new();
    for one in asked {
        if let Ok(seq) = one.await.unwrap() {
            seqs.push(seq);
        }
    }
    seqs.sort_unstable();
    assert_eq!(seqs, [1, 2, 3, 4, 5], "no number given to what was not kept, and none twice");
    assert_eq!(journal.last_seq(), 5);
    let mut heard = Vec::new();
    while let Ok(event) = hears.try_recv() {
        heard.push((event.seq, event.topic.clone()));
    }
    assert_eq!(heard.iter().map(|(seq, _)| *seq).collect::<Vec<_>>(), [1, 2, 3, 4, 5]);
    assert!(heard.iter().all(|(_, topic)| topic == "thing.made"));
}

#[tokio::test]
async fn a_commit_that_cannot_be_made_keeps_nothing_of_anybody_in_it_and_tells_each_and_gives_no_number() {
    let dir = scratch("lost");
    let (journal, db) = on_disk(&dir, Kept::default()).await;
    // Nothing the factory keeps is checked only as a commit is made; this
    // is, so that a commit can be made to fail: a part that names a whole
    // which is not there is taken as it is written and refused at the end.
    db.call(|connection| {
        connection.execute_batch(
            "CREATE TABLE wholes (name TEXT PRIMARY KEY);
             CREATE TABLE parts (
                 name TEXT PRIMARY KEY,
                 of TEXT NOT NULL REFERENCES wholes (name) DEFERRABLE INITIALLY DEFERRED
             );",
        )
    })
    .await
    .unwrap();
    let (mut hears, commits) = (journal.live(), Commits::of(&db));

    // Asked together: rows, something said, and one part of nothing.
    let go = held(&db).await;
    let mut asked: Vec<Came> = Vec::new();
    for n in 0..20 {
        let commits = commits.clone();
        asked.push(match n {
            5 => came_to(
                db.call(move |connection| {
                    commits.doing(n);
                    connection.execute("INSERT INTO parts VALUES ('part', 'no whole')", [])
                }),
                |_| 0,
            ),
            9 => {
                // Said the plain way, which asks nothing of the database
                // until it is waited for: so it is waited for here, and
                // is asked in its turn.
                let journal = journal.clone();
                let said = tokio::spawn(async move {
                    journal.emit("thing.made", Source::default(), json!({})).await.map(|event| event.seq)
                });
                waiting(&db, n + 1).await;
                Box::pin(async move { said.await.unwrap() })
            }
            _ => came_to(
                db.call(move |connection| {
                    commits.doing(n);
                    connection.execute("INSERT INTO things (name) VALUES (?1)", [format!("thing-{n:02}")])
                }),
                |_| 0,
            ),
        });
    }
    // All of it is waiting before any of it is done, and nothing else is.
    assert_eq!(db.waiting(), 20);
    let let_go = Instant::now();
    drop(go);
    let mut came = Vec::new();
    for one in asked {
        came.push(one.await);
    }
    commits.as_gathered(20, let_go);

    // The commit the part was in cannot be made, whichever pieces it
    // came to keep: all twenty, where the program gathers work and the
    // machine let the commit be made in the time one may be open; the
    // part alone, where the program gathers nothing. Every piece in it
    // is told that nothing of it was kept, and nothing of it is; every
    // piece in another commit is kept, as if this one had never been.
    let made = commits.kept();
    let lost = made.iter().find(|kept| kept.contains(&5)).expect("the part was never written");
    let parts: i64 = db
        .call(|connection| connection.query_row("SELECT count(*) FROM parts", [], |row| row.get(0)))
        .await
        .unwrap();
    assert_eq!(parts, 0);
    let kept = names(&db).await;
    for n in (0..20).filter(|n| *n != 9) {
        let name = format!("thing-{n:02}");
        match (&came[n], lost.contains(&n)) {
            (Err(why), true) => {
                assert!(why.to_string().contains("could not be kept"), "{why}");
                assert!(!kept.contains(&name), "{name} is kept, and was in the commit that failed: {lost:?}");
            }
            (Ok(_), false) => {
                assert!(
                    kept.contains(&name),
                    "{name} is not kept, and was in a commit that was made: {made:?}"
                )
            }
            (told, _) => panic!("the commit that failed kept {lost:?}, and piece {n} was told {told:?}"),
        }
    }

    // What was said says nothing of which commit it is in. It was asked
    // between two pieces that do: it is in the commit of both, or, where
    // one commit ended between them, of one of the two.
    let (said, before, after) = (&came[9], lost.contains(&8), lost.contains(&10));
    assert!(said.is_err() || !(before && after), "said in the commit that failed, and told {said:?}");
    assert!(said.is_ok() || before || after, "said in a commit that was made, and told {said:?}");
    match said {
        // Not kept: it was never the journal's, and nobody heard it.
        Err(why) => {
            assert!(why.to_string().contains("could not be kept"), "{why}");
            assert_eq!(journal.last_seq(), 0);
            assert!(hears.try_recv().is_err(), "an event that was not kept was heard");
        }
        // Kept: the journal's first, heard once.
        Ok(seq) => {
            assert_eq!((*seq, journal.last_seq()), (1, 1));
            assert_eq!(hears.try_recv().ok().map(|event| event.seq), Some(1));
            assert!(hears.try_recv().is_err());
        }
    }
    // And the next thing asked is done as if nothing had been: a number
    // that was not kept is given again, to what is.
    let next = journal.emit("thing.made", Source::default(), json!({})).await.unwrap();
    assert_eq!(next.seq, journal.last_seq());
    assert_eq!(next.seq, if said.is_err() { 1 } else { 2 });
    db.call(|connection| connection.execute("INSERT INTO things (name) VALUES ('after')", [])).await.unwrap();
    assert!(names(&db).await.contains(&"after".to_string()));
}

/// How many events a database file holds, read by somebody else: the
/// journal's own thread may be held.
fn events_in(dir: &Path) -> Vec<String> {
    let reader = rusqlite::Connection::open(dir.join("journal.db")).unwrap();
    let mut read = reader.prepare("SELECT topic FROM events ORDER BY seq").unwrap();
    read.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap()
}

#[tokio::test]
async fn a_place_the_journal_was_told_ends_the_commit_it_falls_in_and_nothing_asked_after_it_is_kept() {
    let dir = scratch("place");
    let (journal, db) = on_disk(&dir, Kept::default()).await;
    journal.shut_after(|event| event.topic == "the.place");

    // Asked together: two before the place, the place, and two after it,
    // one of which says nothing.
    let go = held(&db).await;
    crowd(&db).await;
    let said = |topic: &'static str| {
        let journal = journal.clone();
        tokio::spawn(
            async move { journal.emit(topic, Source::default(), json!({})).await.map(|event| event.seq) },
        )
    };
    let before = [said("one"), said("two")];
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    let place = said("the.place");
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    let after = said("three");
    let written = tokio::spawn({
        let db = db.clone();
        async move {
            db.call(|connection| connection.execute("INSERT INTO things (name) VALUES ('late')", [])).await
        }
    });
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    drop(go);

    for one in before {
        assert!(one.await.unwrap().is_ok());
    }
    assert_eq!(place.await.unwrap().unwrap(), 3);
    journal.shut().await;
    // What was asked after the place is never answered, and is not there.
    for _ in 0..200 {
        tokio::task::yield_now().await;
    }
    assert!(!after.is_finished() && !written.is_finished());
    assert_eq!(events_in(&dir), ["one", "two", "the.place"]);
    let reader = rusqlite::Connection::open(dir.join("journal.db")).unwrap();
    let late: i64 = reader.query_row("SELECT count(*) FROM things", [], |row| row.get(0)).unwrap();
    assert_eq!(late, 0, "what was written after the place was kept");
    assert_eq!(journal.last_seq(), 3);
}

#[tokio::test]
async fn a_place_told_with_the_database_left_open_keeps_what_is_written_after_it_and_nothing_that_is_said() {
    let dir = scratch("open");
    let (journal, db) = on_disk(&dir, Kept::default()).await;
    journal.shut_after_saying(|event| event.topic == "the.place");

    let go = held(&db).await;
    crowd(&db).await;
    let place = tokio::spawn({
        let journal = journal.clone();
        async move { journal.emit("the.place", Source::default(), json!({})).await.map(|event| event.seq) }
    });
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    // After it, in what would have been the same commit: something written
    // with nothing said, and something written and said.
    let quiet = tokio::spawn({
        let db = db.clone();
        async move {
            db.call(|connection| connection.execute("INSERT INTO things (name) VALUES ('quiet')", [])).await
        }
    });
    let loud = tokio::spawn({
        let journal = journal.clone();
        async move {
            journal
                .write(|writing| {
                    writing.transaction().execute("INSERT INTO things (name) VALUES ('loud')", [])?;
                    writing.say("too.late", Source::default(), json!({}))
                })
                .await
        }
    });
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }
    drop(go);
    assert_eq!(place.await.unwrap().unwrap(), 1);
    assert_eq!(quiet.await.unwrap().unwrap(), 1);
    for _ in 0..200 {
        tokio::task::yield_now().await;
    }
    assert!(!loud.is_finished(), "what is said after the place is never answered");
    assert_eq!(names(&db).await, ["quiet"]);
    assert_eq!(events_in(&dir), ["the.place"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn what_was_answered_is_in_a_copy_of_the_files_taken_at_any_moment_among_many_writers() {
    let dir = scratch("answered");
    // The log is never brought back while this is played, so that it only
    // grows: a copy of the file and then of its log is then what a daemon
    // killed at that moment would have left, torn at its end or not.
    let (journal, db) = on_disk(&dir, Kept { ask: i64::MAX / 4, bound: i64::MAX / 4 }).await;
    let answered: Arc<Mutex<Vec<String>>> = Arc::default();
    let mut writers = Vec::new();
    for who in 0..8 {
        let (journal, db, answered) = (journal.clone(), db.clone(), answered.clone());
        writers.push(tokio::spawn(async move {
            for n in 0..150 {
                let name = format!("w{who}-{n:03}");
                let written = name.clone();
                // By turns a row alone, and a row with something said of it.
                match n % 2 {
                    0 => {
                        db.call(move |connection| {
                            connection.execute("INSERT INTO things (name) VALUES (?1)", [&written])
                        })
                        .await
                        .unwrap();
                    }
                    _ => {
                        journal
                            .write(move |writing| {
                                writing
                                    .transaction()
                                    .execute("INSERT INTO things (name) VALUES (?1)", [&written])?;
                                writing.say("thing.made", Source::default(), json!({ "name": written }))
                            })
                            .await
                            .unwrap();
                    }
                }
                answered.lock().unwrap().push(name);
            }
        }));
    }
    // Three copies, each at whatever moment it falls.
    for copy in 0..3 {
        tokio::time::timeout(Duration::from_secs(40), async {
            while answered.lock().unwrap().len() < 200 * (copy + 1) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the writers to have got on");
        // What had been answered, read before the copy is begun.
        let were = answered.lock().unwrap().clone();
        let to = dir.join(format!("copy-{copy}"));
        std::fs::create_dir_all(&to).unwrap();
        std::fs::copy(dir.join("journal.db"), to.join("journal.db")).unwrap();
        std::fs::copy(dir.join("journal.db-wal"), to.join("journal.db-wal")).unwrap();
        let reader = rusqlite::Connection::open(to.join("journal.db")).unwrap();
        let sound: String = reader.query_row("PRAGMA quick_check", [], |row| row.get(0)).unwrap();
        assert_eq!(sound, "ok");
        let mut read = reader.prepare("SELECT name FROM things").unwrap();
        let there: std::collections::BTreeSet<String> =
            read.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        let missing: Vec<&String> = were.iter().filter(|name| !there.contains(*name)).collect();
        assert!(missing.is_empty(), "answered and not in the copy: {missing:?}");
        // And what was said of a row is there if and only if the row is.
        let mut said = reader.prepare("SELECT data FROM events").unwrap();
        let events: Vec<String> =
            said.query_map([], |row| row.get(0)).unwrap().collect::<rusqlite::Result<_>>().unwrap();
        let spoken =
            there.iter().filter(|name| name[name.len() - 3..].parse::<u32>().unwrap() % 2 == 1).count();
        assert_eq!(events.len(), spoken, "a row and what is said of it are kept together");
    }
    for writer in writers {
        writer.await.unwrap();
    }
}
