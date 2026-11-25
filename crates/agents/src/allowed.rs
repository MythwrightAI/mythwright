//! What an agent is told of what its slot lets it do, as part of its
//! brief: what it may ask of the factory, by area, as the Tools panel
//! says it; what its program may do on the device (its wall, how it is
//! held, the web); what it signs in
//! with, by kind and never by value; and who changes any of it. The
//! brief is made again for every turn and written only where it
//! differs, so a change of any of these is in the next turn's.

use kernel::principal::areas;

use crate::signins::{KEY, SUBSCRIPTION};
use crate::{PERMISSIONS, Slot};

/// The part of `slot`'s brief that says what it may do and who says so.
/// `walled`: whether its program works behind a wall this turn;
/// `grouped`, what the groups it is in hold (`people::groups`), which it
/// holds beside its slot's.
pub(crate) fn told(slot: &Slot, walled: bool, grouped: &[String]) -> String {
    let (mut held, mut not) = (Vec::new(), Vec::new());
    let mut scopes = slot.scopes.clone();
    scopes.extend(grouped.iter().filter(|scope| !kernel::principal::covers(&slot.scopes, scope)).cloned());
    for area in areas() {
        match area.held(&scopes) {
            Some(words) => held.push(format!("  {}: {words}", area.area)),
            None => not.push(format!("`{}`", area.area)),
        }
    }
    let nothing_of = match not.as_slice() {
        [] => String::new(),
        not => format!("\nAnd nothing of: {}.", not.join(", ")),
    };
    // How many, and not which: a place that is not there is shown
    // nowhere, and what the wall shows is the wall's to say.
    let device = match (walled, slot.reads.len(), slot.changes.len()) {
        (false, ..) => {
            "Your program works with no wall: it sees this device as whoever runs the daemon does.".into()
        }
        (true, 0, 0) => "Your program works behind a wall.".to_string(),
        (true, reads, changes) => format!(
            "Your program works behind a wall. Besides your work and your home, your slot shows it {reads} more \
             place(s) of this device to read and {changes} to change, where they are there."
        ),
    };
    let mode = PERMISSIONS.iter().find(|(name, _)| *name == slot.permission_mode);
    let mode = mode.map(|(name, means)| format!(" It is held to `{name}`: {means}.")).unwrap_or_default();
    let web = match slot.web {
        true => " It may use the web unasked.",
        false => " It asks leave before it uses the web, and in a piece of work nobody is there to give it.",
    };
    let signin = match slot.signins.as_slice() {
        [] => "It signs in to its model's provider as whoever runs the daemon.".to_string(),
        draws => {
            let said: Vec<String> = draws
                .iter()
                .map(|draw| match draw.as_str() {
                    SUBSCRIPTION => "any subscription the factory holds for it".to_string(),
                    KEY => "any key the factory holds for it".to_string(),
                    name => format!("the sign-in `{name}`"),
                })
                .collect();
            format!(
                "It signs in with what the factory holds, drawn before each piece of work from, in this order: {}.",
                said.join("; ")
            )
        }
    };
    let groups = match grouped {
        [] => String::new(),
        grouped => format!(
            "\nOf that you hold {} by the groups you are in, which whoever manages the factory changes; you never do.",
            grouped.iter().map(|scope| format!("`{scope}`")).collect::<Vec<_>>().join(", ")
        ),
    };
    format!(
        "\n\nWhat you may do is your slot's, as it stands now.\nOf the factory you may ask, by area:\n{held}{nothing_of}{groups}\n\
         {device}{mode}{web} {signin}\n\
         Your slot is set by a person, or by an agent that holds what it gives: changing an agent takes \
         `agents:write`, taking its wall away, showing it more of this device, the web and going on without the \
         daemon take `shell`, and a sign-in or a key takes `secrets:use`. Nobody gives what they do not hold, and \
         you cannot change your own slot, even within what you hold: to be let do more, ask the people you work \
         with.\n\
         Where your work needs a change of a record that you may not make (a cell of a table that takes more than \
         you hold), ask for it rather than for more yourself: `{cli} ask <record> <field>=<value>... --why \
         <words>`. It is made, as yours, once a person who could make it approves it, and you are told where you \
         asked. `{cli} asked` says how each stands. Your own slot is not asked for so.",
        held = held.join("\n"),
        cli = brand::CLI,
    )
}

/// What an agent's program sees of this device, as it is held while the
/// daemon runs, in the words every place that says it uses (`mw agent
/// list` and `add`, the card about an agent): what it sees, and how.
/// A slot says whether it asks for a wall; a factory started holding none
/// (`--walls off`) gives it none whatever it says, and is said so, since
/// its program then sees the device as whoever runs the daemon does.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Seen {
    /// Its work, or the whole device.
    pub what: String,
    /// How that is: behind a wall (and shown how many more places), or no
    /// wall and why.
    pub how: String,
    /// Whether its program is behind a wall now, for what is said only of
    /// one that is (what it may do with the repository).
    pub walled: bool,
}

/// What a slot that asks for a wall (`wall`) and is shown `more` places
/// besides its work sees, where the factory holds walls (`held`) or not.
pub fn seen(wall: bool, held: bool, more: usize) -> Seen {
    let device = |how: &str| Seen { what: "this device".into(), how: how.into(), walled: false };
    match (wall, held, more) {
        (false, ..) => device("no wall, as whoever runs the daemon"),
        (true, false, _) => device("no wall, since this factory holds none"),
        (true, true, 0) => Seen { what: "its work".into(), how: "behind a wall".into(), walled: true },
        (true, true, more) => Seen {
            what: "its work".into(),
            how: format!("behind a wall, and shown {more} more"),
            walled: true,
        },
    }
}

impl crate::Agents {
    /// What `slot`'s program sees of this device as the daemon holds walls now ([`seen`]).
    pub fn sees(&self, slot: &Slot) -> Seen {
        let held = self.inner.hosting.walls() != hosting::Walls::Open;
        seen(slot.wall, held, slot.reads.len() + slot.changes.len())
    }
}
