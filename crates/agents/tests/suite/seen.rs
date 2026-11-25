//! What an agent's program is said to see of the device, as walls are
//! held while the daemon runs (`agents::allowed::seen`): the one place the
//! command line and the card about an agent take their words from.

use agents::allowed::{Seen, seen};

fn said(what: &str, how: &str, walled: bool) -> Seen {
    Seen { what: what.into(), how: how.into(), walled }
}

#[test]
fn what_an_agent_sees_is_what_is_held_as_the_daemon_runs_and_not_only_what_its_slot_asks_for() {
    let cases = [
        // (asks for a wall, the factory holds walls, places shown besides its work)
        ((true, true, 0), said("its work", "behind a wall", true)),
        ((true, true, 2), said("its work", "behind a wall, and shown 2 more", true)),
        // A factory started holding none (`--walls off`) gives it none.
        ((true, false, 0), said("this device", "no wall, since this factory holds none", false)),
        ((true, false, 3), said("this device", "no wall, since this factory holds none", false)),
        // One that asks for none has none, whatever the factory holds.
        ((false, true, 0), said("this device", "no wall, as whoever runs the daemon", false)),
        ((false, false, 1), said("this device", "no wall, as whoever runs the daemon", false)),
    ];
    for ((wall, held, more), expected) in cases {
        assert_eq!(seen(wall, held, more), expected, "wall {wall}, held {held}, {more} more");
    }
}
