//! What of the agents' does not go into a copy of a factory.
//!
//! A copy is a factory's database at one moment, for another factory to be
//! started from (`journal::copy`). The agents go with it as they were set
//! up: each slot's name, project, instructions, what it may do and what it
//! runs on. So does what each has done, turn by turn. What does not:
//!
//! - **Work in hand.** A turn that was queued or running is the other
//!   factory's, and its agent there is doing it. In a copy it is ended as
//!   failed, in so many words, so that no agent of the new factory takes
//!   up work it was never given.
//! - **Credentials.** The one each agent had was for the other factory,
//!   and none is copied (`studio::copy`); the file it was kept in is in
//!   the folder of slots, which is not copied either. An agent in the new
//!   factory is given one of that factory's when it is next about to work
//!   (`Agents::credential`), and nothing fails for the want of one before.
//!   An agent the factory does not start (`crate::external`) has its
//!   token wherever its program runs, and that one is the other
//!   factory's: here its slot is there, with what it holds, and it has
//!   no credential and no invitation (those are under a name of the
//!   small durable state that no copy keeps, `studio::enrolled`) until
//!   somebody invites it to this factory.
//! - **The sign-ins the factory holds** (`signins`), every row of them:
//!   which there are, who held which and what was set aside. Their values
//!   were never in the database; they are files in a folder of the data
//!   folder that is not copied either. A sign-in handed to two factories
//!   would be held by more programs than it says, and a folder's would be
//!   renewed by each under the other. What each agent draws its sign-in
//!   from goes with the agent, as its other settings do: in the new
//!   factory its turns fail, saying that the factory holds none of those,
//!   until some are added there. It does not fall back to signing in as
//!   whoever runs the daemon, which nobody asked of it.
//! - **Where the listener for messages had read to** (`agents`). A new
//!   factory starts listening from the end of its journal, so a message
//!   that was said before the copy is never answered a second time. (A
//!   turn is also kept for each message answered, and no message gets two.)
//! - **The conversations agents' programs remember** (`agent.session`):
//!   each is an id in that program's own records, on the other factory's
//!   device and under its user. Carrying one on from a copy would be two
//!   factories writing one conversation.
//! - **What each program said it offers and last ran on** (`agent.models`,
//!   `agent.model`): said of the account it was signed in with there. Each
//!   is asked again when it is next wanted.
//!
//! The agents stay as they were, taking work or stopped: one that was
//! taking work answers what is said to it in the new factory from then on.

use journal::copy::{Connection, forget};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// The tables this crate keeps, by the name they are migrated under, and
/// the version of each this build knows.
pub const KNOWS: &[(&str, usize)] = &[("agents", crate::schema::SCHEMA.len())];

/// The tables this crate keeps, each spoken for above: one that no crate
/// lists is emptied in a copy.
pub const TABLES: &[&str] = &["slots", "turns", "signins"];

/// What is said of a turn that was waiting or under way where the copy was
/// taken.
pub const NOT_TAKEN_UP: &str = "it was waiting or under way in the factory this one was copied from, and a copy takes up none of that factory's work";

/// What a copy holds of agents, and what it does not.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stood {
    /// Turns that were queued or running.
    pub turns: usize,
    /// Every agent of the copy, by name: each is there, without a credential.
    pub agents: Vec<String>,
    /// Sign-ins the factory held, none of which is in the copy.
    #[serde(default)]
    pub signins: usize,
}

/// Ends the agents' work in hand in a copy, as of `at` (milliseconds), and
/// forgets what was remembered of them that is not true of a new factory.
/// Done again it changes nothing, and names the same agents.
pub fn leave_out(connection: &Connection, at: i64) -> rusqlite::Result<Stood> {
    let turns = end_turns(connection, at)?;
    // One that was leaving had only that work to finish: it is gone.
    connection.execute("UPDATE slots SET active = 0, leaving = 0 WHERE leaving = 1", [])?;
    let signins = connection.execute("DELETE FROM signins", [])?;
    for namespace in ["agents", crate::removed::SESSIONS, crate::MODELS_KEPT, crate::MODEL_REPORTED] {
        forget(connection, namespace)?;
    }
    let mut statement = connection.prepare("SELECT name FROM slots ORDER BY created, name")?;
    let agents = statement.query_map([], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    Ok(Stood { turns, agents, signins })
}

/// Ends the turns that were waiting or under way in a copy, as of `at`:
/// how many that was.
fn end_turns(connection: &Connection, at: i64) -> rusqlite::Result<usize> {
    let turns = connection.execute(
        "UPDATE turns SET status = 'failed', error = ?1, finished = coalesce(finished, ?2)
         WHERE status IN ('queued', 'running')",
        params![NOT_TAKEN_UP, at],
    )?;
    // One whose answer was kept and not yet said to have ended did end,
    // in the other factory: it is as it came out, and nothing is said
    // of it here.
    connection.execute(
        "UPDATE turns SET status = CASE WHEN error IS NULL THEN 'done' ELSE 'failed' END
         WHERE status = 'answered'",
        [],
    )?;
    Ok(turns)
}

/// Ends the agents' work in hand in the copy of a project's own journal,
/// as [`leave_out`] does in the factory's: the turns that journal keeps
/// ([`crate::apart::KEPT_APART`]). How many were waiting or under way;
/// none, for a journal that keeps no turns. The list of agents and the
/// sign-ins are made in such a file and hold nothing, and whatever is
/// found in them does not travel.
pub fn leave_out_beside(connection: &Connection, at: i64) -> rusqlite::Result<usize> {
    let made: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'turns')",
        [],
        |row| row.get(0),
    )?;
    if !made {
        return Ok(0);
    }
    connection.execute("DELETE FROM slots", [])?;
    connection.execute("DELETE FROM signins", [])?;
    end_turns(connection, at)
}
