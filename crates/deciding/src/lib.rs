//! What a fleet's blueprints ask so as to work nothing out themselves.
//!
//! A blueprint that ships says what exists, what is heard and what is
//! done; what is decided is asked of a block: a function of plain values
//! with a table of cases, and the action that hands it what a step was
//! handed. The blocks here are the ones a fleet's texts ask of a piece
//! of work where it is: what a turn that came for it comes to, what a
//! round that ended comes to, what an agent's turn on it came to by the
//! word it ended with, what it is owed where it is looked over, what
//! the votes on it come to, what comes next in a list of things
//! done with it; and whom something that waits for a person is told
//! to; how long a card took, read from its history (`timed`); and what
//! a contest of an item comes to, which contender is which and what the
//! judges decided (`contested`); and whether several items handed on
//! together were all passed, or one went back (`passed`); what a plan's
//! tickets come to against rows there already and the tickets a document
//! lists (`gathered`, `tickets`), and what two sides of something kept in
//! step come to (`sides`), which came out of the studio, whose tables
//! they knew nothing of. Each is
//! handed the words of a row as plain arguments and answers a word and what goes with it. And one
//! that no blueprint asks, of the
//! same kind: what one card cost, by round (`spent`), which the daemon
//! lays out for whoever reads a card.
//!
//! None of them uses an agent, which is why they are not in `agents`:
//! they came out of it when it reached the most lines a crate may have,
//! and it still offers each under the name it always gave
//! (`agents::work`, `agents::reviews`, and so on). Their actions are
//! added to a registry by whatever adds the agents' own
//! (`Agents::register`), in the order they always were.

pub mod ahead;
pub mod begun;
pub mod chain;
pub mod column;
pub mod contested;
pub mod counted;
pub mod decided;
pub mod discussed;
pub mod dispatch;
pub mod finished;
pub mod flagsdue;
pub mod gathered;
pub mod passed;
pub mod picks;
pub mod present;
pub mod pressed;
pub mod reviews;
pub mod rounds;
pub mod segments;
pub mod setups;
pub mod sides;
pub mod spent;
pub mod tally;
pub mod tickets;
pub mod timed;
pub mod told;
pub mod triggered;
pub mod turn;
pub mod unkept;
pub mod work;

use blueprint::ActionSpec;
use conductor::{ActionCx, ActionResult, Registry};
use kernel::{BoxFut, Fields};

/// Adds every block here to what a blueprint can do, in the order they
/// always were: `between` is asked where the agents' own `agent.present`
/// stands among them, by what adds the agents' actions, and does nothing
/// for a registry with no agents in it (a sandbox test has
/// these as they are: each is a function of what it is handed).
pub fn register(registry: &mut Registry, between: impl FnOnce(&mut Registry)) {
    tally::register(registry);
    counted::register(registry);
    between(registry);
    unkept::register(registry);
    rounds::register(registry);
    work::register(registry);
    reviews::register(registry);
    column::register(registry);
    segments::register(registry);
    decided::register(registry);
    told::register(registry);
    discussed::register(registry);
    turn::register(registry);
    chain::register(registry);
    timed::register(registry);
    contested::register(registry);
    passed::register(registry);
    triggered::register(registry);
    // What a plan's tickets and two sides of something come to, which
    // came out of the studio, which added them with its tables' own.
    tickets::register(registry);
    gathered::register(registry);
    sides::register(registry);
    // Which of a project's feature flags are past their day.
    flagsdue::register(registry);
    // Whether an item waits behind others worked before it and not merged.
    ahead::register(registry);
}

/// Adds one action to what a blueprint can do: handed what its step was
/// handed, and the step's context where what it answers is read as the
/// step's instance may read it.
pub(crate) fn add<F, Fut>(registry: &mut Registry, spec: ActionSpec, run: F)
where
    F: Fn(ActionCx, Fields) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ActionResult> + Send + 'static,
{
    registry.add_fn(spec, move |cx, args| -> BoxFut<'static, ActionResult> { Box::pin(run(cx, args)) });
}
