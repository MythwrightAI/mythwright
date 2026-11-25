//! How the tests are shared out between processes and processors.
//!
//! A program that starts a daemon for each test gets no faster past a
//! dozen threads: its tests wait on one another inside the one process.
//! So where there are processors for it, test programs are run side by
//! side on eight threads each, and one with many tests is run as several
//! processes that each take a share of them. This is the arithmetic of
//! that, apart from the running (`xtask/src/suites.rs`), so that it can be
//! asked directly: what must hold is that every test is in exactly one
//! share, whatever the numbers.

/// How many threads one process of tests is given. Four processes of
/// eight and eight of four finish within a second of each other; one of
/// thirty-two takes more than twice as long.
const WIDTH: usize = 8;

/// How many tests a process is worth starting for. A program with no more
/// than this is run whole.
const SHARE: usize = 64;

/// What there is room for at once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    /// How many shares a program's tests may be put in.
    pub shares: usize,
    /// How many processes run at a time: a quarter more than the
    /// processors would give each its eight threads. Tests wait on files
    /// and on the programs they start as well as on a processor, and a
    /// process left over is not then kept waiting for a whole round (five
    /// of eight threads on thirty-two processors took 11 s where four at
    /// a time took 15).
    pub at_once: usize,
    /// How many tests each process runs at a time.
    pub threads: usize,
}

/// The room there is on `processors`. On fewer than sixteen it is one
/// process at a time with a thread for each: as cargo runs the tests.
pub fn room(processors: usize) -> Room {
    let processors = processors.max(1);
    let shares = (processors / WIDTH).max(1);
    Room { shares, at_once: shares + shares / 4, threads: processors / shares }
}

/// `tests` put in shares, for a program that has that many: every
/// so-manyth test by where it is in the list, so that the tests of one
/// module, which are together in it and are alike in what they cost, are
/// spread between the shares. `None` where the program is to be run whole.
pub fn shares(tests: &[String], room: Room) -> Option<Vec<Vec<String>>> {
    let count = room.shares.min(tests.len().div_ceil(SHARE));
    (count > 1)
        .then(|| (0..count).map(|share| tests.iter().skip(share).step_by(count).cloned().collect()).collect())
}
