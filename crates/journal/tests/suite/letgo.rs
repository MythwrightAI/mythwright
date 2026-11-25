//! Letting the detail of runs' steps go by its age (`journal::letgo`):
//! the events, by topic, under what is owed and what is under way; and
//! the files, wherever they were written.

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use journal::letgo::{Went, writable};
use journal::{EventQuery, Logs};
use kernel::Source;
use serde_json::json;

use crate::{journal, scratch};

const DAY: Duration = Duration::from_secs(24 * 3600);

fn of_run(run: &str) -> Source {
    Source { run: Some(run.into()), ..Source::default() }
}

async fn left(journal: &journal::Journal) -> Vec<(String, Option<String>)> {
    let all = journal.read(EventQuery::default()).await.unwrap();
    all.iter().map(|event| (event.topic.clone(), event.source.run.clone())).collect()
}

#[tokio::test]
async fn what_runs_said_of_their_steps_goes_by_its_age_and_nothing_else_of_the_journal_does() {
    let (journal, clock) = journal().await;
    for run in ["old", "going"] {
        for topic in ["run.started", "step.started", "step.finished", "run.finished"] {
            journal.emit(topic, of_run(run), json!({})).await.unwrap();
        }
    }
    journal.emit("board.item.moved", Source::default(), json!({})).await.unwrap();
    clock.advance(2 * DAY);
    journal.emit("step.started", of_run("new"), json!({})).await.unwrap();

    // Of what is older than a day, the topics asked for; but nothing of
    // a run that is under way, which reads back what it said.
    let topics = ["run.started", "step.started", "step.finished"];
    let before = journal.now_ms() - DAY.as_millis() as i64;
    let gone = journal.let_go_of(&topics, before, None, &["going".to_string()]).await.unwrap();
    assert_eq!(gone, 3);
    let run = |name: &str| Some(name.to_string());
    assert_eq!(
        left(&journal).await,
        [
            ("run.finished".to_string(), run("old")),
            ("run.started".to_string(), run("going")),
            ("step.started".to_string(), run("going")),
            ("step.finished".to_string(), run("going")),
            ("run.finished".to_string(), run("going")),
            ("board.item.moved".to_string(), None),
            ("step.started".to_string(), run("new")),
        ]
    );
    // Done again, it finds nothing more.
    assert_eq!(journal.let_go_of(&topics, before, None, &["going".to_string()]).await.unwrap(), 0);
}

#[tokio::test]
async fn nothing_above_the_place_something_owed_it_keeps_is_let_go() {
    let (journal, clock) = journal().await;
    for _ in 0..6 {
        journal.emit("step.started", of_run("done"), json!({})).await.unwrap();
    }
    clock.advance(2 * DAY);
    let before = journal.now_ms() - DAY.as_millis() as i64;
    // Something has dealt with the first four and is owed the rest.
    assert_eq!(journal.let_go_of(&["step.started"], before, Some(4), &[]).await.unwrap(), 4);
    let seqs: Vec<u64> = journal.read(EventQuery::default()).await.unwrap().iter().map(|e| e.seq).collect();
    assert_eq!(seqs, [5, 6]);
    // A place of nought holds everything.
    assert_eq!(journal.let_go_of(&["step.started"], before, Some(0), &[]).await.unwrap(), 0);
}

#[tokio::test]
async fn once_events_were_let_go_from_its_middle_the_newest_so_many_are_counted_from_what_is_left() {
    let (journal, clock) = journal().await;
    for _ in 0..3 {
        journal.emit("kept", Source::default(), json!({})).await.unwrap();
    }
    for _ in 0..20 {
        journal.emit("step.finished", of_run("done"), json!({})).await.unwrap();
    }
    clock.advance(2 * DAY);
    journal.emit("kept", Source::default(), json!({})).await.unwrap();
    let before = journal.now_ms() - DAY.as_millis() as i64;
    assert_eq!(journal.let_go_of(&["step.finished"], before, None, &[]).await.unwrap(), 20);
    // Four are left, and of everything old the newest three are kept:
    // counted over the numbers, the twenty that went would have been
    // taken for the newest, and all but the last would have gone.
    let removed = journal.prune_events(journal.now_ms() + 1, 3).await.unwrap();
    assert_eq!(removed, 1);
    let seqs: Vec<u64> = journal.read(EventQuery::default()).await.unwrap().iter().map(|e| e.seq).collect();
    assert_eq!(seqs, [2, 3, 24]);
}

