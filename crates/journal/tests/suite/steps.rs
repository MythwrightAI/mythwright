//! The steps a database's tables are made by only grow at the end
//! (`journal::steps`).
//!
//! A database runs the steps after those it has run. The fault played
//! here is the one a factory would not start for: a list its database
//! had run to the end, handed back with a step written in before the
//! last. The database ran the last a second time, which the database
//! itself refused where that step makes rules, and took without a word,
//! leaving the new step undone for good, where it makes something only
//! if it is not there. A database that keeps what its steps were is
//! refused either list before anything is run; one from before this was
//! kept has nothing to go by the first time, and is as it always was.

use journal::{Db, steps};
use rusqlite::Connection;

use crate::scratch;

const TABLE: &str = "CREATE TABLE things (id TEXT, project TEXT)";
const BESIDE: &str = "ALTER TABLE things ADD COLUMN beside TEXT";
/// A step that cannot be run twice: the database refuses the second.
const RULES: &str = "CREATE TRIGGER things_say_whose BEFORE INSERT ON things WHEN NEW.project IS NULL
     BEGIN SELECT RAISE (ABORT, 'a thing says whose it is'); END";
/// A step that can: run again it does nothing, and says nothing.
const FOUND_BY: &str = "CREATE INDEX IF NOT EXISTS things_by_project ON things (project)";

/// The lists as a database ran them, and as each was then handed back:
/// with a step written in before the last, and with it added last.
static RAN_RULES: &[&str] = &[TABLE, RULES];
static IN_BEFORE_RULES: &[&str] = &[TABLE, BESIDE, RULES];
static AFTER_RULES: &[&str] = &[TABLE, RULES, BESIDE];
static RAN_FOUND_BY: &[&str] = &[TABLE, FOUND_BY];
static IN_BEFORE_FOUND_BY: &[&str] = &[TABLE, BESIDE, FOUND_BY];
static AFTER_FOUND_BY: &[&str] = &[TABLE, FOUND_BY, BESIDE];

/// The columns of `things`, and what the database keeps of the list:
/// how many steps it ran and what each was.
async fn stands(db: &Db) -> (Vec<String>, i64, Option<String>) {
    db.call(|connection| {
        let columns = connection
            .prepare("SELECT name FROM pragma_table_info('things')")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        let (ran, were) = connection.query_row(
            "SELECT version, steps FROM schema_versions WHERE namespace = 'things'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((columns, ran, were))
    })
    .await
    .unwrap()
}

fn marks(list: &[&str]) -> Option<String> {
    Some(list.iter().map(|step| steps::mark(step)).collect::<Vec<_>>().join(" "))
}

#[tokio::test]
async fn a_list_with_a_step_written_in_before_the_last_is_refused_before_anything_is_run() {
    let lists =
        [(RAN_RULES, IN_BEFORE_RULES, AFTER_RULES), (RAN_FOUND_BY, IN_BEFORE_FOUND_BY, AFTER_FOUND_BY)];
    for (ran, written_in, added_last) in lists {
        let db = Db::memory().unwrap();
        db.migrate("things", ran).await.unwrap();
        let before = stands(&db).await;
        assert_eq!(before, (vec!["id".to_string(), "project".to_string()], 2, marks(ran)));

        // Whether or not its last step could be run a second time.
        let refused = db.migrate("things", written_in).await.unwrap_err();
        assert_eq!(refused.kind, kernel::ErrorKind::Conflict, "{}", refused.message);
        for says in [
            "the `things` tables in a database in memory were made by other steps than this program has",
            "of the 2 it has run, step 2 is not the one this program has in its place",
            "written in before the last",
            "Nothing was changed",
            "Run the program this database was last run by",
        ] {
            assert!(refused.message.contains(says), "{says}: {}", refused.message);
        }
        assert_eq!(stands(&db).await, before, "nothing of it was run, and nothing kept");

        // Added last, the step is run, once, and the database keeps it.
        db.migrate("things", added_last).await.unwrap();
        let columns = vec!["id".to_string(), "project".to_string(), "beside".to_string()];
        assert_eq!(stands(&db).await, (columns.clone(), 3, marks(added_last)));
        db.migrate("things", added_last).await.unwrap();
        assert_eq!(stands(&db).await, (columns, 3, marks(added_last)));
        // And the list it was refused is refused still, by its second step.
        let refused = db.migrate("things", written_in).await.unwrap_err();
        assert!(refused.message.contains("of the 3 it has run, step 2 is not"), "{}", refused.message);
    }
}

/// A database as a program before this left it, having run `ran`: how
/// many steps is all it keeps of them.
fn from_before(file: &std::path::Path, ran: &[&str]) {
    let connection = Connection::open(file).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE schema_versions (
                 namespace TEXT PRIMARY KEY,
                 version INTEGER NOT NULL
             );",
        )
        .unwrap();
    for step in ran {
        connection.execute_batch(step).unwrap();
    }
    connection
        .execute("INSERT INTO schema_versions (namespace, version) VALUES ('things', ?1)", [ran.len() as i64])
        .unwrap();
}

