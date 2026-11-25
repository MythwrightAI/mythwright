//! How many places each scenario passes and how many plays are made of
//! it, written down and held to.
//!
//! Which plays are made of a scenario follows from what its conductor
//! keeps, and in what order: the order the first play passes its places
//! in is the order the others are made in, and a play finds where it is
//! stopped a second time among what its conductor keeps after a start.
//! So a count that is another from one run to the next says that an
//! order of two things goes by something nobody wrote: by load, most
//! likely, which is how the counts of nine tests came to move while each
//! held (`crate::stepped`, `crate::anywhere::abreast`). Every order has
//! to hold, and the test still says whether this one did; but an order
//! that only a busy machine comes to is one that is played once in so
//! many runs, and a fault in it shows once in so many gates.
//!
//! So what each scenario came to is kept (`tests/fixtures/played.txt`: the
//! test, which of the things it played, the scenario, its places and
//! its plays), and a scenario that comes to something else fails its
//! test, saying both. Where the plays themselves were changed, or the
//! runner now keeps other things, `WRITE_PLAYED=1 cargo test -p conductor`
//! writes what they come to now, and the file is committed with the
//! change; a line of a test that is gone is taken out by hand.
//!
//! Held where the suite is played as the gates play it. Told another
//! way (a journal for each project, how the journal gathers, every
//! second stop) it plays other things, and holds none of this.

use std::cell::Cell;
use std::sync::Mutex;

/// Where what each scenario came to is kept, in the crate's folder.
const KEPT: &str = "tests/fixtures/played.txt";

/// One at a time, of all the tests there are, where the file is written.
static WRITING: Mutex<()> = Mutex::new(());

thread_local! {
    /// How many things the test of this thread has played so far.
    static PLAYED: Cell<usize> = const { Cell::new(0) };
}

/// Holds what a scenario came to, asked from the thread of its test, to
/// what is kept of it; or writes that down, where whoever runs the suite
/// said to.
pub fn held(scenario: &str, places: usize, plays: usize) {
    let another_way = journal::apart().is_some()
        || ["JOURNAL_GATHERS", "ANYWHERE"].iter().any(|said| std::env::var(said).is_ok());
    // A test is known by its thread, which is called what it is.
    let test = std::thread::current().name().unwrap_or_default().to_string();
    if another_way || test.is_empty() || test == "main" {
        return;
    }
    let nth = PLAYED.with(|played| {
        played.set(played.get() + 1);
        played.get()
    });
    let key = format!("{test}\t{nth}\t{scenario}\t");
    let now = format!("{key}{places}\t{plays}");
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(KEPT);
    let _one = WRITING.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let kept = std::fs::read_to_string(&file).unwrap_or_default();
    let was = kept.lines().find(|line| line.starts_with(&key));
    if was == Some(now.as_str()) {
        return;
    }
    if std::env::var("WRITE_PLAYED").is_ok() {
        let mut lines: Vec<&str> = kept.lines().filter(|line| !line.starts_with(&key)).collect();
        lines.push(&now);
        lines.sort_unstable();
        std::fs::write(&file, lines.join("\n") + "\n").expect("what was played can be written down");
        return;
    }
    let came = |line: &str| {
        let mut fields = line.rsplit('\t');
        let (plays, places) = (fields.next().unwrap_or_default(), fields.next().unwrap_or_default());
        format!("{places} places and {plays} plays")
    };
    panic!(
        "{scenario}: it came to {}, and {} is what is kept of it ({KEPT}). Which plays are made follows from \
         the order its conductor keeps things in, so either what is played was changed, or the runner now \
         keeps other things (then `WRITE_PLAYED=1 cargo test -p conductor` writes what every scenario \
         comes to now, to be committed with the change); or an order of two things went by something \
         nobody wrote, which is a fault to be found: run the test again, alone and beside the others, and \
         see whether it comes to the same",
        came(&now),
        was.map_or("nothing".to_string(), came),
    );
}

/// Writes down how many places the test this is asked from passed and
/// how many plays it made of them, where whatever runs the suite to time
/// it asked to be told (`PLAYED_TO`, a file: the gates and `cargo xtask
/// build-times`, which keep it beside what each test took). Told nothing,
/// as a program run by anybody else is, it writes nothing. The test is
/// known by its thread, which is called what it is.
pub fn played(places: usize, plays: usize) {
    let Ok(to) = std::env::var("PLAYED_TO") else { return };
    let test = std::thread::current().name().unwrap_or_default().to_string();
    let line = format!("{}\t{places}\t{plays}\t{test}\n", env!("CARGO_PKG_NAME"));
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(to) {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}
