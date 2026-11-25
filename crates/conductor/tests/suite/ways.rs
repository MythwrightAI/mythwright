//! Which way the suite is being played, by the one word a test program
//! is told of its projects' journals (`PROJECT_JOURNALS`, read in one
//! place: `journal::apart`). The plays ask the first; a test of a
//! project's own journal asks the second before it holds anything.

/// Whether the suite is played with a journal for each project
/// (`PROJECT_JOURNALS=own`): what a play applies is then a project's,
/// which has a journal of its own, and is kept, heard, stopped and taken
/// up as such a project's is ([`crate::Rig::applies_for`]). Unsaid, what a play
/// applies is nobody's, and the factory's one journal keeps it. A test
/// that is not a play says which it is of for itself.
pub fn own_journals() -> bool {
    journal::apart() == Some(true)
}

/// Whether the suite is played with every project kept in the factory's
/// own journal (`PROJECT_JOURNALS=one`, `journal::apart`), as a factory
/// from before keeps them. A test of a project's own journal holds
/// nothing then, and is not played: it asks first, and says so.
pub fn one_journal() -> bool {
    let one = journal::apart() == Some(false);
    if one {
        println!("not played: it is of a project's own journal, and the suite is played with one journal");
    }
    one
}