#[tokio::test]
async fn a_database_from_before_keeps_what_its_steps_were_from_the_first_time_it_is_opened() {
    let dir = scratch("steps-before");
    let file = dir.join("things.db");
    from_before(&file, RAN_FOUND_BY);
    let db = Db::open(&file).unwrap();
    // Opened, it has where to keep them and has kept nothing yet.
    let columns = vec!["id".to_string(), "project".to_string()];
    assert_eq!(stands(&db).await, (columns.clone(), 2, None));

    // It is taken to have run the steps this program has, and nothing is
    // run for it.
    db.migrate("things", RAN_FOUND_BY).await.unwrap();
    assert_eq!(stands(&db).await, (columns.clone(), 2, marks(RAN_FOUND_BY)));

    // Whoever opens it next, this program or one that keeps as much, goes
    // by that: the list that was taken without a word is refused.
    db.close();
    db.closed().await;
    let db = Db::open(&file).unwrap();
    let refused = db.migrate("things", IN_BEFORE_FOUND_BY).await.unwrap_err();
    assert!(
        refused.message.contains(&format!("the `things` tables in {}", file.display())),
        "{}",
        refused.message
    );
    assert!(refused.message.contains("step 2 is not the one this program has"), "{}", refused.message);
    assert_eq!(stands(&db).await, (columns, 2, marks(RAN_FOUND_BY)));
    db.migrate("things", AFTER_FOUND_BY).await.unwrap();
    assert_eq!(stands(&db).await.1, 3);
}

#[tokio::test]
async fn a_database_from_before_handed_a_wrong_list_the_first_time_has_nothing_to_go_by() {
    // What a step written in before the last did, and still does to a
    // database that kept only how many steps it ran: the last is run a
    // second time. Where the database refuses that, nothing is changed
    // and its program does not start.
    let dir = scratch("steps-unknown");
    let file = dir.join("rules.db");
    from_before(&file, RAN_RULES);
    let db = Db::open(&file).unwrap();
    let refused = db.migrate("things", IN_BEFORE_RULES).await.unwrap_err();
    assert!(refused.message.contains("trigger things_say_whose already exists"), "{}", refused.message);
    assert_eq!(stands(&db).await, (vec!["id".to_string(), "project".to_string()], 2, None));

    // Where it does not, the list is taken, and the step that was
    // written in is never run: the database says it has run three steps
    // and has no such column. It keeps what this program's steps are,
    // which it never ran, so the list put right is what is refused.
    let file = dir.join("found-by.db");
    from_before(&file, RAN_FOUND_BY);
    let db = Db::open(&file).unwrap();
    db.migrate("things", IN_BEFORE_FOUND_BY).await.unwrap();
    assert_eq!(
        stands(&db).await,
        (vec!["id".to_string(), "project".to_string()], 3, marks(IN_BEFORE_FOUND_BY))
    );
    let refused = db.migrate("things", AFTER_FOUND_BY).await.unwrap_err();
    assert!(refused.message.contains("of the 3 it has run, step 2 is not"), "{}", refused.message);
}

#[tokio::test]
async fn nothing_is_written_for_a_list_whose_every_step_was_run() {
    let dir = scratch("steps-rest");
    let file = dir.join("things.db");
    let db = Db::open(&file).unwrap();
    db.migrate("things", AFTER_RULES).await.unwrap();
    let wrote = db.writes();
    assert!(wrote > 0);
    db.migrate("things", AFTER_RULES).await.unwrap();
    assert_eq!(db.writes(), wrote, "handed the list again");
    db.close();
    db.closed().await;

    // Nor by whoever opens it next, the opening included.
    let db = Db::open(&file).unwrap();
    db.migrate("things", AFTER_RULES).await.unwrap();
    assert_eq!(db.writes(), 0, "opened again and handed the list");
    let changes: i64 = db.call(|connection| Ok(connection.total_changes() as i64)).await.unwrap();
    assert_eq!(changes, 0, "nothing was written as it was opened");
}

