//! What is said of the setups a factory keeps (`crate::setups`): that one
//! was saved or taken away, said in the write that keeps it, in the
//! factory's own journal for good (`journal::HERE`), and heard by
//! whoever may read projects. It carries no rows: whoever is to show them
//! reads them again.

use blueprint::EventSpec;
use blueprint::ValueType::*;

pub fn events() -> Vec<EventSpec> {
    let all = vec![
        EventSpec::new(
            "project.setup.saved",
            "A setup new projects may be set up with was saved: rows of blueprints, under a name, or as the factory's default.",
        )
        .field("name", String, "what it is called; `default` for the factory's default for new projects")
        .field("rows", Int, "how many rows it has")
        .field("by", String, "who saved it, as an actor"),
        EventSpec::new(
            "project.setup.removed",
            "A setup new projects could be set up with was taken away; the factory's default taken away is what ships again.",
        )
        .field("name", String, "what it was called")
        .field("by", String, "who took it away, as an actor"),
    ];
    blueprint::events::heard(
        all,
        &[("project.setup.saved", "projects:read"), ("project.setup.removed", "projects:read")],
    )
}