#[tokio::test]
async fn how_long_is_said_and_kept_and_how_far_it_went_says_which_runs_were_let_go() {
    let (journal, _) = journal().await;
    let logs = Logs::new(scratch("went").join("logs"));
    assert_eq!(journal.logs_kept().await.days, None, "unsaid, everything is kept for ever");
    for refused in [0, journal::letgo::MOST_DAYS + 1] {
        let said = journal.keep_logs_for(Some(refused), "user:ada").await.unwrap_err().message;
        assert!(said.contains("a day at least"), "{said}");
    }
    let (was, now) = journal.keep_logs_for(Some(7), "user:ada").await.unwrap();
    assert_eq!((was.days, now.days, now.by.as_deref()), (None, Some(7), Some("user:ada")));
    journal.went(&logs, Went { before: 1_000, days: 7 }).await.unwrap();
    assert_eq!(logs.went(), Some(Went { before: 1_000, days: 7 }));
    let kept = journal.logs_kept().await;
    assert_eq!(kept.days, Some(7));
    let said = kept.of_run(Some(999)).unwrap();
    assert!(said.contains("let go after 7 days"), "{said}");
    assert_eq!(kept.of_run(Some(1_000)), None, "a run that ended since");
    assert_eq!(kept.of_run(None), None, "a run under way");
    // Said again, how far it went stays.
    journal.keep_logs_for(None, "user:ada").await.unwrap();
    assert!(journal.logs_kept().await.of_run(Some(1)).is_some());
}

fn line(logs: &Logs, run: &str, said: &str) {
    logs.append("runs", &format!("{run}.greet"), 1, 'o', said);
}

fn read(logs: &Logs, run: &str) -> Vec<String> {
    logs.read("runs", &format!("{run}.greet"), 0, 10).into_iter().map(|line| line.text).collect()
}

#[tokio::test]
async fn a_folder_said_for_logs_is_written_in_from_then_on_and_what_was_written_before_is_read_where_it_is() {
    let dir = scratch("folders");
    let (journal, _) = journal().await;
    let logs = Logs::new(dir.join("data").join("logs"));
    line(&logs, "run_a", "in the data folder");
    let elsewhere = dir.join("elsewhere");
    let (_, now) =
        journal.write_logs_in(&logs, Some(&elsewhere.to_string_lossy()), "user:ada").await.unwrap();
    assert!(elsewhere.join("runs").is_dir(), "it is made where it is not there");
    line(&logs, "run_b", "elsewhere");
    assert!(elsewhere.join("runs").join("run_b.greet.log").is_file());
    assert_eq!(
        (read(&logs, "run_a"), read(&logs, "run_b")),
        (vec!["in the data folder".into()], vec!["elsewhere".into()])
    );

    // Back to the usual: the folder is read still, and as a factory
    // starts it is told so again.
    journal.write_logs_in(&logs, None, "user:ada").await.unwrap();
    line(&logs, "run_c", "back");
    assert!(dir.join("data").join("logs").join("runs").join("run_c.greet.log").is_file());
    let started = Logs::new(dir.join("data").join("logs"));
    journal.tell_logs(&started).await;
    assert_eq!(read(&started, "run_b"), ["elsewhere"]);
    assert_eq!(journal.writing().await.before, [now.folder.unwrap()]);

    // What is old goes from every folder, but for a run under way.
    let keep: HashSet<String> = ["run_b".to_string()].into();
    let later =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
    let (files, bytes) = started.let_go_before(later + 1_000, &keep);
    assert_eq!(files, 2);
    assert!(bytes > 0);
    assert!(read(&started, "run_a").is_empty() && read(&started, "run_c").is_empty());
    assert_eq!(read(&started, "run_b"), ["elsewhere"]);
}

#[tokio::test]
async fn a_folder_for_logs_is_refused_where_it_is_not_said_whole_or_cannot_be_written_in() {
    let dir = scratch("refused");
    let said = writable("logs/here").unwrap_err().message;
    assert!(said.contains("said whole"), "{said}");
    let file = dir.join("a-file");
    std::fs::write(&file, "").unwrap();
    let under: &Path = &file.join("logs");
    let said = writable(&under.to_string_lossy()).unwrap_err().message;
    assert!(said.starts_with("logs cannot be written in") && said.contains("could not be made"), "{said}");
}
