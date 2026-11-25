//! Whether what a database writes is waited for on the disk.
//!
//! A database in a file waits for the disk at a few moments: as the file
//! is made, each time its log is brought back into it, as it is closed,
//! and for a commit that is to be on the disk before it is answered
//! ([`crate::Db::call_to_the_disk`]). That is what keeps a loss of power,
//! or of the system, from taking more than is said
//! ([`crate::checkpoint`]), and a factory that is run always waits.
//!
//! A test cannot hold any of it: none takes the power away, and what a
//! program that is killed had written is the system's to keep whether or
//! not the disk was waited for. And the waits are most of what a test
//! that opens many databases takes: a small play of the runner's, 73
//! conductors stopped and started on files, took 12 to 15 s waiting and
//! 1.7 s not, and three quarters of the samples of its threads were in
//! the system's call that waits for the disk (`docs/build-times.md`). So
//! a factory made for a test says it does not wait:
//! [`crate::Db::open_unwaited`] where the test opens the file itself, a
//! node's configuration where a node does, and for a daemon that a test
//! starts as a program of its own, one word in its environment
//! ([`SAYS`]), read here and nowhere else. A daemon that is run is never
//! told.
//!
//! What does not wait is what goes through the journal's own thread and
//! the thread that brings its log back. What is written to be on the
//! disk by a way of its own waits whatever was said here: a file written
//! out again (`rewrite`), a project's file moved apart (`moved`), a copy
//! of one moment (`moment`). And a test of the disk itself says nothing,
//! and so waits: the tests of this crate's own suite open their files
//! with [`crate::Db::open`].

use std::sync::OnceLock;

/// Whether a database waits for the disk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Disk {
    /// As a factory that is run does.
    #[default]
    Waited,
    /// Never: what is written is handed to the system and no more is
    /// asked. For a factory made for a test.
    Unwaited,
}

/// What says, to a daemon started by a test, that it waits for the disk
/// nowhere: `unwaited`. Unsaid, or saying anything else, it waits as
/// every daemon does.
const SAYS: &str = "JOURNAL_DISK";

/// What this program was told when it was started.
pub(crate) fn said() -> Disk {
    static SAID: OnceLock<Disk> = OnceLock::new();
    *SAID.get_or_init(|| match std::env::var(SAYS).ok().as_deref() {
        Some("unwaited") => Disk::Unwaited,
        _ => Disk::Waited,
    })
}

impl Disk {
    /// What the database is told of it, when it is opened and for as
    /// long as nothing is to be on the disk before it is answered.
    pub(crate) fn synchronous(self) -> &'static str {
        match self {
            Disk::Waited => "NORMAL",
            Disk::Unwaited => "OFF",
        }
    }
}