#[tokio::test]
async fn a_count_put_back_by_hand_has_the_steps_after_it_run_again() {
    // How a test makes a database as an earlier program left it: what
    // the later steps made is undone, and the count put back. What the
    // database kept of those steps is then of steps it has not run.
    static WAS: &[&str] = &[TABLE, BESIDE];
    static IS: &[&str] = &[TABLE, "ALTER TABLE things ADD COLUMN beside INTEGER"];
    let db = Db::memory().unwrap();
    db.migrate("things", WAS).await.unwrap();
    db.call(|connection| {
        connection.execute_batch(
            "ALTER TABLE things DROP COLUMN beside;
             UPDATE schema_versions SET version = version - 1 WHERE namespace = 'things';",
        )
    })
    .await
    .unwrap();
    db.migrate("things", IS).await.unwrap();
    let columns = vec!["id".to_string(), "project".to_string(), "beside".to_string()];
    assert_eq!(stands(&db).await, (columns, 2, marks(IS)));
}

#[test]
fn a_step_is_the_one_a_database_ran_where_it_is_the_same_text_or_was_changed_to_it_and_meant_to_be() {
    // The text a database is handed, whole: a comment is part of it.
    assert_eq!(steps::mark(TABLE), steps::mark(TABLE));
    assert_eq!(steps::mark(TABLE).len(), 16);
    assert_ne!(steps::mark(TABLE), steps::mark(&format!("{TABLE} -- of things")));
    assert_ne!(steps::mark(TABLE), steps::mark(&format!(" {TABLE}")));

    let has: Vec<String> = ["a2", "b2", "c1", "d1"].iter().map(|mark| mark.to_string()).collect();
    let changed = "# what was changed, and why\n\
                   2026-11-19\tthings\t1\ta1\ta2\ta word of its comment\n\
                   2026-11-19\tthings\t2\tb0\tb1\tonce\n\
                   2026-11-20\tthings\t2\tb1\tb2\tand again\n\
                   2026-11-20\tothers\t3\tc0\tc1\tanother list's third\n\
                   2026-11-20\tthings\t4\td1\td0\tthere and\n\
                   2026-11-21\tthings\t4\td0\td1\tback\n";
    // (what the database ran, the first step that is not the list's)
    let cases: &[(&[&str], Option<usize>)] = &[
        // The steps the list has, as many as were run or fewer.
        (&["a2", "b2", "c1", "d1"], None),
        (&["a2", "b2"], None),
        (&[], None),
        // One changed where it stood and written down as meant, once or
        // more than once.
        (&["a1", "b2"], None),
        (&["a2", "b0", "c1"], None),
        (&["a1", "b1", "c1", "d0"], None),
        // What was changed is of its own list and its own step.
        (&["a2", "b2", "c0"], Some(3)),
        (&["b0", "b2"], Some(1)),
        // And is read one way: a step is not what it was changed from.
        (&["a2", "b2", "c1", "d1"], None),
        (&["zz", "b2"], Some(1)),
        // A step written in before the end: every step from there on is
        // another, and the first is said.
        (&["a2", "c1", "d1"], Some(2)),
        // Two that changed places.
        (&["a2", "c1", "b2", "d1"], Some(2)),
    ];
    for (ran, parts) in cases {
        assert_eq!(steps::parted("things", ran, &has, changed), *parts, "{ran:?}");
    }
    // With nothing written down, a step changed is another step. One
    // changed there and back, and then to something else, was each of
    // those; and it is not what it was never changed to.
    assert_eq!(steps::parted("things", &["a1"], &has, ""), Some(1));
    let round = "d\tthings\t1\tx\ty\tthere\nd\tthings\t1\ty\tx\tback\nd\tthings\t1\tx\ta2\tand on\n";
    for was in ["x", "y", "a2"] {
        assert_eq!(steps::parted("things", &[was], &has, round), None, "{was}");
    }
    let has = vec!["z".to_string()];
    for was in ["x", "y", "a2"] {
        assert_eq!(steps::parted("things", &[was], &has, round), Some(1), "{was}");
    }
}
