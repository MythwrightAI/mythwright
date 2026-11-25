//! The events said here: the client replaced, and the names the daemon
//! answers to.

use blueprint::events::ANYONE;
use blueprint::{EventSpec, ValueType::*};

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new("system.named", "The daemon was told a name it is asked under, and answers to it.")
            .field("name", String, "the name"),
        EventSpec::new("system.unnamed", "A name the daemon answered to was taken back.")
            .field("name", String, "the name"),
        EventSpec::new("client.updated", "The client's files were replaced by another build.")
            .field("build", String, "the build there is now")
            .field("was", String, "the build there was"),
    ];
    // That the client was replaced is for anyone who is signed in: every
    // page goes by it.
    blueprint::events::heard(
        all,
        &[("client", ANYONE), ("system.named", "fleet:read"), ("system.unnamed", "fleet:read")],
    )
}
