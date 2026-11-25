//! What is said of a project's outline (`crate`): that it was changed,
//! said in the write that keeps it, in the factory's own journal whatever
//! journal keeps the rest of what is said of the project
//! (`journal::HERE`), and heard by whoever may read projects. It carries
//! no outline: what it names may be kept from some who hear it, who read
//! it again as they may see it.

use blueprint::EventSpec;
use blueprint::ValueType::*;

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new(
            "project.arranged",
            "How a project's things are arranged in the side panel was changed: a folder made, renamed or \
             removed, something put in one or out of it, a thing shown as something else, or the arrangement \
             forgotten. Kept with the outline, in \
             the factory's own journal, and heard as the project's.",
        )
        .field("project", String, "the project")
        .field("nth", Int, "how many times its outline has been written, this time among them")
        .field("by", String, "who changed it, as an actor")
        .field(
            "forgot",
            Bool,
            "whether the arrangement was forgotten, the panel going back to how it is with none",
        ),
    ];
    blueprint::events::heard(all, &[("project.arranged", "projects:read")])
}
