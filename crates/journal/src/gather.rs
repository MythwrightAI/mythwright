//! Several pieces of work kept by one commit.
//!
//! Every piece of work the database's thread does was a commit of its
//! own, and a commit is most of what a write costs: the pages it changed
//! written out whole, though the next piece of work changes the same
//! ones. So where the thread has fallen behind ([`DEEP`] pieces of work
//! or more are waiting), it does what is waiting in the same transaction
//! and commits once ([`MOST`] pieces at most, and for no longer than
//! [`FOR`]). Nothing waits for company: with nothing else asked, a piece
//! of work is committed by itself as it always was.
//!
//! It is done only where the thread is behind because it is a trade, and
//! is worth making only there. Whoever is in a commit with others is
//! answered when all of them have been done: later than they would have
//! been by themselves, where what the others do is more than the commit
//! they are spared (reading is half of what is asked, and costs no
//! commit). So a thread that is keeping up answers each as it comes;
//! one that is not gets through what is waiting two to three times as
//! fast, and everybody waiting behind is the better for it. The figures,
//! and what sharing among whatever waits came to in a factory at work
//! (nothing that could be measured, either way), are in
//! `docs/architecture.md`, *The journal*.
//!
//! What whoever asks is promised is what it was:
//!
//! - **Each piece of work is whole or not at all.** One that opens a
//!   whole of its own ([`whole`]) and fails takes back what it wrote and
//!   nothing of anybody else's.
//! - **Nobody is answered before what they wrote is kept.** The answers
//!   are handed over after the commit, in the order the work was done; and
//!   where the commit fails, every piece of work in it is told so and
//!   nothing of any of them is kept.
//! - **What follows from something being kept follows the commit**
//!   ([`then`]): an event is the journal's last, and is heard by whoever
//!   follows, only once it is on the disk with everything written beside
//!   it.
//! - **A place the journal was told is where it was.** The piece of work
//!   that reaches it is the last in its commit ([`cut`]): what was asked
//!   after it is not done with it.
//!
//! A piece of work that holds the thread until it is let go is done
//! apart (`Db::call_apart`): what was done before it is committed and
//! answered first, and it has a commit of its own.

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
use std::time::Duration;

use kernel::Error;
use rusqlite::{Connection, Savepoint};

/// How many pieces of work one commit keeps at most. More than this and
/// the first of them waits for its answer longer than it gains by; the
/// pages a commit writes are mostly shared by the first few.
pub(crate) const MOST: usize = 64;

/// How long a commit goes on taking work before it is made, at most:
/// what the first piece of work in it may wait for the rest, where the
/// rest is slow.
pub(crate) const FOR: Duration = Duration::from_millis(2);

/// How many pieces of work are waiting when the thread is taken to have
/// fallen behind, and keeps what is waiting by one commit. Fewer, and
/// each is committed and answered as it comes.
pub(crate) const DEEP: usize = 16;

/// What says, to a program run for it, how work is gathered: `1` has
/// every piece of work committed by itself, and `all` has whatever is
/// waiting kept by one commit however little that is. A suite that is to
/// be played each way says one or the other. Unsaid, it is what a daemon
/// does: [`MOST`] at most, where [`DEEP`] or more are waiting.
const SAYS: &str = "JOURNAL_GATHERS";

/// How work is gathered, as this program was told when it was started:
/// how many pieces a commit keeps at most, and how many have to be
/// waiting for a commit to take in more than one.
pub(crate) fn said() -> (usize, usize) {
    static SAID: OnceLock<(usize, usize)> = OnceLock::new();
    *SAID.get_or_init(|| match std::env::var(SAYS).ok().as_deref() {
        Some("1") => (1, usize::MAX),
        Some("all") => (MOST, 1),
        _ => (MOST, DEEP),
    })
}

/// Whether this program was told to keep whatever is waiting by one
/// commit however little that is (`all`): then how many commits there
/// are is how the machine's work fell, and says nothing of how much was
/// asked. What a test outside the journal goes by of this is asked here,
/// so that the word is read in one place.
pub fn gathering_forced() -> bool {
    said().1 == 1
}

/// What is done once a commit is made, or once it has failed: told which.
type Then = Box<dyn FnOnce(bool)>;

#[derive(Default)]
struct Open {
    then: Vec<Then>,
    cut: bool,
}

thread_local! {
    /// The commit this thread has open, if it is a database's thread
    /// and is in the middle of one.
    static OPEN: RefCell<Open> = RefCell::default();
}

/// Has `effect` done once what this piece of work wrote is kept, told
/// `true`; or told `false`, where the commit it was to be kept by
/// failed and nothing of it is. In the order they are asked for, before
/// anybody is answered. Asked from a piece of the database's work.
pub(crate) fn then(effect: impl FnOnce(bool) + 'static) {
    OPEN.with_borrow_mut(|open| open.then.push(Box::new(effect)));
}

/// Has the commit under way end with this piece of work: what was asked
/// after it is done in the next, once what follows this one has been done.
pub(crate) fn cut() {
    OPEN.with_borrow_mut(|open| open.cut = true);
}

/// Whether the commit under way is to end with the piece of work just done.
pub(crate) fn is_cut() -> bool {
    OPEN.with_borrow(|open| open.cut)
}

/// Does what was to follow the commit, saying whether it was made, and
/// leaves nothing for the next.
pub(crate) fn follow(kept: bool) {
    let open = OPEN.with_borrow_mut(std::mem::take);
    for effect in open.then {
        effect(kept);
    }
}

thread_local! {
    /// Whether this thread is a database's own, which has a transaction
    /// open for whatever it is asked.
    static HERE: Cell<bool> = const { Cell::new(false) };
}

/// Says that this thread is a database's own.
pub(crate) fn here() {
    HERE.set(true);
}

/// Whether this thread is a database's own.
pub(crate) fn is_here() -> bool {
    HERE.get()
}

/// What a piece of the thread's work is told where the database refused
/// it for the transaction the thread has open: in a sentence that says
/// what to write, where the database's own words say only that there is
/// one. Whoever wrote the work did not begin that transaction and may
/// not know of it. Nothing, of any other failure.
pub(crate) fn refused(err: &rusqlite::Error) -> Option<Error> {
    let rusqlite::Error::SqliteFailure(_, Some(said)) = err else {
        return None;
    };
    if said.contains("cannot start a transaction within a transaction") {
        return Some(Error::internal(
            "a piece of work for the journal's database began a transaction of its own, and the database's \
             thread has one open for it already: write what is to be kept or taken back together in a whole, \
             `journal::whole(connection)`, in place of `connection.transaction()`",
        ));
    }
    // Emptying the file's free pages, how the file is held, how its log is kept.
    let outside = ["cannot VACUUM from within a transaction", "may not be changed inside a transaction"]
        .iter()
        .any(|words| said.contains(words))
        || said.contains("from within a transaction");
    outside.then(|| {
        Error::internal(format!(
            "a piece of work for the journal's database asked what cannot be done inside a transaction, and \
             the database's thread has one open for all it does ({said}): do it on a connection of its own, \
             as a copy is made (`journal::copy`)"
        ))
    })
}

/// A whole of a piece of work's own: what is written in it is kept
/// together when it is committed, and taken back together where it is
/// put away without. This is how a piece of the database's work says
/// that several things it writes are one, and the only way: the thread
/// has a transaction open already, which a second cannot be begun in.
pub fn whole(connection: &mut Connection) -> rusqlite::Result<Savepoint<'_>> {
    connection.savepoint()
}
